import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { BankApp } from "../target/types/bank_app";
import { PublicKey, SystemProgram, TransactionInstruction } from "@solana/web3.js";
import { BN } from "bn.js";
import { createAssociatedTokenAccountInstruction, getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID } from "@solana/spl-token";

describe("bank-app", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env()
  anchor.setProvider(provider);

  const program = anchor.workspace.BankApp as Program<BankApp>;

  const BANK_APP_ACCOUNTS = {
    bankInfo: PublicKey.findProgramAddressSync(
      [Buffer.from("BANK_INFO_SEED")],
      program.programId
    )[0],
    bankVault: PublicKey.findProgramAddressSync(
      [Buffer.from("BANK_VAULT_SEED")],
      program.programId
    )[0],
    userReserve: (pubkey: PublicKey, tokenMint?: PublicKey) => {
      let SEEDS = [
        Buffer.from("USER_RESERVE_SEED"),
        pubkey.toBuffer(),
      ]

      if (tokenMint != undefined) {
        SEEDS.push(tokenMint.toBuffer())
      }

      return PublicKey.findProgramAddressSync(
        SEEDS,
        program.programId
      )[0]
    }
  }

  async function init(){
    try{
      const bank_info=await program.account.bankInfo.fetch(BANK_APP_ACCOUNTS.bankInfo);
      console.log("Bank info: ",bank_info);
    }
    catch {
      const tx = await program.methods.initialize()
      .accounts({
        bankInfo: BANK_APP_ACCOUNTS.bankInfo,
        bankVault: BANK_APP_ACCOUNTS.bankVault,
        authority: provider.publickey,
        systemProgram: SystemProgram.programId
      }).rpc();
      console.log("Init successfull: ",tx);
    }
  }

  async function deposit_sol(amount:anchor.BN){
    const tx=await program.methods.deposit(amount)
    .accounts({
      bankInfo:BANK_APP_ACCOUNTS.bankInfo,
      bankVault:BANK_APP_ACCOUNTS.bankVault,
      userReserve: BANK_APP_ACCOUNTS.userReserve(provider.publicKey),
      user:provider.publicKey,
      systemProgram:SystemProgram.programId
    }).rpc();
    console.log("Deposit sol ok ",tx);
    const userReserve=await program.account.userReserve.fetch(BANK_APP_ACCOUNTS.userReserve(provider.publicKey));
    console.log("User reserve: ",userReserve.depositedAmount.toString());
  }

  async function deposit_token(tokenMint:anchor.web3.PublicKey,amount:anchor.BN){
    let userAta = getAssociatedTokenAddressSync(tokenMint, provider.publicKey)
    let bankAta = getAssociatedTokenAddressSync(tokenMint, BANK_APP_ACCOUNTS.bankVault, true)
    let preInstructions: TransactionInstruction[] = []
      if (await provider.connection.getAccountInfo(bankAta) == null) {
        preInstructions.push(createAssociatedTokenAccountInstruction(
          provider.publicKey,
          bankAta,
          BANK_APP_ACCOUNTS.bankVault,
          tokenMint
        ))
      }
    console.log("Địa chỉ ATA của User:", userAta.toBase58());
    console.log("Địa chỉ ATA của Bank Vault:", bankAta.toBase58());
    try {
      const userBalance = await provider.connection.getTokenAccountBalance(userAta);
      const bankBalance = await provider.connection.getTokenAccountBalance(bankAta);
      console.log("Số dư ví User:", userBalance.value.uiAmount, "Tokens");
      console.log("Số dư két Bank:", bankBalance.value.uiAmount, "Tokens");
    } catch (e) {
      console.log("Một trong hai ví ATA chưa được khởi tạo trên mạng lưới!");
    }
    const tx = await program.methods.depositToken(new BN(1_000_000_000))
      .accounts({
        bankInfo: BANK_APP_ACCOUNTS.bankInfo,
        bankVault: BANK_APP_ACCOUNTS.bankVault,
        tokenMint,
        userAta,
        bankAta,
        userReserve: BANK_APP_ACCOUNTS.userReserve(provider.publicKey, tokenMint),
        user: provider.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId
      }).preInstructions(preInstructions).rpc();
    console.log("Deposit token signature: ", tx);

    const userReserve = await program.account.userReserve.fetch(BANK_APP_ACCOUNTS.userReserve(provider.publicKey, tokenMint))
    console.log("User reserve: ", userReserve.depositedAmount.toString())
  }
  async function withdraw_sol(amount:anchor.BN){
    const tx = await program.methods.withdraw(amount)
      .accounts({
        bankInfo: BANK_APP_ACCOUNTS.bankInfo,
        bankVault: BANK_APP_ACCOUNTS.bankVault,
        userReserve: BANK_APP_ACCOUNTS.userReserve(provider.publicKey),
        user: provider.publicKey,
        systemProgram: SystemProgram.programId
      }).rpc();
    console.log("Withdraw signature: ", tx);

    const userReserve = await program.account.userReserve.fetch(BANK_APP_ACCOUNTS.userReserve(provider.publicKey));
    console.log("User reserve after withdraw: ", userReserve.depositedAmount.toString());
  }
  async function toggle_pause(){
    const tx = await program.methods.togglePause()
      .accounts({
        bankInfo: BANK_APP_ACCOUNTS.bankInfo,
        authority: provider.publicKey, 
      }).rpc();
    console.log("Toggle pause signature: ", tx);

    const bankInfo = await program.account.bankInfo.fetch(BANK_APP_ACCOUNTS.bankInfo);
    console.log("Is bank paused now? ", bankInfo.isPaused);
  }
  async function withdraw_token(tokenMint: anchor.web3.PublicKey, amount: anchor.BN) {
    const userAta = getAssociatedTokenAddressSync(
      tokenMint,
      provider.publicKey
    );
    const bankAta = getAssociatedTokenAddressSync(
        tokenMint, 
        BANK_APP_ACCOUNTS.bankVault, 
        true 
    );
    const tx = await program.methods.withdrawToken(amount)
      .accounts({
        bankInfo: BANK_APP_ACCOUNTS.bankInfo,
        bankVault: BANK_APP_ACCOUNTS.bankVault,
        tokenMint: tokenMint,
        userAta: userAta, 
        bankAta: bankAta,
        userReserve: BANK_APP_ACCOUNTS.userReserve(provider.publicKey, tokenMint),
        user: provider.publicKey,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    return tx;
  }

  
  // it("Is pause!",async()=>{
  //   await toggle_pause();
  // });
  it("Is initialized!", async () => {
    await init();
  });

  it("Is deposited!", async () => {
    await deposit_sol(
      new anchor.BN(1000) 
    );
  });

  it("Is deposited token!", async () => {
    await deposit_token(
      new anchor.web3.PublicKey("3cYRG1gzC93CiPQ2ZB1FyEpy11NoX8N9AGxqqQVAg4Fj"), // Chuyển String -> PublicKey
      new anchor.BN(1000) 
    );
  });
  
  it("Is withdraw sol!",async()=>{
    await withdraw_sol(
      new anchor.BN(1000000000000)
    );
  });
  it("Is withdraw token!",async()=>{
    await withdraw_token(
      new anchor.web3.PublicKey("3cYRG1gzC93CiPQ2ZB1FyEpy11NoX8N9AGxqqQVAg4Fj"), // Chuyển String -> PublicKey
      new anchor.BN(1000) 
    );
  });

});
