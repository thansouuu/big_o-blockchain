import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { BankApp } from "../target/types/bank_app";
import { StakingApp } from "../target/types/staking_app";
import { expect } from "chai";
import { 
  PublicKey, 
  SystemProgram, 
  TransactionInstruction, 
  TransactionMessage, 
  VersionedTransaction 
} from "@solana/web3.js";
import { BN } from "bn.js";
import { 
  TOKEN_PROGRAM_ID, 
  ASSOCIATED_TOKEN_PROGRAM_ID, 
  getAssociatedTokenAddressSync, 
  createAssociatedTokenAccountInstruction 
} from "@solana/spl-token";

import { setupLookupTable } from "./helpers/lookup_table";

describe("Bank App - Versioned Transactions", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.BankApp as Program<BankApp>;
  const stakingProgram = anchor.workspace.StakingApp as Program<StakingApp>;

  const STAKING_PROGRAM_ID = stakingProgram.programId;
  const BANK_PROGRAM_ID = program.programId;

  // Địa chỉ token bạn đã mint từ CLI
  const mockTokenMint = new PublicKey('3cYRG1gzC93CiPQ2ZB1FyEpy11NoX8N9AGxqqQVAg4Fj');
  const mockTokenMint2 = new PublicKey('9db4VTqEdFG8gAvXB39XwRpukkKez2yASvf65vtXxSH6'); 

  const bankVaultPda = PublicKey.findProgramAddressSync([Buffer.from("BANK_VAULT_SEED")], BANK_PROGRAM_ID)[0];

  const BANK_PDAS = {
    bankInfo: PublicKey.findProgramAddressSync([Buffer.from("BANK_INFO_SEED")], BANK_PROGRAM_ID)[0],
    bankVault: bankVaultPda,
    tokenReserve: (tokenMint: PublicKey) => PublicKey.findProgramAddressSync([Buffer.from("BANK_TOKEN_SEED"), bankVaultPda.toBuffer(), tokenMint.toBuffer()], BANK_PROGRAM_ID)[0],
    solReserve: PublicKey.findProgramAddressSync([Buffer.from("BANK_TOKEN_SEED"), bankVaultPda.toBuffer()], BANK_PROGRAM_ID)[0],
    userReserve: (pubkey: PublicKey, tokenMint?: PublicKey) => {
      let seeds = [Buffer.from("USER_RESERVE_SEED"), pubkey.toBuffer()];
      if (tokenMint) seeds.push(tokenMint.toBuffer());
      return PublicKey.findProgramAddressSync(seeds, BANK_PROGRAM_ID)[0];
    },
  };

  const STAKING_PDAS = {
    stakingVault: PublicKey.findProgramAddressSync([Buffer.from("STAKING_VAULT")], STAKING_PROGRAM_ID)[0],
    userInfo: (ownerPubkey: PublicKey, tokenMint?: PublicKey) => {
      let seeds = [Buffer.from("USER_INFO"), ownerPubkey.toBuffer()];
      if (tokenMint) seeds.push(tokenMint.toBuffer());
      return PublicKey.findProgramAddressSync(seeds, STAKING_PROGRAM_ID)[0];
    }
  };

  let altAccount: anchor.web3.AddressLookupTableAccount;

  it("1.1 Khởi tạo Ngân hàng", async () => {
    try {
      await program.account.bankInfo.fetch(BANK_PDAS.bankInfo);
      console.log("✅ Bank đã được khởi tạo.");
    } catch {
      await program.methods.initialize()
        .accounts({
          bankInfo: BANK_PDAS.bankInfo,
          bankVault: BANK_PDAS.bankVault,
          solReserve: BANK_PDAS.solReserve,
          authority: provider.publicKey,
          systemProgram: SystemProgram.programId
        }).rpc();
      console.log("✅ Khởi tạo Ngân hàng thành công!");
    }
  });

  it("1.2 Khởi tạo Staking Info (SOL và Token)", async () => {
    const tokens = [null, mockTokenMint, mockTokenMint2]; // null đại diện cho SOL
    
    for (const mint of tokens) {
      const userInfoPda = STAKING_PDAS.userInfo(BANK_PDAS.bankVault, mint || undefined);
      try {
        await stakingProgram.account.userInfo.fetch(userInfoPda);
        console.log(`✅ Staking Info cho ${mint ? 'Token' : 'SOL'} đã tồn tại.`);
      } catch {
        // Mượn tay Bank App tạo qua CPI (dùng invest hoặc investToken tùy loại)
        if (mint) {
          await program.methods.investToken(new BN(0), true)
            .accounts({
              bankInfo: BANK_PDAS.bankInfo,
              bankVault: BANK_PDAS.bankVault,
              stakingVault: STAKING_PDAS.stakingVault,
              stakingInfo: userInfoPda,
              stakingProgram: STAKING_PROGRAM_ID,
              tokenProgram: TOKEN_PROGRAM_ID,
              tokenMint: mint,
              associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
              bankAta: getAssociatedTokenAddressSync(mint, BANK_PDAS.bankVault, true),
              stakingAta: getAssociatedTokenAddressSync(mint, STAKING_PDAS.stakingVault, true),
              authority: provider.publicKey,
              systemProgram: SystemProgram.programId,
            }).rpc();
        } else {
          await program.methods.invest(new BN(0), true)
            .accounts({
              bankInfo: BANK_PDAS.bankInfo,
              bankVault: BANK_PDAS.bankVault,
              stakingVault: STAKING_PDAS.stakingVault,
              stakingInfo: userInfoPda,
              stakingProgram: STAKING_PROGRAM_ID,
              authority: provider.publicKey,
              systemProgram: SystemProgram.programId,
            }).rpc();
        }
        console.log(`✅ Đã khởi tạo xong Staking Info cho ${mint ? mint.toBase58() : 'SOL'}`);
      }
    }
  });

  it("1.3 Admin niêm yết các Token vào Bank", async () => {
    const mints = [mockTokenMint, mockTokenMint2];
    for (const mint of mints) {
      try {
        await program.account.tokenReserve.fetch(BANK_PDAS.tokenReserve(mint));
        console.log(`✅ Token ${mint.toBase58()} đã được niêm yết.`);
      } catch {
        await program.methods.addToken()
          .accounts({
            authority: provider.publicKey,
            bankInfo: BANK_PDAS.bankInfo,
            bankVault: BANK_PDAS.bankVault,
            tokenMint: mint,
            tokenReserve: BANK_PDAS.tokenReserve(mint),
            systemProgram: SystemProgram.programId,
          }).rpc();
        console.log(`✅ Đã niêm yết thành công token: ${mint.toBase58()}`);
      }
    }
  });

  it("2. Tạo Address Lookup Table (ALT)", async () => {
    const rawAddresses = [
      BANK_PDAS.bankInfo, BANK_PDAS.bankVault, BANK_PDAS.solReserve, 
      STAKING_PDAS.stakingVault, STAKING_PDAS.userInfo(BANK_PDAS.bankVault),
      STAKING_PROGRAM_ID, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID,
      SystemProgram.programId, provider.publicKey,
    ];

    // Thêm các địa chỉ riêng biệt của từng token vào bảng
    [mockTokenMint, mockTokenMint2].forEach(mint => {
      rawAddresses.push(
        mint,
        BANK_PDAS.tokenReserve(mint),
        BANK_PDAS.userReserve(provider.publicKey, mint),
        STAKING_PDAS.userInfo(BANK_PDAS.bankVault, mint),
        getAssociatedTokenAddressSync(mint, provider.publicKey),
        getAssociatedTokenAddressSync(mint, BANK_PDAS.bankVault, true),
        getAssociatedTokenAddressSync(mint, STAKING_PDAS.stakingVault, true)
      );
    });

    const uniqueAddresses = Array.from(new Set(rawAddresses.map(a => a.toBase58()))).map(a => new PublicKey(a));

    console.log(`⏳ Đang khởi tạo ALT cho ${uniqueAddresses.length} địa chỉ...`);
    altAccount = await setupLookupTable(provider, uniqueAddresses);
  });

  it("3. Deposit SOL (Versioned Transaction)", async () => {
    const amount = new BN(10**9); 

    const depositSolIx = await program.methods.deposit(amount)
      .accounts({
        bankInfo: BANK_PDAS.bankInfo,
        solReserve: BANK_PDAS.solReserve,
        bankVault: BANK_PDAS.bankVault,
        userReserve: BANK_PDAS.userReserve(provider.publicKey),
        stakingVault: STAKING_PDAS.stakingVault,
        stakingProgram: STAKING_PROGRAM_ID,
        stakingInfo: STAKING_PDAS.userInfo(BANK_PDAS.bankVault),
        user: provider.publicKey,
        authority: provider.publicKey,
        systemProgram: SystemProgram.programId,
      }).instruction();

    const latestBlockhash = await provider.connection.getLatestBlockhash();
    const messageV0 = new TransactionMessage({
      payerKey: provider.publicKey,
      recentBlockhash: latestBlockhash.blockhash,
      instructions: [depositSolIx],
    }).compileToV0Message([altAccount]); 

    const transaction = new VersionedTransaction(messageV0);
    transaction.sign([(provider.wallet as anchor.Wallet).payer]);

    const txId = await provider.connection.sendTransaction(transaction);
    console.log("✅ Deposit SOL thành công! Hash:", txId);
  });

  it("4. Batch Deposit Tokens (Nhiều loại Token cùng lúc)", async () => {
    const deposits = [
      { tokenMint: mockTokenMint, amount: new BN(10 * 10**9) },
      { tokenMint: mockTokenMint2, amount: new BN(20 * 10**9) }
    ];

    let instructions: TransactionInstruction[] = [];

    for (const item of deposits) {
      const userAta = getAssociatedTokenAddressSync(item.tokenMint, provider.publicKey);
      const bankAta = getAssociatedTokenAddressSync(item.tokenMint, BANK_PDAS.bankVault, true);

      // Tự động thêm lệnh tạo ví ATA cho Bank nếu chưa có
      if (await provider.connection.getAccountInfo(bankAta) == null) {
        instructions.push(createAssociatedTokenAccountInstruction(
          provider.publicKey, bankAta, BANK_PDAS.bankVault, item.tokenMint
        ));
      }

      const ix = await program.methods.depositToken(item.amount)
        .accounts({
          tokenReserve: BANK_PDAS.tokenReserve(item.tokenMint),
          bankInfo: BANK_PDAS.bankInfo,
          bankVault: BANK_PDAS.bankVault,
          tokenMint: item.tokenMint,
          userAta: userAta,
          bankAta: bankAta,
          userReserve: BANK_PDAS.userReserve(provider.publicKey, item.tokenMint),
          stakingProgram: STAKING_PROGRAM_ID,
          stakingInfo: STAKING_PDAS.userInfo(BANK_PDAS.bankVault, item.tokenMint),
          stakingVault: STAKING_PDAS.stakingVault,
          user: provider.publicKey,
          stakingAta: getAssociatedTokenAddressSync(item.tokenMint, STAKING_PDAS.stakingVault, true),
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          authority: provider.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId
        }).instruction();
        
      instructions.push(ix);
    }

    const latestBlockhash = await provider.connection.getLatestBlockhash();
    const messageV0 = new TransactionMessage({
      payerKey: provider.publicKey,
      recentBlockhash: latestBlockhash.blockhash,
      instructions: instructions,
    }).compileToV0Message([altAccount]); 

    const transaction = new VersionedTransaction(messageV0);
    transaction.sign([(provider.wallet as anchor.Wallet).payer]);

    const txId = await provider.connection.sendTransaction(transaction);
    console.log("✅ Batch Deposit Token thành công! Hash:", txId);
  });

  it("5. Batch Deposit SOL (Nhiều lệnh nạp trong 1 Tx)", async () => {
    const solAmounts = [new BN(0.1 * 10**9), new BN(0.1 * 10**9)];
    let instructions: TransactionInstruction[] = [];

    for (const amount of solAmounts) {
      const ix = await program.methods.deposit(amount)
        .accounts({
          bankInfo: BANK_PDAS.bankInfo,
          solReserve: BANK_PDAS.solReserve,
          bankVault: BANK_PDAS.bankVault,
          userReserve: BANK_PDAS.userReserve(provider.publicKey),
          stakingVault: STAKING_PDAS.stakingVault,
          stakingProgram: STAKING_PROGRAM_ID,
          stakingInfo: STAKING_PDAS.userInfo(BANK_PDAS.bankVault),
          user: provider.publicKey,
          authority: provider.publicKey,
          systemProgram: SystemProgram.programId,
        }).instruction();
      instructions.push(ix);
    }

    const latestBlockhash = await provider.connection.getLatestBlockhash();
    const messageV0 = new TransactionMessage({
      payerKey: provider.publicKey,
      recentBlockhash: latestBlockhash.blockhash,
      instructions: instructions,
    }).compileToV0Message([altAccount]); 

    const transaction = new VersionedTransaction(messageV0);
    transaction.sign([(provider.wallet as anchor.Wallet).payer]);

    const txId = await provider.connection.sendTransaction(transaction);
    console.log("✅ Batch Deposit SOL thành công! Hash:", txId);
  });
  it("6.1 Test Raw CPI Stake SOL", async () => {
      const amount = new BN(0.2 * 10**9); // 0.2 SOL
      const isStake = true;

      // Lấy số dư Staking Vault trước khi gọi
      const balanceBefore = await provider.connection.getBalance(STAKING_PDAS.stakingVault);

      // Gọi instruction invest (lúc này cpi_staking_interaction đã dùng call_stake_sol_raw)
      const ix = await program.methods.invest(amount, isStake)
        .accounts({
          bankInfo: BANK_PDAS.bankInfo,
          bankVault: BANK_PDAS.bankVault,
          stakingVault: STAKING_PDAS.stakingVault,
          stakingInfo: STAKING_PDAS.userInfo(BANK_PDAS.bankVault),
          stakingProgram: STAKING_PROGRAM_ID,
          authority: provider.publicKey,
          systemProgram: SystemProgram.programId, // Hoặc SystemProgram.programId
        }).instruction();

      const latestBlockhash = await provider.connection.getLatestBlockhash();
      const messageV0 = new TransactionMessage({
        payerKey: provider.publicKey,
        recentBlockhash: latestBlockhash.blockhash,
        instructions: [ix],
      }).compileToV0Message([altAccount]);

      const transaction = new VersionedTransaction(messageV0);
      transaction.sign([(provider.wallet as anchor.Wallet).payer]);
      await provider.connection.sendTransaction(transaction);

      // Lấy số dư sau khi gọi
      const balanceAfter = await provider.connection.getBalance(STAKING_PDAS.stakingVault);

      // Assert: Kết quả phải tăng đúng bằng amount (giống hệt logic CPI cũ)
      expect(balanceAfter - balanceBefore).to.equal(amount.toNumber());
      console.log("✅ Raw CPI Stake SOL thành công! Số dư Vault tăng chính xác.");
    });

    it("6.2 Test Raw CPI Stake Token", async () => {
      const amount = new BN(5 * 10**9); // 5 Tokens
      const isStake = true;
      const mint = mockTokenMint;

      const stakingAta = getAssociatedTokenAddressSync(mint, STAKING_PDAS.stakingVault, true);

      // Lấy số dư Token trong Staking ATA trước khi gọi
      const infoBefore = await provider.connection.getTokenAccountBalance(stakingAta);
      const balanceBefore = new BN(infoBefore.value.amount);

      // Gọi instruction investToken (đã được Duy chuyển sang dùng call_stake_token_raw)
      const ix = await program.methods.investToken(amount, isStake)
        .accounts({
          bankInfo: BANK_PDAS.bankInfo,
          bankVault: BANK_PDAS.bankVault,
          stakingVault: STAKING_PDAS.stakingVault,
          stakingInfo: STAKING_PDAS.userInfo(BANK_PDAS.bankVault, mint),
          stakingProgram: STAKING_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          tokenMint: mint,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          bankAta: getAssociatedTokenAddressSync(mint, BANK_PDAS.bankVault, true),
          stakingAta: stakingAta,
          authority: provider.publicKey,
          systemProgram: SystemProgram.programId,
        }).instruction();

      const latestBlockhash = await provider.connection.getLatestBlockhash();
      const messageV0 = new TransactionMessage({
        payerKey: provider.publicKey,
        recentBlockhash: latestBlockhash.blockhash,
        instructions: [ix],
      }).compileToV0Message([altAccount]);

      const transaction = new VersionedTransaction(messageV0);
      transaction.sign([(provider.wallet as anchor.Wallet).payer]);
      await provider.connection.sendTransaction(transaction);

      // Lấy số dư sau khi gọi
      const infoAfter = await provider.connection.getTokenAccountBalance(stakingAta);
      const balanceAfter = new BN(infoAfter.value.amount);

      // Assert: Chênh lệch số dư phải khớp với amount
      expect(balanceAfter.sub(balanceBefore).toString()).to.equal(amount.toString());
      console.log("✅ Raw CPI Stake Token thành công! Token đã được forward sang Staking App.");
    });

});
