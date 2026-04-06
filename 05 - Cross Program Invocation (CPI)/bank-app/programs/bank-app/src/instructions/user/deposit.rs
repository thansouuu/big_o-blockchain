use anchor_lang::{prelude::*, system_program};

use crate::{
    constant::{BANK_TOKEN_SEED,BANK_INFO_SEED, BANK_VAULT_SEED, USER_RESERVE_SEED},
    error::BankAppError,
    state::{BankInfo, UserReserve,SolReserve},
    transfer_helper::{sol_transfer_from_user,cpi_staking_interaction}
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
    ///CHECK:
    #[account(mut)]
    pub staking_vault: UncheckedAccount<'info>,
    pub staking_program: Program<'info,StakingApp>,
    #[account(
        mut, 
        seeds = [USER_INFO, bank_vault.key().as_ref()],
        bump,
        seeds::program = staking_program.key(),
    )]
    pub staking_info: Box<Account<'info, UserInfo>>,

    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut, address = bank_info.authority)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> Deposit<'info> {
    pub fn process(ctx: Context<Deposit>, deposit_amount: u64) -> Result<()> {
        if ctx.accounts.bank_info.is_paused {
            return Err(BankAppError::BankAppPaused.into());
        }

        let user_reserve = &mut ctx.accounts.user_reserve;
        let token_share = &mut ctx.accounts.sol_reserve.token_share;
        let pda_seeds: &[&[&[u8]]] = &[&[BANK_VAULT_SEED, &[ctx.accounts.bank_info.bump]]];

        let new_share = if *token_share == 0 {
            deposit_amount
        } else {
            // LẦN 1: PING ĐỂ ĐỒNG BỘ LÃI SUẤT
            cpi_staking_interaction(
                ctx.accounts.staking_program.to_account_info(),
                ctx.accounts.staking_vault.to_account_info(),  
                ctx.accounts.staking_info.to_account_info(),      
                ctx.accounts.bank_vault.to_account_info(),
                ctx.accounts.authority.to_account_info(),     
                ctx.accounts.system_program.to_account_info(),
                0, // <--- SỬA: Truyền 0 để chỉ đồng bộ lãi, không rút lõi quỹ SOL
                true,
                pda_seeds
            )?;

            // SỬA: Ép tải lại dữ liệu vào RAM
            ctx.accounts.staking_info.reload()?;
            
            let total_asset = ctx.accounts.staking_info.amount;
            
            // SỬA: Bắt ngoại lệ nếu két trống
            if total_asset == 0 {
                deposit_amount
            } else {
                let result_u128 = (*token_share as u128)
                    .checked_mul(deposit_amount as u128)
                    .ok_or(BankAppError::ErrorMath)? 
                    .checked_div(total_asset as u128)
                    .ok_or(BankAppError::DivideByZero)?;
                u64::try_from(result_u128).map_err(|_| BankAppError::ErrorMath)?
            }
        };

        *token_share = token_share
            .checked_add(new_share)
            .ok_or(BankAppError::ErrorMath)?;
        user_reserve.token_share = user_reserve.token_share
            .checked_add(new_share)
            .ok_or(BankAppError::ErrorMath)?;

        sol_transfer_from_user(
            &ctx.accounts.user,
            ctx.accounts.bank_vault.to_account_info(),
            &ctx.accounts.system_program,
            deposit_amount,
        )?;

        // LẦN 2: MANG TIỀN VỪA THU ĐI AUTO-STAKE
        cpi_staking_interaction(
            ctx.accounts.staking_program.to_account_info(),
            ctx.accounts.staking_vault.to_account_info(),  
            ctx.accounts.staking_info.to_account_info(),      
            ctx.accounts.bank_vault.to_account_info(),
            ctx.accounts.authority.to_account_info(),     
            ctx.accounts.system_program.to_account_info(),
            deposit_amount, // <--- Truyền đúng tiền gửi vào
            true,
            pda_seeds
        )?;

        Ok(())
    }
}