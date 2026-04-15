use anchor_lang::{
    prelude::*,
    solana_program::{
        program::{invoke, invoke_signed},
        system_instruction::transfer,
        instruction::{Instruction, AccountMeta},
        hash::hash,
    },
    
};
use anchor_spl::token::{self, Token};
// use staking_app::cpi::accounts::Stake as CpiStake;
// use staking_app::cpi::accounts::StakeToken as CpiStakeToken;
// use staking_app::program::StakingApp;

use staking_cpi::cpi::accounts::StakingSol as CpiStake;
use staking_cpi::cpi::accounts::StakingToken as CpiStakeToken;
use staking_cpi::cpi::accounts::StakingSol;
use staking_cpi::cpi::accounts::StakingToken;

pub fn sol_transfer_from_user<'info>(
    signer: &Signer<'info>,
    destination: AccountInfo<'info>,
    system_program: &Program<'info, System>,
    amount: u64,
) -> Result<()> {
    let ix = transfer(signer.key, destination.key, amount);
    invoke(
        &ix,
        &[
            signer.to_account_info(),
            destination,
            system_program.to_account_info(),
        ],
    )?;
    Ok(())
}

pub fn sol_transfer_from_pda<'info>(
    source: AccountInfo<'info>,
    destination: AccountInfo<'info>,
    system_program: &Program<'info, System>,
    pda_seeds: &[&[&[u8]]],
    amount: u64,
) -> Result<()> {
        let ix=transfer(source.key,destination.key,amount);
        invoke_signed(
            &ix,
            &[
                source,
                destination,
                system_program.to_account_info(),
            ],
            pda_seeds,
        )?;
    Ok(())
}

pub fn token_transfer_from_user<'info>(
    from: AccountInfo<'info>,
    authority: &Signer<'info>,
    to: AccountInfo<'info>,
    token_program: &Program<'info, Token>,
    amount: u64,
) -> Result<()> {
    let cpi_ctx: CpiContext<_> = CpiContext::new(
        token_program.to_account_info(),
        token::Transfer {
            from,
            authority: authority.to_account_info(),
            to,
        },
    );
    token::transfer(cpi_ctx, amount)?;
    Ok(())
}

pub fn token_transfer_from_pda<'info>(
    from: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    to: AccountInfo<'info>,
    token_program: &Program<'info, Token>,
    pda_seeds: &[&[&[u8]]],
    amount: u64,
) -> Result<()> {
        let ix=token::spl_token::instruction::transfer(token_program.key,from.key,to.key,authority.key,&[],amount)?;
        invoke_signed(
            &ix,
            &[
                from,
                to,
                authority,
                token_program.to_account_info(),
            ],
            pda_seeds,
        )?;
    Ok(())
}

pub fn cpi_staking_interaction<'info>(
    staking_program: AccountInfo<'info>,
    staking_vault: AccountInfo<'info>,
    user_info: AccountInfo<'info>,   
    bank_vault: AccountInfo<'info>,   
    payer: AccountInfo<'info>,       
    system_program: AccountInfo<'info>,
    amount: u64,
    is_stake: bool,               
    bank_vault_seeds: &[&[&[u8]]], 
) -> Result<()> {
    

    //code bài tập 1 
    // let cpi_accounts = CpiStake {
    //     staking_vault,
    //     user_info,
    //     user: bank_vault, 
    //     payer,
    //     system_program,
    // };
    // let cpi_ctx = CpiContext::new_with_signer(
    //     staking_program, 
    //     cpi_accounts, 
    //     bank_vault_seeds
    // );
    // // staking_app::cpi::staking_sol(cpi_ctx, amount, is_stake)
    // staking_cpi::cpi::staking_sol(cpi_ctx, amount, is_stake)

    //code bài tập 2 
    let account_infos = [
        staking_vault,      
        user_info,         
        bank_vault,        
        payer,              
        system_program,     
    ];

    call_stake_sol_raw(
        &account_infos, 
        staking_program.key, 
        amount, 
        is_stake, 
        bank_vault_seeds
    )
}

pub fn cpi_staking_interaction_token<'info>(
    staking_program: AccountInfo<'info>,
    staking_vault: AccountInfo<'info>,
    token_mint: AccountInfo<'info>,
    bank_ata: AccountInfo<'info>,      
    staking_ata: AccountInfo<'info>,
    user_info: AccountInfo<'info>,
    bank_vault: AccountInfo<'info>,    
    payer: AccountInfo<'info>,      
    token_program: AccountInfo<'info>,
    system_program: AccountInfo<'info>,
    associated_token_program: AccountInfo<'info>,
    amount: u64,
    is_stake: bool,
    signer_seeds: &[&[&[u8]]],     
) -> Result<()> {
    
    //code bài 1
    // let cpi_accounts = CpiStakeToken {
    //     staking_vault,
    //     token_mint,
    //     user_ata: bank_ata,         // Bank mang ví của mình đi cắm
    //     staking_ata,
    //     user_info,
    //     user: bank_vault,           // Bank đứng tên sổ cắm cọc
    //     payer,
    //     token_program,
    //     system_program,
    //     associated_token_program,
    // };

    // let cpi_ctx = CpiContext::new_with_signer(staking_program, cpi_accounts, signer_seeds);

    // // staking_app::cpi::staking_token(cpi_ctx, amount, is_stake)
    // staking_cpi::cpi::staking_token(cpi_ctx, amount, is_stake)

    //code bài 2
    let account_infos = [
        staking_vault,            
        token_mint,                
        bank_ata,                  
        staking_ata,                
        user_info,                
        bank_vault,                 
        payer,                     
        token_program,              
        system_program,             
        associated_token_program,   
    ];

    call_stake_token_raw(
        &account_infos,
        staking_program.key,
        amount,
        is_stake,
        signer_seeds
    )
}

pub fn call_stake_sol_raw(
    accounts: &[AccountInfo],
    staking_program_id: &Pubkey,
    amount: u64,
    is_stake: bool,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    let mut data: Vec<u8> = Vec::with_capacity(17); // 8 (disc) + 8 (u64) + 1 (bool)
    
    let disc = &hash("global:staking_sol".as_bytes()).to_bytes()[..8];
    data.extend_from_slice(disc);
    data.extend_from_slice(&amount.to_le_bytes());
    data.push(if is_stake { 1 } else { 0 });

    let account_metas = vec![
        AccountMeta::new(*accounts[0].key, false),     // staking_vault (writable)
        AccountMeta::new(*accounts[1].key, false),     // user_info (writable)
        AccountMeta::new(*accounts[2].key, true),      // user (signer, writable)
        AccountMeta::new(*accounts[3].key, true),      // payer (signer, writable)
        AccountMeta::new_readonly(*accounts[4].key, false), // system_program (readonly)
    ];

    let instruction = Instruction {
        program_id: *staking_program_id,
        accounts: account_metas,
        data,
    };

    invoke_signed(
        &instruction,
        accounts,
        signer_seeds, 
    )?;

    Ok(())
}

pub fn call_stake_token_raw(
    accounts: &[AccountInfo], 
    staking_program_id: &Pubkey,
    amount: u64,
    is_stake: bool,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    let mut data: Vec<u8> = Vec::with_capacity(17);
    let disc = &hash("global:staking_token".as_bytes()).to_bytes()[..8];
    
    data.extend_from_slice(disc);
    data.extend_from_slice(&amount.to_le_bytes());
    data.push(if is_stake { 1 } else { 0 });

    let account_metas = vec![
        AccountMeta::new(*accounts[0].key, false),          // staking_vault (W)
        AccountMeta::new_readonly(*accounts[1].key, false), // token_mint (R)
        AccountMeta::new(*accounts[2].key, false),          // user_ata (W)
        AccountMeta::new(*accounts[3].key, false),          // staking_ata (W)
        AccountMeta::new(*accounts[4].key, false),          // user_info (W)
        AccountMeta::new(*accounts[5].key, true),           // user (PDA - Signer, W)
        AccountMeta::new(*accounts[6].key, true),           // payer (Signer, W)
        AccountMeta::new_readonly(*accounts[7].key, false), // token_program (R)
        AccountMeta::new_readonly(*accounts[8].key, false), // system_program (R)
        AccountMeta::new_readonly(*accounts[9].key, false), // associated_token_program (R)
    ];

    let instruction = Instruction {
        program_id: *staking_program_id,
        accounts: account_metas,
        data,
    };
    invoke_signed(&instruction, accounts, signer_seeds)?;

    Ok(())
}