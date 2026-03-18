use anchor_lang::{prelude::*, system_program};
pub mod constant;
pub mod error;
pub mod instructions;
pub mod state;
pub mod transfer_helper;
use instructions::*;
declare_id!("6dHk4wFxpKGP7orzUqCUS6QA5KX12vpJzM6NbGB7MNRD");



#[program]
pub mod staking_app {
    use super::*;
    pub fn staking_sol(ctx:Context<Stake>, amount: u64, is_stake: bool) -> Result<()>{
        return Stake::process(ctx, amount, is_stake);
    }
    pub fn staking_token(ctx:Context<StakeToken>, amount: u64, is_stake: bool) -> Result<()>{
        return StakeToken::process(ctx, amount, is_stake);
    }

}




