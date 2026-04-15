use anchor_lang::{prelude::*, system_program};
use anchor_spl::{
    token::Token,
    token_interface::{Mint, TokenAccount},
};

use crate::{
    constant::{STAKING_APR, SECOND_PER_YEAR, STAKING_VAULT, USER_INFO },
    error::StakingAppError,
    state::{UserInfo},
    transfer_helper::{sol_transfer_from_user, sol_transfer_from_pda}
};
#[derive(Accounts)]
pub struct Stake<'info> {
    /// CHECK:
    #[account(
        init_if_needed,
        payer = payer,
        seeds = [STAKING_VAULT],
        bump,
        space = 0,
        owner = system_program::ID
    )]
    pub staking_vault: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        seeds = [USER_INFO, user.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<UserInfo>(),
    )]
    pub user_info: Box<Account<'info, UserInfo>>,

    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> Stake<'info>{
    pub fn process(ctx: Context<Stake>, amount: u64, is_stake: bool) -> Result<()> {
        let user_info = &mut ctx.accounts.user_info;

        let current_time: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
        let pass_time = if user_info.last_update_time == 0 {
            //just initialized
            0
        } else {
           current_time
                .checked_sub(user_info.last_update_time)
                .unwrap_or(0)
        };

        // user_info.amount += user_info.amount * STAKING_APR * pass_time / 100 / SECOND_PER_YEAR;
        let reward_u128 = (user_info.amount as u128)
            .checked_mul(STAKING_APR as u128)
            .ok_or(StakingAppError::ErrorMath)?
            .checked_mul(pass_time as u128)
            .ok_or(StakingAppError::ErrorMath)?
            .checked_div(100)
            .ok_or(StakingAppError::DivideByZero)?
            .checked_div(SECOND_PER_YEAR as u128)
            .ok_or(StakingAppError::DivideByZero)?;

        let reward = u64::try_from(reward_u128)
            .map_err(|_| StakingAppError::Overflow)?;
        user_info.amount = user_info.amount
            .checked_add(reward)
            .ok_or(StakingAppError::Overflow)?;
        user_info.last_update_time = current_time;

        if amount != 0 {
            if is_stake {
                sol_transfer_from_user(
                    &ctx.accounts.user,
                    ctx.accounts.staking_vault.to_account_info(),
                    &ctx.accounts.system_program,
                    amount,
                )?;

                user_info.amount = user_info.amount
                    .checked_add(amount)
                    .ok_or(StakingAppError::ErrorMath)?;
            } else {
                let pda_seeds: &[&[&[u8]]] = &[&[STAKING_VAULT, &[ctx.bumps.staking_vault]]];
                require!(
                    user_info.amount>=amount,
                    StakingAppError::Overflow
                );
                sol_transfer_from_pda(
                    ctx.accounts.staking_vault.to_account_info(),
                    ctx.accounts.user.to_account_info(),
                    &ctx.accounts.system_program,
                    pda_seeds,
                    amount,
                )?;

                user_info.amount = user_info.amount
                    .checked_sub(amount)
                    .ok_or(StakingAppError::Underflow)?;
            }
        }
        Ok(())
    }
}