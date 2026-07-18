//! Persistent binary-market state. Config has no write helper after instantiate.

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Uint128};
use cw_reality::state::Question as OracleQuestion;
use cw_storage_plus::{Item, Map};
use pm_types::{Payout, ProtocolVersion, TierId};

#[cw_serde]
pub struct Config {
    pub protocol_version: ProtocolVersion,
    pub factory: Addr,
    pub creator: Addr,
    pub initial_lp: Addr,
    pub oracle: Addr,
    pub verdict_authority: Addr,
    pub tier: TierId,
    pub collateral_denom: String,
    pub close_ts: u64,
    pub opening_ts: u64,
    pub initial_liquidity: Uint128,
    pub oracle_bounty: Uint128,
    pub oracle_initial_bond: Uint128,
    pub answer_timeout_secs: u32,
    pub arbitration_timeout_secs: u32,
    pub fee_bps: u16,
    pub min_trade: Uint128,
    pub max_trade_bps: u16,
    pub max_position_per_side: Uint128,
    pub collateral_cap: Uint128,
    pub challenge_bond: Uint128,
    pub yes_answer: Binary,
    pub no_answer: Binary,
    pub invalid_answer: Binary,
    pub unresolved_answer: Binary,
    pub question: String,
    pub question_hash: Binary,
    pub nonce: u64,
}

#[cw_serde]
pub struct Lifecycle {
    pub activated: bool,
    pub payout: Option<Payout>,
    pub resolution_answer: Option<Binary>,
    pub resolution_height: Option<u64>,
    pub resolution_time: Option<u64>,
    pub challenge_used: bool,
}

#[cw_serde]
pub struct Accounting {
    /// P: complete-set collateral principal.
    pub principal: Uint128,
    /// F: accrued fees.
    pub fees: Uint128,
    /// C: challenge-bond liability.
    pub challenge: Uint128,
    pub pool_yes: Uint128,
    pub pool_no: Uint128,
    pub total_yes: Uint128,
    pub total_no: Uint128,
    pub lp_supply: Uint128,
    pub lp_burned: Uint128,
    pub lp_paid: Uint128,
    pub neutral_half_dust: u8,
    pub lp_accrual: Uint128,
    pub principal_at_resolution: Option<Uint128>,
    /// Immutable F snapshot used by cumulative LP fee floors.
    pub fees_at_resolution: Option<Uint128>,
    /// T2: unpaid position liability in half-ujuno numerator units.
    pub terminal_liability_twice: Option<Uint128>,
    pub pool_yes_at_resolution: Option<Uint128>,
    pub pool_no_at_resolution: Option<Uint128>,
    pub total_yes_at_resolution: Option<Uint128>,
    pub total_no_at_resolution: Option<Uint128>,
}

#[cw_serde]
pub struct Position {
    pub yes: Uint128,
    pub no: Uint128,
}
impl Default for Position {
    fn default() -> Self {
        Self {
            yes: Uint128::zero(),
            no: Uint128::zero(),
        }
    }
}

#[cw_serde]
pub struct NeutralRedemption {
    pub cumulative_numerator: Uint128,
    pub whole_paid: Uint128,
    pub finalized_half: bool,
}

#[cw_serde]
pub struct Challenge {
    pub challenger: Addr,
    pub answer: Binary,
    pub oracle_bond: Uint128,
    pub started_at: u64,
    pub deadline: u64,
    /// Exact oracle state before `RequestArbitration`. All later arbitration
    /// transitions are verified against this consensus snapshot.
    pub oracle_snapshot: OracleQuestion,
}

#[cw_serde]
pub enum ReplyInProgress {
    Activation { expected_question_id: Binary },
    Challenge { challenger: Addr },
    GovernanceVerdict { answer: Binary, payee: Addr },
    StalledCancellation,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const LIFECYCLE: Item<Lifecycle> = Item::new("lifecycle");
pub const ACCOUNTING: Item<Accounting> = Item::new("accounting");
pub const QUESTION_ID: Item<Binary> = Item::new("question_id");
pub const CHALLENGE: Item<Challenge> = Item::new("challenge");
pub const REPLY_IN_PROGRESS: Item<ReplyInProgress> = Item::new("reply_in_progress");
pub const POSITIONS: Map<&Addr, Position> = Map::new("positions");
pub const NEUTRAL_REDEMPTIONS: Map<&Addr, NeutralRedemption> = Map::new("neutral_redemptions");

/// Position balances deliberately use zero semantics for absent addresses.
pub fn load_position(
    storage: &dyn cosmwasm_std::Storage,
    address: &Addr,
) -> cosmwasm_std::StdResult<Position> {
    Ok(POSITIONS.may_load(storage, address)?.unwrap_or_default())
}
