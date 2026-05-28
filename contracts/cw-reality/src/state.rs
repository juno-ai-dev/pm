//! Storage layout and state-machine types.
//!
//! Reality.eth derives state implicitly from four fields on `Question`. We
//! make the state explicit (`State` enum) and assert on entry to every
//! state-mutating handler — see `docs/reality-eth-lessons.md` §1.

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Uint128};
use cw_storage_plus::{Item, Map};

use crate::filter::AnswerSchemaFilter;
use crate::hash::HistoryHash;

/// Reality.eth's `UNRESOLVED_ANSWER` sentinel — `bytes32(-2)`. Reserved for
/// the explicit-decline path via `SubmitArbitration`. Match Reality's bit
/// pattern so future EVM-side integrations can recognise it.
pub const UNRESOLVED_ANSWER_BYTES: [u8; 32] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfe,
];

/// Question identifier — a content-derived 32-byte hash. Derivation includes
/// the contract address and the user-supplied nonce (FM-5 from lessons §6),
/// so a nonce collision under the same asker reverts at hash time.
pub type QuestionId = [u8; 32];

/// Per-instance configuration. Set at instantiation, never mutated.
#[cw_serde]
pub struct Config {
    /// Instantiator / migration admin. Optional — `None` is a published-frozen
    /// instance with no upgrade path.
    pub admin: Option<Addr>,
    /// Mandatory floor for `initial_bond` on every `AskQuestion`. Defends
    /// against tiny-bond griefing (FM-1).
    pub min_initial_bond_floor: Uint128,
    /// Mandatory floor for `answer_timeout_secs` on every `AskQuestion`.
    /// Reality.eth has no contract-level floor; SuDAO + Gnosis Guild were
    /// drained partly because their integrators chose 12h / 1h timeouts
    /// (FM-8).
    pub min_answer_timeout_secs: u32,
}

/// Coarse answer type. The exact payload bytes are opaque to the contract;
/// cw-filter validates schema per question.
#[cw_serde]
pub enum AnswerType {
    Bool,
    Uint,
    String,
    Bytes,
}

/// The explicit state machine derived from Reality.eth's modifier set.
#[cw_serde]
pub enum State {
    /// Question does not exist.
    NotCreated,
    /// Created but no answer yet.
    OpenUnanswered,
    /// At least one answer submitted; not in arbitration; not past
    /// `finalize_ts`.
    OpenAnswered,
    /// Arbitrator froze the question via `RequestArbitration`.
    PendingArbitration,
    /// `finalize_ts` reached (or arbitration finalized); `Claim` not yet
    /// drained the history.
    Finalized,
    /// History fully drained — all payouts credited.
    Claimed,
}

/// A question's stored state. See lessons §1 for the precise transitions
/// each handler asserts.
#[cw_serde]
pub struct Question {
    pub asker: Addr,
    pub text: String,
    pub answer_type: AnswerType,
    /// Bond denom pinned at ask time. Every subsequent bond on this question
    /// must match (lessons §2.3).
    pub bond_denom: String,
    pub initial_bond: Uint128,
    pub min_bond: Uint128,
    pub answer_timeout_secs: u32,
    pub arbitrator: Option<Addr>,
    /// Maximum time the arbitrator has between `RequestArbitration` and
    /// `SubmitArbitration` before a non-arbitrator may force-cancel
    /// (re-extension defense against a stalled arbitrator).
    pub arbitration_timeout_secs: u32,
    /// Arbitration request deadline (set when `RequestArbitration` lands).
    /// If `None`, no arbitration request is outstanding.
    pub arbitration_deadline: Option<u64>,
    /// Per-question cw-filter binding. Captured at ask time so later
    /// cw-filter migrations cannot brick this question (lessons §7.4).
    pub answer_schema: Option<AnswerSchemaFilter>,
    /// User-supplied nonce — enables an asker to re-pose the same question
    /// (lessons §8 q1).
    pub nonce: u64,
    /// Optional opening timestamp; question is not answerable until this
    /// time has elapsed (Reality.eth `opening_ts`).
    pub opening_ts: Option<u64>,
    /// Asker-supplied bounty (paid out to the latest right-answerer).
    pub bounty: Uint128,
    // ---- Mutable fields ----
    /// Latest "best" answer (i.e. the answer of the highest-bonded round so
    /// far). `None` until the first answer lands.
    pub best_answer: Option<Binary>,
    /// Latest bond amount in the chain. Used for the 2× doubling guard.
    pub current_bond: Uint128,
    /// Tip of the history-hash chain.
    pub history_hash: HistoryHash,
    /// Number of rounds (answers + arbitrator-submitted answers) so far.
    pub round_count: u32,
    /// `block.time.seconds()` when the question will finalize, idle.
    /// `None` until the first answer lands.
    pub finalize_ts: Option<u64>,
    /// Frozen by `RequestArbitration`; cleared by `CancelArbitration` or
    /// `SubmitArbitration`.
    pub is_pending_arbitration: bool,
    /// Set when the question is fully claimed (`history_hash` exhausted on
    /// the resumable claim walk).
    pub is_claimed: bool,
}

impl Question {
    /// Derive the explicit state from stored fields, given `now`.
    pub fn state_at(&self, now: u64) -> State {
        if self.is_claimed {
            return State::Claimed;
        }
        if self.is_pending_arbitration {
            return State::PendingArbitration;
        }
        match self.finalize_ts {
            None => State::OpenUnanswered,
            Some(ts) if ts <= now => State::Finalized,
            Some(_) => State::OpenAnswered,
        }
    }
}

/// Resumable-claim state per Reality.eth's `question_claims[qid]`. Allocated
/// lazily during a `Claim` call that runs out of gas (or hits a self-imposed
/// per-call round limit). Removed on completion.
#[cw_serde]
pub struct Claim {
    pub payee: Option<Addr>,
    pub last_bond: Uint128,
    pub queued_funds: Uint128,
    /// Tip of the partial history walk — same shape as Reality.eth's
    /// `questions[qid].history_hash` after a partial claim.
    pub cursor_hash: HistoryHash,
}

// ---- Storage ----

pub const CONFIG: Item<Config> = Item::new("config");

pub const QUESTIONS: Map<&[u8], Question> = Map::new("questions");

/// Pull-payment ledger: address → (denom → amount). Drained via `Withdraw`.
/// Per-denom map matches Reality.eth's `balanceOf` semantics while supporting
/// the multi-denom model documented in lessons §2.3.
pub const BALANCES: Map<(&Addr, &str), Uint128> = Map::new("balances");

/// Resumable claim state for partially-claimed questions.
pub const CLAIMS: Map<&[u8], Claim> = Map::new("claims");
