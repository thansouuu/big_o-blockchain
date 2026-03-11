use anchor_lang::prelude::*;

#[error_code]
pub enum StakingAppError {
    #[msg("Không đủ số dư thực hiện giao dịch!")]
    Overflow,
}