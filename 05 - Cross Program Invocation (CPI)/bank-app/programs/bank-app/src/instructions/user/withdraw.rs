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
    pub system_program: Program<'info, System>,
}

impl<'info> Withdraw<'info> {
    pub fn process(ctx: Context<Withdraw>, withdraw_amount: u64) -> Result<()> {
        if ctx.accounts.bank_info.is_paused {
            return Err(BankAppError::BankAppPaused.into());
        }

        let pda_seeds: &[&[&[u8]]] = &[&[BANK_VAULT_SEED, &[ctx.accounts.bank_info.bump]]];
        let user_reserve=&mut ctx.accounts.user_reserve;
        
        
        let staking_info = &mut ctx.accounts.staking_info;
        let token_share = &mut ctx.accounts.sol_reserve.token_share;
        let current_time: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
        let pass_time = if staking_info.last_update_time == 0 {
            0
        } else {
            current_time - staking_info.last_update_time
        };

        let lai=staking_info.amount * STAKING_APR * pass_time / 100 / SECOND_PER_YEAR;
        
        let rent = Rent::get()?;
        let amount_bank=ctx.accounts.bank_vault.lamports()-rent.minimum_balance(0);
        
        let total_asset=staking_info.amount+amount_bank+lai;
        require!(
            user_reserve.deposited_amount*total_asset/(*token_share)>=withdraw_amount,
            BankAppError::Overflow
        );
        let delta = if withdraw_amount>amount_bank {
            withdraw_amount-amount_bank
        }
        else {
            0
        };
        cpi_staking_interaction(
            ctx.accounts.staking_program.to_account_info(), // Nhớ thêm vào struct Withdraw
            ctx.accounts.staking_vault.to_account_info(),   // Nhớ thêm vào struct Withdraw
            ctx.accounts.staking_info.to_account_info(),       // Account info bên staking
            ctx.accounts.bank_vault.to_account_info(),
            ctx.accounts.user.to_account_info(),            // Regular user trả phí gas
            ctx.accounts.system_program.to_account_info(),
            delta,
            false, // is_stake = false (RÚT TIỀN VỀ)
            pda_seeds
        )?;
        sol_transfer_from_pda(
            ctx.accounts.bank_vault.to_account_info(),
            ctx.accounts.user.to_account_info(),
            &ctx.accounts.system_program,
            pda_seeds,
            withdraw_amount
        )?;
        let new_share=(*token_share)*withdraw_amount/total_asset;
        user_reserve.deposited_amount-=new_share;
        (*token_share)-=new_share;
        Ok(())
    }
}
