use anchor_lang::prelude::*;

#[constant]
pub const STAKING_APR: u64 = 5;
pub const SECOND_PER_YEAR: u64 = 31_536_000;
pub const STAKING_VAULT: &[u8] = b"STAKING_VAULT";
pub const USER_INFO: &[u8] = b"USER_INFO";
