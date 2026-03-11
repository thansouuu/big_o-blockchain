use anchor_lang::{prelude::*, system_program};

use crate::{
    constant::{BANK_INFO_SEED, BANK_VAULT_SEED, USER_RESERVE_SEED},
    error::BankAppError,
    state::{BankInfo, UserReserve},
    transfer_helper::sol_transfer_from_user,
};

#[derive(Accounts)]
pub struct TogglePause<'info>{
    #[account(
        mut,
        seeds=[BANK_INFO_SEED],
        bump
    )]
    pub bank_info:Account<'info,BankInfo>,
    #[account(
        mut,
        address=bank_info.authority
    )]
    pub authority:Signer<'info>,
}

impl<'info> TogglePause<'info>{
    pub fn process(ctx:Context<TogglePause>)->Result<()>{
        ctx.accounts.bank_info.is_paused=!ctx.accounts.bank_info.is_paused;
        Ok(())
    }
}