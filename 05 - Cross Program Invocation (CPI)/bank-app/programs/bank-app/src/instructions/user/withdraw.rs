use anchor_lang::{prelude::*, system_program};
use staking_app::transfer_helper::sol_transfer_from_pda;
use crate::{
    constant::{BANK_TOKEN_SEED,BANK_INFO_SEED, BANK_VAULT_SEED, USER_RESERVE_SEED},
    error::BankAppError,
    state::{BankInfo, UserReserve,SolReserve},
    transfer_helper::cpi_staking_interaction,
};

use staking_app::{
    constant::{USER_INFO,STAKING_APR,SECOND_PER_YEAR},
    state::{UserInfo},
};
use staking_app::{cpi, program::StakingApp};
#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(
        mut,
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
        mut,
        seeds = [USER_INFO, bank_vault.key().as_ref()], 
        bump,
        seeds::program = staking_program.key()
    )]
    pub staking_info: Box<Account<'info, UserInfo>>,

    #[account(
        mut, 
        seeds = [USER_RESERVE_SEED, user.key().as_ref()],
        bump,
    )]
    pub user_reserve: Box<Account<'info, UserReserve>>,
    ///CHECK:
    #[account(mut)]
    pub staking_vault: UncheckedAccount<'info>,
    pub staking_program: Program<'info, StakingApp>,

    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut, address = bank_info.authority)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> Withdraw<'info> {
    pub fn process(ctx: Context<Withdraw>, withdraw_amount: u64) -> Result<()> {
        if ctx.accounts.bank_info.is_paused {
            return Err(BankAppError::BankAppPaused.into());
        }
        let user_reserve=&mut ctx.accounts.user_reserve;
        let token_share = &mut ctx.accounts.sol_reserve.token_share;

        let pda_seeds: &[&[&[u8]]] = &[&[BANK_VAULT_SEED, &[ctx.accounts.bank_info.bump]]];
        let rent = Rent::get()?;
        let bank_amount = ctx.accounts.bank_vault.lamports()
            .checked_sub(rent.minimum_balance(0))
            .unwrap_or(0);

        cpi_staking_interaction(
                ctx.accounts.staking_program.to_account_info(),
                ctx.accounts.staking_vault.to_account_info(),  
                ctx.accounts.staking_info.to_account_info(),      
                ctx.accounts.bank_vault.to_account_info(),
                ctx.accounts.authority.to_account_info(),     
                ctx.accounts.system_program.to_account_info(),
                bank_amount,
                true,
                pda_seeds
        )?;    
        //task 
        //sua lai config cua test
        //nap tien vao staking-app de tra tien lai 
        //test
        let total_asset=ctx.accounts.staking_info.amount;

        let left_side = (user_reserve.token_share as u128)
            .checked_mul(total_asset as u128)
            .ok_or(BankAppError::Overflow)?;

        let right_side = (withdraw_amount as u128)
            .checked_mul(*token_share as u128)
            .ok_or(BankAppError::Overflow)?;

        require!(left_side >= right_side, BankAppError::Overflow);
         cpi_staking_interaction(
                ctx.accounts.staking_program.to_account_info(),
                ctx.accounts.staking_vault.to_account_info(),  
                ctx.accounts.staking_info.to_account_info(),      
                ctx.accounts.bank_vault.to_account_info(),
                ctx.accounts.authority.to_account_info(),     
                ctx.accounts.system_program.to_account_info(),
                withdraw_amount,
                false,
                pda_seeds
        )?; 
        sol_transfer_from_pda(
            ctx.accounts.bank_vault.to_account_info(),
            ctx.accounts.user.to_account_info(),
            &ctx.accounts.system_program,
            pda_seeds,
            withdraw_amount
        )?;
        let new_share_u128 = (*token_share as u128)
            .checked_mul(withdraw_amount as u128)
            .ok_or(BankAppError::ErrorMath)? 
            .checked_div(total_asset as u128)
            .ok_or(BankAppError::DivideByZero)?;
        let new_share = u64::try_from(new_share_u128)
            .map_err(|_| BankAppError::ErrorMath)?;
        // let new_share=(*token_share)*withdraw_amount/total_asset;
        user_reserve.token_share = user_reserve.token_share
            .checked_sub(new_share)
            .ok_or(BankAppError::Underflow)?;
        *token_share = (*token_share)
            .checked_sub(new_share)
            .ok_or(BankAppError::Underflow)?;
        Ok(())
    }
}
