use anchor_lang::{prelude::*, system_program};

use crate::{
    constant::{BANK_TOKEN_SEED,BANK_INFO_SEED, BANK_VAULT_SEED},
    state::{BankInfo,SolReserve},
};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        seeds = [BANK_INFO_SEED],
        bump,
        payer = authority,
        space = 8 + std::mem::size_of::<BankInfo>(),
    )]
    pub bank_info: Box<Account<'info, BankInfo>>,

    ///CHECK:
    #[account(
        init,
        seeds = [BANK_VAULT_SEED],
        bump,
        payer = authority,
        space = 0,
        owner = system_program::ID
    )]
    pub bank_vault: UncheckedAccount<'info>,
    #[account(
        init,
        payer = authority,
        seeds = [
            BANK_TOKEN_SEED,
            bank_vault.key().as_ref(), 
        ],
        bump,
        space = 8 + std::mem::size_of::<SolReserve>() 
    )]
    pub sol_reserve: Box<Account<'info, SolReserve>>,

    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn process(ctx: Context<Initialize>) -> Result<()> {
        let bank_info = &mut ctx.accounts.bank_info;

        bank_info.authority = ctx.accounts.authority.key();
        bank_info.is_paused = false;
        bank_info.bump = ctx.bumps.bank_vault;
        ctx.accounts.sol_reserve.token_share=0;
        msg!("bank app initialized!");
        Ok(())
    }
}
