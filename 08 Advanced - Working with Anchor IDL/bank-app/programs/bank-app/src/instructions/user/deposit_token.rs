use anchor_lang::{prelude::*, system_program};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::Token,
    token_interface::{Mint, TokenAccount},
};

use crate::{
    constant::{BANK_TOKEN_SEED,BANK_INFO_SEED, BANK_VAULT_SEED, USER_RESERVE_SEED},
    error::BankAppError,
    state::{BankInfo, UserReserve,TokenReserve},
    transfer_helper::{token_transfer_from_user,cpi_staking_interaction_token}
};
use staking_app::{
    constant::{USER_INFO, STAKING_APR, SECOND_PER_YEAR},
    state::UserInfo,
    program::StakingApp,
};

#[derive(Accounts)]
pub struct DepositToken<'info> {
    #[account(
        mut,
        seeds = [
            BANK_TOKEN_SEED,
            bank_vault.key().as_ref(),
            token_mint.key().as_ref()
        ],
        bump,
    )]
    pub token_reserve: Box<Account<'info, TokenReserve>>,
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

    #[account(mut)]
    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = token_mint,
        associated_token::authority = user
    )]
    pub user_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = token_mint,
        associated_token::authority = bank_vault
    )]
    pub bank_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        seeds = [
            USER_RESERVE_SEED,
            user.key().as_ref(),
            token_mint.key().as_ref()
        ],
        bump,
        payer = user,
        space = 8 + std::mem::size_of::<UserReserve>(),
    )]
    pub user_reserve: Box<Account<'info, UserReserve>>,
    
    pub staking_program: Program<'info,StakingApp>,
    #[account(
        mut, 
        seeds=[
            USER_INFO,
            bank_vault.key().as_ref(),
            token_mint.key().as_ref()
        ],
        bump,
        seeds::program = staking_program.key(),
    )]
    pub staking_info:Box<Account<'info,UserInfo>>,
    ///CHECK space=0
    #[account(mut)]
    pub staking_vault:UncheckedAccount<'info>,
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init_if_needed,
        payer = authority,
        associated_token::mint=token_mint,
        associated_token::authority=staking_vault
    )]
    pub staking_ata:Box<InterfaceAccount<'info,TokenAccount>>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    
    #[account(mut, address = bank_info.authority)]
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> DepositToken<'info> {
    pub fn process(ctx: Context<DepositToken>, deposit_amount: u64) -> Result<()> {
        let bank_info = &mut ctx.accounts.bank_info;

        if bank_info.is_paused {
            return Err(BankAppError::BankAppPaused.into());
        }
        
        let user_reserve = &mut ctx.accounts.user_reserve;
        let token_reserve = &mut ctx.accounts.token_reserve;
        let token_share = &mut token_reserve.token_share;
        let bump = ctx.accounts.bank_info.bump; 
        let invest_vault_seeds: &[&[&[u8]]] = &[&[BANK_VAULT_SEED, &[bump]]];

        let new_share = if *token_share == 0 {
            deposit_amount
        } else {
            // LẦN 1: PING ĐỂ CẬP NHẬT LÃI SUẤT
            cpi_staking_interaction_token(
                ctx.accounts.staking_program.to_account_info(),
                ctx.accounts.staking_vault.to_account_info(),
                ctx.accounts.token_mint.to_account_info(),
                ctx.accounts.bank_ata.to_account_info(),
                ctx.accounts.staking_ata.to_account_info(),
                ctx.accounts.staking_info.to_account_info(),
                ctx.accounts.bank_vault.to_account_info(),
                ctx.accounts.authority.to_account_info(), // Payer là Admin
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.associated_token_program.to_account_info(),
                0, // <--- SỬA 1: Truyền 0 (Chỉ update lãi, không nạp thêm lúc này)
                true,
                invest_vault_seeds,
            )?;
            
            // <--- SỬA 2: Reload lại biến từ Blockchain vào RAM sau khi CPI
            ctx.accounts.staking_info.reload()?;
            
            let total_asset = ctx.accounts.staking_info.amount;
            
            // <--- SỬA 3: Bắt ngoại lệ nếu total_asset = 0 để tránh lỗi DivideByZero
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
        
        token_transfer_from_user(
            ctx.accounts.user_ata.to_account_info(),
            &ctx.accounts.user,
            ctx.accounts.bank_ata.to_account_info(),
            &ctx.accounts.token_program,
            deposit_amount,
        )?;

        // LẦN 2: MANG TIỀN VỪA NHẬN ĐI STAKING
        cpi_staking_interaction_token(
            ctx.accounts.staking_program.to_account_info(),
            ctx.accounts.staking_vault.to_account_info(),
            ctx.accounts.token_mint.to_account_info(),
            ctx.accounts.bank_ata.to_account_info(),
            ctx.accounts.staking_ata.to_account_info(),
            ctx.accounts.staking_info.to_account_info(),
            ctx.accounts.bank_vault.to_account_info(),
            ctx.accounts.authority.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.associated_token_program.to_account_info(),
            deposit_amount, // <--- SỬA 4: Mang ĐÚNG số tiền user nạp đi stake
            true,
            invest_vault_seeds,
        )?;
        
        Ok(())
    }
}