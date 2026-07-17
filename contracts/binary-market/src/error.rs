use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("arithmetic overflow")]
    Overflow(#[from] OverflowError),
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("expected exactly one {expected} coin of {denom}")]
    InvalidFunds {
        expected: cosmwasm_std::Uint128,
        denom: String,
    },
    #[error("no funds accepted")]
    UnexpectedFunds,
    #[error("unauthorized")]
    Unauthorized,
    #[error("market is not activated")]
    NotActivated,
    #[error("market is closed")]
    MarketClosed,
    #[error("deadline expired")]
    DeadlineExpired,
    #[error("market already resolved")]
    AlreadyResolved,
    #[error("market is not resolved")]
    NotResolved,
    #[error("amount must be at least {minimum}")]
    AmountBelowMinimum { minimum: cosmwasm_std::Uint128 },
    #[error("collateral cap exceeded")]
    CollateralCapExceeded,
    #[error("insufficient YES/NO position balance")]
    InsufficientPosition,
    #[error("redemption must burn at least one position unit")]
    EmptyRedemption,
    #[error("trade slippage limit exceeded")]
    SlippageExceeded,
    #[error("FPMM arithmetic rejected the trade: {0}")]
    Math(String),
    #[error("pre-resolution accounting invariant failed: {0}")]
    InvariantViolation(String),
    #[error("no challenge is pending")]
    NoPendingChallenge,
    #[error("governance verdict deadline reached")]
    ArbitrationDeadlineReached,
    #[error("stalled challenge cannot be finalized before deadline")]
    ArbitrationDeadlineNotReached,
    #[error("oracle arbitration state verification failed: {0}")]
    ArbitrationMismatch(String),
    #[error("arbitration deadline arithmetic overflow")]
    ArbitrationDeadlineOverflow,
    #[error("oracle round-count arithmetic overflow")]
    ArbitrationRoundOverflow,
    #[error("oracle arbitration submessage failed: {0}")]
    ArbitrationSubmessage(String),
    #[error("verdict answer must not be empty")]
    InvalidVerdictAnswer,
    #[error("unknown reply id {0}")]
    UnknownReplyId(u64),
    #[error("reply state does not match reply id")]
    ReplyStateMismatch,
    #[error("oracle activation verification failed: {0}")]
    ActivationMismatch(String),
    #[error("oracle resolution verification failed: {0}")]
    ResolutionMismatch(String),
    #[error("action is specified but implemented by a later issue")]
    NotImplemented,
}
