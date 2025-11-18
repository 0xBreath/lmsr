//! Error codes for the program.
//!
//! Custom error for Anchor programs start at 6000. i.e. here Unauthorized error would be 6000 and
//! InvalidProgramCount would be 6001.

use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Too many outcomes")]
    TooManyOutcomes,

    #[msg("Outcome is below two")]
    NotEnoughOutcomes,

    #[msg("Account Not Signer")]
    AccountNotSigner,

    #[msg("Account Not Writable")]
    AccountNotWritable,

    #[msg("Account Not Executable")]
    AccountNotExecutable,

    #[msg("Missing Remaining Account")]
    MissingRemainingAccount,

    #[msg("Invalid Token Program")]
    InvalidTokenProgram,

    #[msg("Math Overflow")]
    MathOverflow,

    #[msg("Invalid Account Owner")]
    InvalidAccountOwner,

    #[msg("Invalid outcome index")]
    InvalidOutcomeIndex,

    #[msg("Transfer failed")]
    TransferFailed,

    #[msg("Token mint failed")]
    TokenMintFailed,

    #[msg("Invalid mint count")]
    InvalidMintCount,

    #[msg("Invalid mint seed")]
    InvalidMintSeed,

    #[msg("Invalid label length")]
    InvalidLabelLength,

    #[msg("Deposit is zero")]
    DepositIsZero,

    #[msg("Burn is zero")]
    BurnIsZero,

    #[msg("Shares are zero")]
    SharesAreZero,

    #[msg("Insufficient funds")]
    InsufficientFunds,

    #[msg("Burn is more than supply")]
    BurnIsMoreThanSupply,

    #[msg("Insufficient vault funds")]
    InsufficientVaultFunds,

    #[msg("Vault transfer failed")]
    VaultTransferFailed,

    #[msg("Market expired")]
    MarketExpired,

    #[msg("Market not ready to resolve")]
    MarketNotReadyToResolve,

    #[msg("Market must last at least 1 second")]
    MarketTooQuick,

    #[msg("Reserve is zero")]
    ReserveIsZero,

    #[msg("Liquidity parameter is zero")]
    LiquidityParameterIsZero,

    #[msg("Supply is zero")]
    SupplyIsZero,

    #[msg("Market not resolved")]
    OutcomeHasZeroReserves,

    #[msg("No outcome has consensus")]
    NoOutcomeHasConsensus,

    #[msg("Market is already resolved")]
    MarketAlreadyResolved,

    #[msg("Market not resolved")]
    MarketNotResolved,

    #[msg("Outcome is not the winner")]
    OutcomeNotWinner,
}

/// Check a condition and return an error if it is not met.
///
/// # Arguments
/// * `condition` - The condition to check.
/// * `error` - The error to return if the condition is not met.
#[macro_export]
macro_rules! check_condition {
    ($condition:expr, $error:expr) => {
        if !$condition {
            return Err(error!(ErrorCode::$error));
        }
    };
}
