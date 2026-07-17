//! Binary-market public wire protocol.

use crate::question::QuestionInput;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, Uint128, Uint512};
use cw_reality::state::Question as OracleQuestion;
use pm_types::{Outcome, Payout, ProtocolVersion, TierId};

#[cw_serde]
pub struct InstantiateMsg {
    pub factory: String,
    pub creator: String,
    pub oracle: String,
    /// Immutable exact sender authorized to forward the arbitration verdict.
    pub verdict_authority: String,
    pub tier: TierId,
    /// Typed semantic source. The market constructs authoritative JCS bytes.
    pub question: QuestionInput,
    pub nonce: u64,
    pub close_ts: u64,
    pub opening_ts: u64,
    pub initial_liquidity: Uint128,
    pub oracle_bounty: Uint128,
    pub oracle_initial_bond: Uint128,
    pub answer_timeout_secs: u32,
    pub arbitration_timeout_secs: u32,
    /// Fee numerator over 10,000.
    pub fee_bps: u16,
    pub min_trade: Uint128,
    /// Maximum gross trade as a numerator over 10,000 of the selected reserve.
    pub max_trade_bps: u16,
    /// Maximum position units held by one address for either outcome.
    pub max_position_per_side: Uint128,
    pub collateral_cap: Uint128,
    pub challenge_bond: Uint128,
}

#[cw_serde]
pub enum ExecuteMsg {
    Split {
        amount: Uint128,
    },
    Merge {
        amount: Uint128,
    },
    Buy {
        outcome: Outcome,
        min_out: Uint128,
        deadline: u64,
    },
    Sell {
        outcome: Outcome,
        return_amount: Uint128,
        max_in: Uint128,
        deadline: u64,
    },
    Challenge {},
    GovernanceVerdict {
        question_id: Binary,
        answer: Binary,
        payee: String,
    },
    FinalizeStalledChallenge {},
    Resolve {},
    RedeemPositions {
        yes: Uint128,
        no: Uint128,
    },
    RedeemLp {
        amount: Uint128,
    },
    ClaimLpAccrual {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(IdentityResponse)]
    Identity {},
    #[returns(StateResponse)]
    State {},
    #[returns(AccountingResponse)]
    Accounting {},
    #[returns(PoolResponse)]
    Pool {},
    #[returns(QuoteResponse)]
    QuoteBuy { outcome: Outcome, gross: Uint128 },
    #[returns(QuoteResponse)]
    QuoteSell {
        outcome: Outcome,
        return_amount: Uint128,
    },
    #[returns(PositionResponse)]
    Position { address: String },
    #[returns(LpPositionResponse)]
    LpPosition {},
    #[returns(ChallengeResponse)]
    Challenge {},
    #[returns(ResolutionResponse)]
    Resolution {},
    #[returns(SolvencyResponse)]
    Solvency {},
    #[returns(QuestionResponse)]
    Question {},
}

#[cw_serde]
pub enum LifecycleStatus {
    Initializing,
    Trading,
    AwaitingResolution,
    PendingArbitration,
    Resolved,
}

#[cw_serde]
pub struct ConfigResponse {
    pub protocol_version: ProtocolVersion,
    pub factory: String,
    pub creator: String,
    pub initial_lp: String,
    pub oracle: String,
    /// Immutable exact sender authorized to forward the arbitration verdict.
    pub verdict_authority: String,
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
}

#[cw_serde]
pub struct IdentityResponse {
    pub protocol_version: ProtocolVersion,
    pub factory: String,
    pub market: String,
    pub nonce: u64,
    pub question_id: Option<Binary>,
}
#[cw_serde]
pub struct StateResponse {
    pub status: LifecycleStatus,
    pub activated: bool,
    pub challenge_used: bool,
}
#[cw_serde]
pub struct AccountingResponse {
    pub principal: Uint128,
    pub fees: Uint128,
    pub challenge: Uint128,
    pub terminal_liability_twice: Option<Uint128>,
    pub total_yes: Uint128,
    pub total_no: Uint128,
    pub lp_supply: Uint128,
    pub lp_burned: Uint128,
    pub lp_paid: Uint128,
    pub neutral_half_dust: u8,
    pub lp_accrual: Uint128,
    pub principal_at_resolution: Option<Uint128>,
    pub fees_at_resolution: Option<Uint128>,
    pub pool_yes_at_resolution: Option<Uint128>,
    pub pool_no_at_resolution: Option<Uint128>,
    pub total_yes_at_resolution: Option<Uint128>,
    pub total_no_at_resolution: Option<Uint128>,
}
#[cw_serde]
pub struct PoolResponse {
    pub yes: Uint128,
    pub no: Uint128,
}
#[cw_serde]
pub struct PositionResponse {
    pub address: String,
    pub yes: Uint128,
    pub no: Uint128,
}
#[cw_serde]
pub struct LpPositionResponse {
    pub owner: String,
    pub supply: Uint128,
    pub burned: Uint128,
    pub paid: Uint128,
    pub later_accrual: Uint128,
}
#[cw_serde]
pub struct ChallengeResponse {
    pub challenger: Option<String>,
    pub answer: Option<Binary>,
    pub answer_hex: Option<String>,
    pub answer_base64: Option<String>,
    pub oracle_bond: Option<Uint128>,
    pub challenge_bond: Uint128,
    pub started_at: Option<u64>,
    pub deadline: Option<u64>,
    pub oracle_snapshot: Option<OracleQuestion>,
}
#[cw_serde]
pub struct ResolutionResponse {
    pub answer: Option<Binary>,
    pub answer_hex: Option<String>,
    pub answer_base64: Option<String>,
    pub payout: Option<Payout>,
    pub height: Option<u64>,
    pub time: Option<u64>,
    pub principal_at_resolution: Option<Uint128>,
    pub terminal_liability_twice: Option<Uint128>,
    pub pool_yes_at_resolution: Option<Uint128>,
    pub pool_no_at_resolution: Option<Uint128>,
    pub total_yes_at_resolution: Option<Uint128>,
    pub total_no_at_resolution: Option<Uint128>,
}
#[cw_serde]
pub struct SolvencyResponse {
    pub height: u64,
    pub block_time: u64,
    pub bank_balance: Uint128,
    pub principal_or_terminal_liability: Uint128,
    pub fee_liability: Uint128,
    pub challenge_liability: Uint128,
    pub lp_whole_coin_accrual: Uint128,
    pub accounted_liability: Uint128,
    pub forced_excess: Uint128,
    pub shortfall: Uint128,
    pub solvent: bool,
}
#[cw_serde]
pub struct QuestionResponse {
    pub text: String,
    pub hash: Binary,
    pub hash_hex: String,
    pub hash_base64: String,
    pub nonce: u64,
    pub question_id: Option<Binary>,
    pub oracle: String,
    pub opening_ts: u64,
    pub close_ts: u64,
    pub yes_answer_hex: String,
    pub yes_answer_base64: String,
    pub no_answer_hex: String,
    pub no_answer_base64: String,
    pub invalid_answer_hex: String,
    pub invalid_answer_base64: String,
    pub unresolved_answer_hex: String,
    pub unresolved_answer_base64: String,
}

/// Exact rational wire value. `Uint512` serializes as an unsigned decimal
/// string and safely holds cross-products used by price impact.
#[cw_serde]
pub struct ExactRatio {
    pub numerator: Uint512,
    pub denominator: Uint512,
}

#[cw_serde]
pub enum ImpactDirection {
    Up,
    Down,
    Flat,
}

#[cw_serde]
pub struct QuoteResponse {
    pub height: u64,
    pub block_time: u64,
    pub outcome: Outcome,
    pub gross: Uint128,
    pub net: Uint128,
    pub fee: Uint128,
    pub input: Uint128,
    pub output: Uint128,
    pub reserve_yes_before: Uint128,
    pub reserve_no_before: Uint128,
    pub reserve_yes_after: Uint128,
    pub reserve_no_after: Uint128,
    pub average_price: ExactRatio,
    pub marginal_before: ExactRatio,
    pub marginal_after: ExactRatio,
    pub fee_rate: ExactRatio,
    pub price_impact: ExactRatio,
    pub impact_direction: ImpactDirection,
    /// Buy's exact execute-side caller protection. Omitted for sells.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_out: Option<Uint128>,
    /// Sell's exact execute-side caller protection. Omitted for buys.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_in: Option<Uint128>,
}
