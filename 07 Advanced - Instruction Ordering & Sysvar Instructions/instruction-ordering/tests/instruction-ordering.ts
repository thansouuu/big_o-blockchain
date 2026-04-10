import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { expect } from "chai";
import { 
  PublicKey, 
  Transaction, 
  Keypair,
  LAMPORTS_PER_SOL 
} from "@solana/web3.js";
import { Exercise } from "../target/types/exercise";

describe("exercise", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Exercise as Program<Exercise>;

  // ---------------- Part 1: Instruction Ordering ----------------

  it("fails to execute without approval", async () => {
    try {
      await program.methods
        .execute(new anchor.BN(1000))
        .accounts({ authority: provider.wallet.publicKey }) // ixSysvar tự động điền
        .rpc();
      expect.fail("Should have failed");
    } catch (err: any) {
      expect(err.message).to.include("MissingPreviousInstruction");
    }
  });

  it("succeeds with approval in same transaction", async () => {
    const approveIx = await program.methods
      .approve()
      .accounts({ authority: provider.wallet.publicKey })
      .instruction();

    const executeIx = await program.methods
      .execute(new anchor.BN(1000))
      .accounts({ authority: provider.wallet.publicKey })
      .instruction();

    const tx = new Transaction().add(approveIx).add(executeIx);
    await provider.sendAndConfirm(tx);
  });

  it("fails with wrong order (execute before approve)", async () => {
    const approveIx = await program.methods
      .approve()
      .accounts({ authority: provider.wallet.publicKey })
      .instruction();

    const executeIx = await program.methods
      .execute(new anchor.BN(1000))
      .accounts({ authority: provider.wallet.publicKey })
      .instruction();

    // Thêm sai thứ tự: execute trước, approve sau
    const tx = new Transaction().add(executeIx).add(approveIx);

    try {
      await provider.sendAndConfirm(tx);
      expect.fail("Should have failed");
    } catch (err: any) {
      // Vì execute chạy đầu tiên, nó sẽ thấy index = 0 và báo lỗi MissingPreviousInstruction
      expect(err.message).to.include("MissingPreviousInstruction");
    }
  });

  // ---------------- Part 2: Regular Account<T> vs Zero-Copy ----------------

  it("initializes and uses regular large approval data", async () => {
    await program.methods
      .initializeLargeApprovalRegular()
      .accounts({ authority: provider.wallet.publicKey }) // PDA và SystemProgram tự điền
      .rpc();

    await program.methods
      .processLargeApprovalRegular()
      .accounts({ authority: provider.wallet.publicKey })
      .rpc();

    const [regularPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("approval_regular"), provider.wallet.publicKey.toBuffer()],
      program.programId
    );
    const info = await provider.connection.getAccountInfo(regularPda);
    expect(info!.data.length).to.equal(8 + 32 + (128 * 8));
  });

  it("initializes and uses zero-copy large approval data", async () => {
    await program.methods
      .initializeLargeApprovalZeroCopy()
      .accounts({ authority: provider.wallet.publicKey })
      .rpc();

    await program.methods
      .processLargeApprovalZeroCopy()
      .accounts({ authority: provider.wallet.publicKey })
      .rpc();

    const [zcPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("approval_zero_copy"), provider.wallet.publicKey.toBuffer()],
      program.programId
    );
    const info = await provider.connection.getAccountInfo(zcPda);
    expect(info!.data.length).to.be.greaterThan(4000);
  });

  // ---------------- Part 3: Multi-Send (Remaining Accounts) ----------------

  it("successfully performs multi_send to 3 recipients", async () => {
    const r1 = Keypair.generate().publicKey;
    const r2 = Keypair.generate().publicKey;
    const r3 = Keypair.generate().publicKey;

    const amount = new anchor.BN(0.01 * LAMPORTS_PER_SOL);

    await program.methods
      .multiSend(amount)
      .accounts({ sender: provider.wallet.publicKey }) // SystemProgram tự điền
      .remainingAccounts([
        { pubkey: r1, isWritable: true, isSigner: false },
        { pubkey: r2, isWritable: true, isSigner: false },
        { pubkey: r3, isWritable: true, isSigner: false },
      ])
      .rpc();

    const balance = await provider.connection.getBalance(r3);
    expect(balance).to.equal(amount.toNumber());
  });

  it("fails multi_send if no recipients provided", async () => {
    try {
      await program.methods
        .multiSend(new anchor.BN(1000))
        .accounts({ sender: provider.wallet.publicKey })
        .remainingAccounts([]) // Danh sách trống
        .rpc();
      expect.fail("Should have failed");
    } catch (err: any) {
      expect(err.message).to.include("NoRecipients");
    }
  });
  it("fails multi_send if a recipient is not writable", async () => {
    const recipient = Keypair.generate().publicKey;
    const amount = new anchor.BN(1000);
    try {
      await program.methods
        .multiSend(amount)
        .accounts({ sender: provider.wallet.publicKey })
        .remainingAccounts([
          { pubkey: recipient, isWritable: false, isSigner: false },
        ])
        .rpc();
      
      expect.fail("Should have failed because recipient is not writable");
    } catch (err: any) {
      expect(err.message).to.include("NotWritable");
    }
  });

  it("fails multi_send if there are too many recipients (> 10)", async () => {
    const amount = new anchor.BN(1000);
    const tooManyRecipients = Array.from({ length: 11 }, () => ({
      pubkey: Keypair.generate().publicKey,
      isWritable: true,
      isSigner: false,
    }));

    try {
      await program.methods
        .multiSend(amount)
        .accounts({ sender: provider.wallet.publicKey })
        .remainingAccounts(tooManyRecipients)
        .rpc();
      
      expect.fail("Should have failed due to too many recipients");
    } catch (err: any) {
      expect(err.message).to.include("TooManyRecipients");
    }
  });
});