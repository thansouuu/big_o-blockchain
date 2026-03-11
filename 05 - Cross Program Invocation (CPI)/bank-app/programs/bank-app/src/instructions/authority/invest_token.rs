use anchor_lang::{prelude::*, system_program};
use anchor_spl::{
    token::Token,
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount},
};
use crate::{
    constant::{BANK_INFO_SEED, BANK_VAULT_SEED},
    error::BankAppError,
    state::BankInfo,
};
use staking_app::{cpi, program::StakingApp};



#[derive(Accounts)]
pub struct InvestToken<'info> {
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

    ///CHECK:
    #[account(mut)]
    pub staking_vault: UncheckedAccount<'info>,
    ///CHECK:
    #[account(mut)]
    pub staking_info: UncheckedAccount<'info>,
    pub staking_program: Program<'info, StakingApp>,
    pub token_program: Program<'info, Token>,
    pub token_mint:Box<InterfaceAccount<'info,Mint>>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    #[account(
        init_if_needed,
        payer = authority,
        associated_token::mint=token_mint,
        associated_token::authority=bank_vault
    )]
    pub bank_ata:Box<InterfaceAccount<'info,TokenAccount>>,
    
    #[account(
        init_if_needed,
        payer = authority,
        associated_token::mint=token_mint,
        associated_token::authority=staking_vault
    )]
    pub staking_ata:Box<InterfaceAccount<'info,TokenAccount>>,
    
    #[account(mut, address = bank_info.authority)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    
}   

impl<'info> InvestToken<'info> {
    pub fn process(ctx: Context<InvestToken>, amount: u64, is_stake: bool) -> Result<()> {
        if ctx.accounts.bank_info.is_paused {
            return Err(BankAppError::BankAppPaused.into());
        }

        let invest_vault_seeds: &[&[&[u8]]] = &[&[BANK_VAULT_SEED, &[ctx.accounts.bank_info.bump]]];

        cpi::staking_token(
            CpiContext::new_with_signer(
                ctx.accounts.staking_program.to_account_info(),
                cpi::accounts::StakeToken{
                    staking_vault:ctx.accounts.staking_vault.to_account_info(),
                    token_mint:ctx.accounts.token_mint.to_account_info(),
                    user_ata:ctx.accounts.bank_ata.to_account_info(),
                    staking_ata:ctx.accounts.staking_ata.to_account_info(),
                    user_info:ctx.accounts.staking_info.to_account_info(),
                    user:ctx.accounts.bank_vault.to_account_info(),
                    payer:ctx.accounts.authority.to_account_info(),
                    token_program:ctx.accounts.token_program.to_account_info(),
                    system_program:ctx.accounts.system_program.to_account_info(),
                    associated_token_program: ctx.accounts.associated_token_program.to_account_info(),
                },
                invest_vault_seeds,
            ),
            amount,
            is_stake,
        )?;

        Ok(())
    }
}
