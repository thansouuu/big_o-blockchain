use anchor_lang::{prelude::*, system_program};

use crate::{
    constant::{BANK_TOKEN_SEED,BANK_INFO_SEED, BANK_VAULT_SEED, USER_RESERVE_SEED},
    error::BankAppError,
    state::{BankInfo, UserReserve,SolReserve},
    transfer_helper::sol_transfer_from_user,
};

use staking_app::{
    constant::{USER_INFO,STAKING_APR,SECOND_PER_YEAR},
    state::{UserInfo},
    program::StakingApp,
};

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(
        seeds = [BANK_INFO_SEED],
        bump
    )]
    pub bank_info: Box<Account<'info, BankInfo>>,
    #[account(
        mut,
        seeds = [
            BANK_TOKEN_SEED,
            bank_vault.key().as_ref(),
        ],
        bump,
    )]
    pub sol_reserve: Box<Account<'info, SolReserve>>,
    ///CHECK:
    #[account(
        mut,
        seeds = [BANK_VAULT_SEED],
        bump,
        owner = system_program::ID
    )]
    pub bank_vault: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        seeds = [USER_RESERVE_SEED, user.key().as_ref()],
        bump,
        payer = user,
        space = 8 + std::mem::size_of::<UserReserve>(),
    )]
    pub user_reserve: Box<Account<'info, UserReserve>>,
    pub staking_program: Program<'info,StakingApp>,
    #[account(
        seeds = [USER_INFO, bank_vault.key().as_ref()],
        bump,
        seeds::program = staking_program.key(),
    )]
    pub staking_info: Box<Account<'info, UserInfo>>,

    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> Deposit<'info> {
    pub fn process(ctx: Context<Deposit>, deposit_amount: u64) -> Result<()> {
        if ctx.accounts.bank_info.is_paused {
            return Err(BankAppError::BankAppPaused.into());
        }

        let user_reserve = &mut ctx.accounts.user_reserve;
        let staking_info=&ctx.accounts.staking_info;
        let token_share = &mut ctx.accounts.sol_reserve.token_share;

        let new_share= if (*token_share)==0 {
            deposit_amount
        }
        else {
            let current_time: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
            let pass_time = if staking_info.last_update_time == 0 {
                0
            } else {
                current_time - staking_info.last_update_time
            };
            let lai = staking_info.amount * STAKING_APR * pass_time / 100 / SECOND_PER_YEAR;
            let rent = Rent::get()?;
            let total_asset=staking_info.amount+ctx.accounts.bank_vault.lamports()-rent.minimum_balance(0)+lai;
            (*token_share)*deposit_amount/total_asset
        };
        (*token_share)+=new_share;
        user_reserve.deposited_amount+=new_share;
        sol_transfer_from_user(
            &ctx.accounts.user,
            ctx.accounts.bank_vault.to_account_info(),
            &ctx.accounts.system_program,
            deposit_amount,
        )?;

        Ok(())
    }
}
