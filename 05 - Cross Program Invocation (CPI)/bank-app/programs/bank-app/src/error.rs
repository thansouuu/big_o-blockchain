use anchor_lang::prelude::*;

#[error_code]
pub enum BankAppError {
    #[msg("The bank app is currently paused.")]
    BankAppPaused,
    #[msg("Không đủ số dư thực hiện giao dịch!")]
    Overflow,
    #[msg("Toán học: Xảy ra tràn số (ErrorMath)!")]
    ErrorMath,
    #[msg("Toán học: Lỗi chia cho 0!")]
    DivideByZero,
    #[msg("Toán học: Xảy ra tràn số âm (Underflow)!")]
    Underflow,
}
