use anchor_lang::{prelude::*, system_program};
use anchor_spl::{
    token_interface::{Mint},
};
use crate::{
    constant::{BANK_TOKEN_SEED,BANK_INFO_SEED, BANK_VAULT_SEED},
    state::{BankInfo,TokenReserve},
};

#[derive(Accounts)]
pub struct AddToken<'info> {
    #[account(
        mut,
        address = bank_info.authority,
    )]
    pub authority: Signer<'info>, // Admin

    #[account(
        seeds = [BANK_INFO_SEED],
        bump
    )]
    pub bank_info: Box<Account<'info, BankInfo>>,

    ///CHECK:
    #[account(seeds = [BANK_VAULT_SEED], bump)]
    pub bank_vault: UncheckedAccount<'info>,

    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init,
        payer = authority,
        seeds = [BANK_TOKEN_SEED, bank_vault.key().as_ref(), token_mint.key().as_ref()],
        bump,
        space = 8 + std::mem::size_of::<TokenReserve>()
    )]
    pub token_reserve: Box<Account<'info, TokenReserve>>,

    pub system_program: Program<'info, System>,
}
impl<'info> AddToken<'info> {
    pub fn process(ctx: Context<AddToken>) -> Result<()> {
        let token_reserve = &mut ctx.accounts.token_reserve;
        token_reserve.token_mint = ctx.accounts.token_mint.key();
        token_reserve.token_share = 0;
        Ok(())
    }
}