use anchor_lang::prelude::*;

#[account]
#[derive(Default)]
pub struct BankInfo {
    pub authority: Pubkey,
    pub is_paused: bool,
    pub bump: u8,
}

#[account]
#[derive(Default)]
pub struct UserReserve {
    pub token_share: u64,
}

#[account]
#[derive(Default)]
pub struct TokenReserve{
    pub token_mint:Pubkey,
    pub token_share:u64,
}

#[account]
#[derive(Default)]
pub struct SolReserve{
    pub token_share:u64,
}