use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("{0}")]
    Overflow(#[from] OverflowError),
    #[error("invalid immutable configuration: {0}")]
    InvalidConfig(String),
    #[error("funds must be exactly one coin of the configured denom and amount")]
    InvalidFunds,
    #[error("a market creation reply is already pending")]
    CreationPending,
    #[error("unknown reply id {0}")]
    UnknownReplyId(u64),
    #[error("instantiate submessage failed: {0}")]
    InstantiateFailed(String),
    #[error("child verification failed: {0}")]
    ChildVerification(String),
}
