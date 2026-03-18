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
}

impl<'info> WithdrawToken<'info>{
    pub fn process(ctx:Context<WithdrawToken>,amount:u64)->Result<()>{
        if ctx.accounts.bank_info.is_paused {
            return Err(BankAppError::BankAppPaused.into());
        }
        let pda_seeds:&[&[&[u8]]]=&[&[BANK_VAULT_SEED,&[ctx.accounts.bank_info.bump]]];
        let user_reserve=&mut ctx.accounts.user_reserve;
        
        let token_reserve = &mut ctx.accounts.token_reserve;
        let staking_info = &mut ctx.accounts.staking_info;
        let current_time: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
        let pass_time = if staking_info.last_update_time == 0 {
            0
        } else {
            current_time - staking_info.last_update_time
        };
        
        let lai = staking_info.amount * STAKING_APR * pass_time / 100 / SECOND_PER_YEAR;
        let total_asset= ctx.accounts.bank_ata.amount+staking_info.amount+lai;
        
        require!(
            (user_reserve.deposited_amount as u128) * (total_asset as u128) >= (amount as u128) * (token_reserve.token_share as u128),
            BankAppError::Overflow
        );


        let delta = if amount>ctx.accounts.bank_ata.amount {
            amount-ctx.accounts.bank_ata.amount
        }
        else {
            0
        };

        cpi_staking_interaction_token(
            ctx.accounts.staking_program.to_account_info(),
            ctx.accounts.staking_vault.to_account_info(),
            ctx.accounts.token_mint.to_account_info(),
            ctx.accounts.bank_ata.to_account_info(),
            ctx.accounts.staking_ata.to_account_info(),
            
            ctx.accounts.staking_info.to_account_info(),
            ctx.accounts.bank_vault.to_account_info(),
            ctx.accounts.user.to_account_info(), // Payer
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.associated_token_program.to_account_info(),
            delta,
            false, // Rút về
            pda_seeds
        )?;

        token_transfer_from_pda(
            ctx.accounts.bank_ata.to_account_info(),
            ctx.accounts.bank_vault.to_account_info(),
            ctx.accounts.user_ata.to_account_info(),
            &ctx.accounts.token_program,
            pda_seeds,
            amount,
        )?;
        let new_share=token_reserve.token_share*amount/total_asset;
        user_reserve.deposited_amount-=new_share;
        token_reserve.token_share-=new_share;
        Ok(())
    }
}