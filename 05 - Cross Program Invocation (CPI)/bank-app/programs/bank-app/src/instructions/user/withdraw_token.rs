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
    transfer_helper::{token_transfer_from_pda,cpi_staking_interaction_token},
};

use staking_app::{
    constant::{USER_INFO, STAKING_VAULT,STAKING_APR, SECOND_PER_YEAR},
    state::UserInfo,
    program::StakingApp,
};

#[derive(Accounts)]
pub struct WithdrawToken<'info>{
    #[account(
        mut,
        seeds=[BANK_INFO_SEED],
        bump
    )]
    pub bank_info:Box<Account<'info,BankInfo>>,
    /// CHECK: Bank vault chỉ là một PDA rỗng (space = 0). Có thể không có mut vì không trực tiếp chứa token
    #[account(
        mut,
        seeds=[BANK_VAULT_SEED],
        bump,
        owner=system_program::ID,
    )]
    pub bank_vault:UncheckedAccount<'info>,
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
    #[account(mut)]
    pub token_mint:Box<InterfaceAccount<'info,Mint>>,
    #[account(
        mut,
        associated_token::mint=token_mint,
        associated_token::authority=user
    )]

   pub user_ata:Box<InterfaceAccount<'info,TokenAccount>>,
    #[account(
        mut,
        associated_token::mint=token_mint,
        associated_token::authority=bank_vault
    )]
    pub bank_ata:Box<InterfaceAccount<'info,TokenAccount>>,
    #[account(
        init_if_needed,
        seeds=[
            USER_RESERVE_SEED,
            user.key().as_ref(),
            token_mint.key().as_ref(),
        ],
        bump,
        payer=user,
        space=8+std::mem::size_of::<UserReserve>(),
    )]
    pub user_reserve:Box<Account<'info,UserReserve>>,
    pub staking_program: Program<'info, StakingApp>,
    #[account(
        mut,
        seeds = [
            USER_INFO, 
            bank_vault.key().as_ref(),
            token_mint.key().as_ref()
        ], 
        bump,
        seeds::program = staking_program.key()
    )]
    pub staking_info: Box<Account<'info, UserInfo>>,
    #[account(
        mut,
        associated_token::mint = token_mint,
        associated_token::authority=staking_vault,
    )]
    pub staking_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    ///CHECK: PDA Authority của bên Staking App. Chỉ dùng để ký CPI ủy quyền Token, không chứa dữ liệu.
    #[account(
        mut,
        seeds = [STAKING_VAULT], 
        bump,
        seeds::program = staking_program.key() 
    )]
    pub staking_vault: UncheckedAccount<'info>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    #[account(mut, address = bank_info.authority)]
    pub authority: Signer<'info>,
}

impl<'info> WithdrawToken<'info>{
    pub fn process(ctx:Context<WithdrawToken>, amount:u64)->Result<()>{
        if ctx.accounts.bank_info.is_paused {
            return Err(BankAppError::BankAppPaused.into());
        }
        let pda_seeds:&[&[&[u8]]]=&[&[BANK_VAULT_SEED,&[ctx.accounts.bank_info.bump]]];
        let user_reserve=&mut ctx.accounts.user_reserve;
        let token_reserve = &mut ctx.accounts.token_reserve;
        let staking_info = &mut ctx.accounts.staking_info;
        
        // LẦN 1: PING ĐỂ CHỐT LÃI SUẤT MỚI NHẤT
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
            0, // <--- SỬA 1: Truyền 0 để Ping lấy lãi, không truyền bank_amount
            true, 
            pda_seeds
        )?;

        // <--- SỬA 2: BẮT BUỘC RELOAD ĐỂ LẤY SỐ DƯ ĐÃ CỘNG LÃI
        ctx.accounts.staking_info.reload()?;
        
        let total_asset = ctx.accounts.staking_info.amount;
        
        // Kiểm tra số dư Share của User có đủ để rút không
        let left_side = (user_reserve.token_share as u128)
            .checked_mul(total_asset as u128)
            .ok_or(BankAppError::Overflow)?;

        let right_side = (amount as u128)
            .checked_mul(token_reserve.token_share as u128)
            .ok_or(BankAppError::Overflow)?;

        require!(left_side >= right_side, BankAppError::Overflow);

        // LẦN 2: BÁO STAKING APP XÌ TIỀN RA TRẢ BANK
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
            amount, // <--- Xin rút đúng số tiền user yêu cầu
            false,  // <--- is_stake = false (Rút tiền)
            pda_seeds
        )?;

        // Bank trả tiền về ví User
        token_transfer_from_pda(
            ctx.accounts.bank_ata.to_account_info(),
            ctx.accounts.bank_vault.to_account_info(),
            ctx.accounts.user_ata.to_account_info(),
            &ctx.accounts.token_program,
            pda_seeds,
            amount,
        )?;

        // Tính toán số Share cần trừ đi
        // <--- SỬA 3: Thêm bọc an toàn nếu total_asset = 0
        let new_share_u128 = if total_asset == 0 {
            0
        } else {
            (token_reserve.token_share as u128)
                .checked_mul(amount as u128)
                .ok_or(BankAppError::ErrorMath)? 
                .checked_div(total_asset as u128)
                .ok_or(BankAppError::DivideByZero)?
        };
            
        let new_share = u64::try_from(new_share_u128)
            .map_err(|_| BankAppError::ErrorMath)?;
            
        user_reserve.token_share = user_reserve.token_share
            .checked_sub(new_share)
            .ok_or(BankAppError::Underflow)?;
        token_reserve.token_share = token_reserve.token_share
            .checked_sub(new_share)
            .ok_or(BankAppError::Underflow)?;
            
        Ok(())
    }
}
   