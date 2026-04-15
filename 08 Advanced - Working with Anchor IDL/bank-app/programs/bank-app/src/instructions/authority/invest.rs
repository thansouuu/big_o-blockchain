use anchor_lang::{prelude::*, system_program};

use crate::{
    constant::{BANK_INFO_SEED, BANK_VAULT_SEED},
    error::BankAppError,
    state::BankInfo,
    transfer_helper::cpi_staking_interaction,
};
use staking_app::{cpi, program::StakingApp};

#[derive(Accounts)]
pub struct Invest<'info> {
    #[account(
        seeds = [BANK_INFO_SEED],
        bump
    )]
    pub bank_info: Box<Account<'info, BankInfo>>,

    ///CHECK:
    #[account(
        mut,
        seeds = [BANK_VAULT_SEED],
        bump,
        owner = system_program::ID
    )]
    pub bank_vault: UncheckedAccount<'info>,

    ///CHECK:
    #[account(mut)]
    pub staking_vault: UncheckedAccount<'info>,
    ///CHECK:
    #[account(mut)]
    pub staking_info: UncheckedAccount<'info>,
    pub staking_program: Program<'info, StakingApp>,

    #[account(mut, address = bank_info.authority)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> Invest<'info> {
    pub fn process(ctx: Context<Invest>, amount: u64, is_stake: bool) -> Result<()> {
        if ctx.accounts.bank_info.is_paused {
            return Err(BankAppError::BankAppPaused.into());
        }

        let bank_info_bump = ctx.accounts.bank_info.bump;
        let pda_seeds: &[&[&[u8]]] = &[&[BANK_VAULT_SEED, &[bank_info_bump]]];


        cpi_staking_interaction(
            ctx.accounts.staking_program.to_account_info(),
            ctx.accounts.staking_vault.to_account_info(),
            ctx.accounts.staking_info.to_account_info(),
            ctx.accounts.bank_vault.to_account_info(),
            ctx.accounts.authority.to_account_info(), // Admin trả phí
            ctx.accounts.system_program.to_account_info(),
            amount,
            is_stake, 
            pda_seeds
        )?;

        Ok(())
    }
}
