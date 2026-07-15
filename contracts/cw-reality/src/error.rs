use cosmwasm_std::{OverflowError, StdError, Uint128};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("unauthorized")]
    Unauthorized {},

    // ---- Instantiation / configuration ----
    #[error("min_initial_bond_floor must be greater than zero")]
    ZeroMinInitialBondFloor {},

    #[error("min_answer_timeout_secs must be at least one hour (3600s)")]
    MinAnswerTimeoutTooLow {},

    // ---- Question lifecycle ----
    #[error("question {id} does not exist")]
    QuestionNotFound { id: String },

    #[error("question {id} already exists (nonce collision — increment nonce)")]
    QuestionAlreadyExists { id: String },

    #[error("question must be in state {expected:?}, currently {actual:?}")]
    InvalidState { expected: String, actual: String },

    #[error("initial_bond {provided} is below the platform floor {floor}")]
    InitialBondBelowFloor { provided: Uint128, floor: Uint128 },

    #[error("answer_timeout_secs {provided} is below the platform floor {floor}")]
    AnswerTimeoutBelowFloor { provided: u32, floor: u32 },

    #[error("answer_timeout_secs exceeds the protocol maximum (365 days)")]
    AnswerTimeoutTooHigh {},

    #[error("question requires bond denom {expected}, got {actual}")]
    BondDenomMismatch { expected: String, actual: String },

    #[error("bond must include exactly one denom; got {count}")]
    InvalidBondFunds { count: usize },

    #[error("bond {provided} must be at least double the previous bond {previous}")]
    BondMustDouble {
        provided: Uint128,
        previous: Uint128,
    },

    #[error("bond {provided} must meet the question minimum {minimum}")]
    BondBelowMinimum { provided: Uint128, minimum: Uint128 },

    #[error("front-run guard tripped: current bond {actual} exceeds expected ceiling {expected}")]
    BondExceedsExpected { actual: Uint128, expected: Uint128 },

    #[error("dispute round cap of {cap} reached")]
    RoundCapReached { cap: u32 },

    // ---- Arbitration ----
    #[error("question has no arbitrator configured")]
    NoArbitrator {},

    #[error("only the configured arbitrator may call this entry point")]
    NotArbitrator {},

    #[error("arbitration requires at least one prior answer")]
    ArbitrationNoAnswer {},

    /// Legacy, currently unused variant retained for source compatibility.
    /// `SubmitArbitration` does not reject merely because the deadline passed.
    #[error("arbitration deadline {deadline} has passed (now {now})")]
    ArbitrationDeadlinePassed { deadline: u64, now: u64 },

    /// Legacy, currently unused variant retained for source compatibility.
    /// `SubmitArbitration` performs no submitted-history membership check.
    #[error("arbitrator must pick from a previously submitted answer")]
    ArbitrationAnswerNotInHistory {},

    // ---- Finalization / claim ----
    #[error("question is not yet finalized")]
    NotFinalized {},

    #[error("history hash verification failed at step {step}")]
    HistoryHashMismatch { step: usize },

    #[error("claim history is exhausted")]
    NothingToClaim {},

    #[error("withdraw failed: caller has no balance")]
    NothingToWithdraw {},

    // ---- Filter / cw-filter ----
    #[error("answer rejected by cw-filter at index {index}: {reason}")]
    AnswerFilterFail { index: usize, reason: String },

    #[error("cw-filter returned fatal at index {index}: {reason}")]
    AnswerFilterFatal { index: usize, reason: String },

    // ---- Reader trust knob ----
    #[error("question parameters do not satisfy the caller's minimum guarantees")]
    GuaranteesNotMet {},
}
