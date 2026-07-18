//! Message surface for `cw-reality`.
//!
//! Variants follow Reality.eth's entry-point set with two CosmWasm-specific
//! additions: pull-payment `Withdraw` (the cw20-receive hook), and
//! `current_bond_seen` front-run guards on every bond-affecting variant
//! (FM-7).

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, Uint128};
use cw20::Cw20ReceiveMsg;

use crate::filter::AnswerSchemaFilter;
use crate::state::{AnswerType, Question, State};

#[cw_serde]
pub struct InstantiateMsg {
    /// Optional migration admin. `None` ships a frozen instance.
    pub admin: Option<String>,
    /// Mandatory floor for `AskQuestion.initial_bond` (FM-1).
    pub min_initial_bond_floor: Uint128,
    /// Mandatory floor for `AskQuestion.answer_timeout_secs` (FM-8).
    /// Reality.eth has no contract-level floor; we add one because
    /// SuDAO and Gnosis Guild were drained partly because integrators
    /// chose 12h / 1h windows.
    pub min_answer_timeout_secs: u32,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Pose a new question. `initial_bond` is the floor on the very first
    /// answer's bond (Reality.eth's `min_bond`). Bonds are pinned to a single
    /// denom captured here.
    AskQuestion {
        text: String,
        answer_type: AnswerType,
        bond_denom: String,
        initial_bond: Uint128,
        answer_timeout_secs: u32,
        arbitrator: Option<String>,
        arbitration_timeout_secs: Option<u32>,
        answer_schema: Option<AnswerSchemaFilter>,
        opening_ts: Option<u64>,
        nonce: u64,
    },

    /// Top up the bounty paid to the eventual winner. Permitted until
    /// finalization.
    FundBounty { question_id: Binary },

    /// Submit an answer with a bond. `current_bond_seen` is the front-run
    /// guard — reject if the on-chain bond at execution time exceeds it.
    SubmitAnswer {
        question_id: Binary,
        answer: Binary,
        current_bond_seen: Option<Uint128>,
    },

    /// Submit a competing answer (≥ 2× current bond). Same shape as
    /// `SubmitAnswer`; kept as a distinct variant so external indexers can
    /// distinguish "first answer in this round" from "counter-answer."
    DisputeAnswer {
        question_id: Binary,
        new_answer: Binary,
        current_bond_seen: Option<Uint128>,
    },

    /// Configured-arbitrator-only. Requires the question to be OpenAnswered
    /// (and thus to have at least one answer), applies the optional
    /// `current_bond_seen` front-run guard, freezes it, and starts the
    /// arbitration deadline.
    RequestArbitration {
        question_id: Binary,
        current_bond_seen: Option<Uint128>,
    },

    /// Pending-arbitration-only. The configured arbitrator may cancel at any
    /// time; anyone may cancel at or after the arbitration deadline. Unfreezes
    /// the question and resets `finalize_ts` to `now + timeout` (re-extend,
    /// not restore).
    CancelArbitration { question_id: Binary },

    /// Arbitrator-only and pending-arbitration-only. Finalizes with any
    /// `winning_answer` bytes chosen by the arbitrator and records the
    /// validated `payee` as that answer's zero-bond history entry. No
    /// submitted-history membership or answer-schema check is performed.
    SubmitArbitration {
        question_id: Binary,
        winning_answer: Binary,
        payee: String,
    },

    /// Drive the resumable claim walk. Anyone may call once finalized.
    /// `history_entries` is the caller-supplied history chain, newest-first;
    /// the contract verifies each entry against the stored chain tip
    /// (Reality.eth `claimWinnings` pattern). The caller may pass any prefix
    /// of the remaining chain; partial walks persist `cursor_hash` via the
    /// `CLAIMS` map and resume on the next call.
    Claim {
        question_id: Binary,
        history_entries: Vec<HistoryEntry>,
    },

    /// Drain the caller's pull-payment balance for the named denom.
    Withdraw { denom: String },

    /// Cw20 hook — wraps a state-mutating variant whose bond is a cw20.
    Receive(Cw20ReceiveMsg),
}

/// Payload variants embedded inside `Cw20ReceiveMsg.msg`.
#[cw_serde]
pub enum ReceiveAction {
    AskQuestion {
        text: String,
        answer_type: AnswerType,
        initial_bond: Uint128,
        answer_timeout_secs: u32,
        arbitrator: Option<String>,
        arbitration_timeout_secs: Option<u32>,
        answer_schema: Option<AnswerSchemaFilter>,
        opening_ts: Option<u64>,
        nonce: u64,
    },
    FundBounty {
        question_id: Binary,
    },
    SubmitAnswer {
        question_id: Binary,
        answer: Binary,
        current_bond_seen: Option<Uint128>,
    },
    DisputeAnswer {
        question_id: Binary,
        new_answer: Binary,
        current_bond_seen: Option<Uint128>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Contract instantiation parameters.
    #[returns(crate::state::Config)]
    Config {},

    /// Full question state.
    #[returns(QuestionResponse)]
    Question { question_id: Binary },

    /// The finalized winning answer, or error if not finalized.
    #[returns(FinalAnswerResponse)]
    FinalAnswer { question_id: Binary },

    /// Reader-side trust knob (Reality.eth's `getFinalAnswerIfMatches`).
    /// Reverts unless every supplied constraint passes — so a downstream
    /// consumer can require the question used at least the bond / timeout /
    /// denom it considers safe.
    #[returns(FinalAnswerResponse)]
    FinalAnswerIfMatches {
        question_id: Binary,
        min_bond: Option<Uint128>,
        min_timeout_secs: Option<u32>,
        required_arbitrator: Option<String>,
        required_denom: Option<String>,
    },

    /// Paginated list of questions, optionally filtered by current state.
    #[returns(QuestionsListResponse)]
    List {
        start_after: Option<Binary>,
        limit: Option<u32>,
        status: Option<State>,
    },

    /// A user's pull-payment balance for the given denom.
    #[returns(BalanceResponse)]
    Balance { address: String, denom: String },
}

#[cw_serde]
pub struct QuestionResponse {
    pub question_id: Binary,
    pub question: Question,
    pub state: State,
}

#[cw_serde]
pub struct FinalAnswerResponse {
    pub question_id: Binary,
    pub final_answer: Binary,
    pub final_bond: Uint128,
}

#[cw_serde]
pub struct QuestionsListResponse {
    pub questions: Vec<QuestionResponse>,
}

#[cw_serde]
pub struct BalanceResponse {
    pub address: String,
    pub denom: String,
    pub amount: Uint128,
}

#[cw_serde]
pub struct MigrateMsg {}

/// One step of the answer-history chain, supplied by the claimer.
///
/// `prev_hash` is the chain hash BEFORE this entry was added (i.e. the hash
/// from which `prev_hash || answer || denom || bond || answerer ||
/// is_commitment` hashes back to the cursor). At step 0 (the first answer
/// ever submitted) `prev_hash` is the all-zero hash.
#[cw_serde]
pub struct HistoryEntry {
    pub prev_hash: Binary,
    pub answer: Binary,
    pub bond_amount: Uint128,
    pub answerer: String,
    pub is_commitment: bool,
}
