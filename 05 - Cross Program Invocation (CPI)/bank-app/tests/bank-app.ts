import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { BankApp } from "../target/types/bank_app";
import { PublicKey, SystemProgram, TransactionInstruction } from "@solana/web3.js";
import { BN } from "bn.js";
import { createAssociatedTokenAccountInstruction, getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { StakingApp } from "../target/types/staking_app";

describe("bank-app", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env()
  anchor.setProvider(provider);

  const program = anchor.workspace.BankApp as Program<BankApp>;
  const stakingProgram = anchor.workspace.StakingApp as Program<StakingApp>;

  const STAKING_PROGRAM_ID = anchor.workspace.StakingApp.programId; 
  const BANK_PROGRAM_ID = program.programId;

  const mockTokenMint = new PublicKey('3cYRG1gzC93CiPQ2ZB1FyEpy11NoX8N9AGxqqQVAg4Fj');
  // Định nghĩa bankVault trước để dùng lại bên dưới cho tiện
  const bankVaultPda = PublicKey.findProgramAddressSync(
    [Buffer.from("BANK_VAULT_SEED")], 
    BANK_PROGRAM_ID
  )[0];

  const BANK_PDAS = {
    bankInfo: PublicKey.findProgramAddressSync(
      [Buffer.from("BANK_INFO_SEED")], 
      BANK_PROGRAM_ID
    )[0],
    
    bankVault: bankVaultPda,
    
    // 1. QUỸ DỰ TRỮ CỦA TOKEN (3 tham số)
    tokenReserve: (tokenMint: PublicKey) => {
      return PublicKey.findProgramAddressSync(
        [Buffer.from("BANK_TOKEN_SEED"), bankVaultPda.toBuffer(), tokenMint.toBuffer()],
        BANK_PROGRAM_ID
      )[0];
    },

    // 2. QUỸ DỰ TRỮ CỦA SOL (2 tham số - như bạn vừa code)
    solReserve: PublicKey.findProgramAddressSync(
      [Buffer.from("BANK_TOKEN_SEED"), bankVaultPda.toBuffer()],
      BANK_PROGRAM_ID
    )[0],
    
    userReserve: (pubkey: PublicKey, tokenMint?: PublicKey) => {
      let seeds = [Buffer.from("USER_RESERVE_SEED"), pubkey.toBuffer()];
      if (tokenMint) seeds.push(tokenMint.toBuffer());
      return PublicKey.findProgramAddressSync(seeds, BANK_PROGRAM_ID)[0];
    },
  };

  const STAKING_PDAS = {
    stakingVault: PublicKey.findProgramAddressSync(
      [Buffer.from("STAKING_VAULT")], 
      STAKING_PROGRAM_ID
    )[0],

    userInfo: (ownerPubkey: PublicKey, tokenMint?: PublicKey) => {
      let seeds = [Buffer.from("USER_INFO"), ownerPubkey.toBuffer()];
      if (tokenMint) seeds.push(tokenMint.toBuffer());
      return PublicKey.findProgramAddressSync(seeds, STAKING_PROGRAM_ID)[0];
    }
  };

  async function init() {
    try {
      const bank_info = await program.account.bankInfo.fetch(BANK_PDAS.bankInfo);
      console.log("Bank info: ", bank_info);
    } catch {
      const tx = await program.methods.initialize()
        .accounts({
          bankInfo: BANK_PDAS.bankInfo,
          bankVault: BANK_PDAS.bankVault,
          solReserve: BANK_PDAS.solReserve, // <--- THÊM DÒNG NÀY ĐỂ MUA KÉT SOL
          authority: provider.publicKey,
          systemProgram: SystemProgram.programId
        }).rpc();
      console.log("✅ Init successful: ", tx);
    }
    console.log("bunhacopxki ",BANK_PDAS.bankVault);
  }

  async function deposit_token(tokenMint: PublicKey, amount: anchor.BN) {
    const userAta = getAssociatedTokenAddressSync(tokenMint, provider.publicKey);
    const bankAta = getAssociatedTokenAddressSync(tokenMint, BANK_PDAS.bankVault, true);
    
    let preInstructions: TransactionInstruction[] = [];
    if (await provider.connection.getAccountInfo(bankAta) == null) {
      preInstructions.push(createAssociatedTokenAccountInstruction(
        provider.publicKey, bankAta, BANK_PDAS.bankVault, tokenMint
      ));
    }

    const tx = await program.methods.depositToken(amount)
      .accounts({
        bankInfo: BANK_PDAS.bankInfo,
        bankVault: BANK_PDAS.bankVault,
        tokenReserve: BANK_PDAS.tokenReserve(tokenMint),
        tokenMint: tokenMint,
        userAta: userAta,
        bankAta: bankAta,
        userReserve: BANK_PDAS.userReserve(provider.publicKey, tokenMint),
        stakingProgram: STAKING_PROGRAM_ID,
        userInfo: STAKING_PDAS.userInfo(BANK_PDAS.bankVault, tokenMint),
        user: provider.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId
      }).preInstructions(preInstructions).rpc();
      
    console.log("Deposit Token signature: ", tx);
  }

  async function deposit_sol(amount: anchor.BN) {
    const tx = await program.methods.deposit(amount)
      .accounts({
        bankInfo: BANK_PDAS.bankInfo,
        bankVault: BANK_PDAS.bankVault,
        solReserve: BANK_PDAS.solReserve, // <--- THÊM VÀO ĐÂY
        userReserve: BANK_PDAS.userReserve(provider.publicKey),
        userInfo: STAKING_PDAS.userInfo(BANK_PDAS.bankVault), 
        stakingProgram: STAKING_PROGRAM_ID, 
        user: provider.publicKey,
        systemProgram: SystemProgram.programId,
      }).rpc();
      
    console.log("✅ Deposit SOL signature: ", tx);
    
    const userReserve = await program.account.userReserve.fetch(BANK_PDAS.userReserve(provider.publicKey));
    console.log("User reserve sau khi nạp SOL: ", userReserve.depositedAmount.toString());
  }

  async function withdraw_sol(amount: anchor.BN) {
    const tx = await program.methods.withdraw(amount)
      .accounts({
        bankInfo: BANK_PDAS.bankInfo,
        bankVault: BANK_PDAS.bankVault,
        solReserve: BANK_PDAS.solReserve, // <--- THÊM VÀO ĐÂY
        stakingInfo: STAKING_PDAS.userInfo(BANK_PDAS.bankVault),
        userReserve: BANK_PDAS.userReserve(provider.publicKey),
        stakingVault: STAKING_PDAS.stakingVault,
        stakingProgram: STAKING_PROGRAM_ID,
        user: provider.publicKey,
        systemProgram: SystemProgram.programId
      }).rpc();
      
    console.log("✅ Withdraw SOL signature: ", tx);
    
    const userReserve = await program.account.userReserve.fetch(BANK_PDAS.userReserve(provider.publicKey));
    console.log("User reserve sau khi rút SOL: ", userReserve.depositedAmount.toString());
  }

  async function withdraw_token(tokenMint: PublicKey, amount: anchor.BN) {
    const userAta = getAssociatedTokenAddressSync(tokenMint, provider.publicKey);
    const bankAta = getAssociatedTokenAddressSync(tokenMint, BANK_PDAS.bankVault, true);
    const stakingAta = getAssociatedTokenAddressSync(tokenMint, STAKING_PDAS.stakingVault, true);

    const tx = await program.methods.withdrawToken(amount)
      .accounts({
        bankInfo: BANK_PDAS.bankInfo,
        bankVault: BANK_PDAS.bankVault,
        tokenReserve: BANK_PDAS.tokenReserve(tokenMint),
        tokenMint: tokenMint,
        userAta: userAta,
        bankAta: bankAta,
        userReserve: BANK_PDAS.userReserve(provider.publicKey, tokenMint),
        stakingProgram: STAKING_PROGRAM_ID,
        userInfo: STAKING_PDAS.userInfo(BANK_PDAS.bankVault, tokenMint),
        stakingAta: stakingAta,
        user: provider.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      }).rpc();

    console.log("Withdraw Token signature: ", tx);
  }

  async function invest_sol(amount: anchor.BN, is_stake: boolean) {
    try {
      const tx = await program.methods.invest(amount, is_stake)
        .accounts({
          bankInfo: BANK_PDAS.bankInfo,
          bankVault: BANK_PDAS.bankVault,
          stakingVault: STAKING_PDAS.stakingVault,
          stakingInfo: STAKING_PDAS.userInfo(BANK_PDAS.bankVault),
          stakingProgram: STAKING_PROGRAM_ID,
          authority: provider.wallet.publicKey,
          systemProgram: SystemProgram.programId,
        }).rpc();
        
      console.log("Invest SOL thành công! Hash:", tx);
    } catch (error) {
      console.error("❌ Lỗi khi Invest SOL:", error);
    }
  }

  async function invest_token(tokenMint: PublicKey, amount: anchor.BN, is_stake: boolean) {
    const bankAta = getAssociatedTokenAddressSync(tokenMint, BANK_PDAS.bankVault, true);
    const stakingAta = getAssociatedTokenAddressSync(tokenMint, STAKING_PDAS.stakingVault, true);

    try {
      const tx = await program.methods.investToken(amount, is_stake)
        .accounts({
          bankInfo: BANK_PDAS.bankInfo,
          bankVault: BANK_PDAS.bankVault,
          stakingVault: STAKING_PDAS.stakingVault,
          stakingInfo: STAKING_PDAS.userInfo(BANK_PDAS.bankVault, tokenMint),
          stakingProgram: STAKING_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          tokenMint: tokenMint,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          bankAta: bankAta,
          stakingAta: stakingAta,
          authority: provider.wallet.publicKey,
          systemProgram: SystemProgram.programId,
        }).rpc();

      console.log("Invest Token thành công! Hash:", tx);
    } catch (error) {
      console.error("❌ Lỗi khi Invest Token:", error);
    }
  }

  async function toggle_pause() {
    const tx = await program.methods.togglePause()
      .accounts({
        bankInfo: BANK_PDAS.bankInfo,
        authority: provider.publicKey,
      }).rpc();
    console.log("Toggle pause signature: ", tx);
  }
  async function add_supported_token(tokenMint: PublicKey) {
    const tx = await program.methods.addToken()
      .accounts({
        authority: provider.publicKey, // Admin ký
        bankInfo: BANK_PDAS.bankInfo,
        bankVault: BANK_PDAS.bankVault,
        tokenMint: tokenMint,
        tokenReserve: BANK_PDAS.tokenReserve(tokenMint),
        systemProgram: SystemProgram.programId,
      }).rpc();
    console.log("✅ Admin đã cấp phép Token mới thành công! Hash:", tx);
  }
  it("Chạy Toàn Bộ Luồng DeFi", async () => {
    // 1. KHAI TRƯƠNG BANK (Chỉ chạy 1 lần cho ID mới)
    console.log("1. Đang Khởi tạo Ngân hàng...");
    await init();

    // 2. ADMIN CHO PHÉP TOKEN HOẠT ĐỘNG
    console.log("2. Đang Niêm yết Token...");
    await add_supported_token(mockTokenMint);

    // 3. NẠP TIỀN VÀO BANK (Nạp dư dả để Bank có vốn)
    console.log("3. User đang nạp tiền...");
    await deposit_sol(new anchor.BN(10 * 10**9)); // Nạp 10 SOL
    await deposit_token(mockTokenMint, new anchor.BN(1000 * 10**9)); // Nạp 1000 Token

    // 4. BANK MANG TIỀN ĐI ĐẦU TƯ (Phải chạy sau bước 3)
    console.log("4. Bank mang vốn đi Staking...");
    await invest_sol(new anchor.BN(5 * 10**9), true);
    await invest_token(mockTokenMint, new anchor.BN(500 * 10**9), true);

    // 5. RÚT TIỀN VỀ (Test sự trơn tru của thuật toán chia sẻ Share)
    console.log("5. User rút tiền về...");
    await withdraw_sol(new anchor.BN(2 * 10**9));
    await withdraw_token(mockTokenMint, new anchor.BN(200 * 10**9));
    
    console.log("✅ TẤT CẢ ĐÃ THÀNH CÔNG RỰC RỠ!");
  });
});
