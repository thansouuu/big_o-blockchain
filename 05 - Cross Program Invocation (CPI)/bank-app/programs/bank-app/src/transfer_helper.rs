use anchor_lang::{
    prelude::*,
    solana_program::{
        program::{invoke, invoke_signed},
        system_instruction::transfer,
    },
};
use anchor_spl::token::{self, Token};
use staking_app::cpi::accounts::Stake as CpiStake;
use staking_app::cpi::accounts::StakeToken as CpiStakeToken;
use staking_app::program::StakingApp;

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
    
    let cpi_accounts = CpiStake {
        staking_vault,
        user_info,
        user: bank_vault, 
        payer,
        system_program,
    };
    let cpi_ctx = CpiContext::new_with_signer(
        staking_program, 
        cpi_accounts, 
        bank_vault_seeds
    );
    staking_app::cpi::staking_sol(cpi_ctx, amount, is_stake)
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
    
    let cpi_accounts = CpiStakeToken {
        staking_vault,
        token_mint,
        user_ata: bank_ata,         // Bank mang ví của mình đi cắm
        staking_ata,
        user_info,
        user: bank_vault,           // Bank đứng tên sổ cắm cọc
        payer,
        token_program,
        system_program,
        associated_token_program,
    };

    let cpi_ctx = CpiContext::new_with_signer(staking_program, cpi_accounts, signer_seeds);

    staking_app::cpi::staking_token(cpi_ctx, amount, is_stake)
}