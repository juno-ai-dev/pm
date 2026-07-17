use binary_market::question::QuestionInput;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, HexBinary, Uint128};
use pm_types::{ProtocolVersion, TierId};

#[cw_serde]
pub struct TierConfig {
    pub min_initial_liquidity: Uint128,
    pub max_initial_liquidity: Uint128,
    pub min_oracle_bounty: Uint128,
    pub max_oracle_bounty: Uint128,
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
pub struct InstantiateMsg {
    pub protocol_version: ProtocolVersion,
    pub market_code_id: u64,
    pub market_checksum: HexBinary,
    pub tier_id: TierId,
    pub tier: TierConfig,
    pub oracle: String,
    pub oracle_code_id: u64,
    pub oracle_checksum: HexBinary,
    pub verdict_authority: String,
    pub collateral_denom: String,
    pub oracle_min_initial_bond_floor: Uint128,
    pub oracle_min_answer_timeout_secs: u32,
}

#[cw_serde]
pub struct CreateMarketMsg {
    pub question: QuestionInput,
    pub close_ts: u64,
    pub opening_ts: u64,
    pub initial_liquidity: Uint128,
    pub oracle_bounty: Uint128,
}

#[cw_serde]
pub enum ExecuteMsg {
    CreateMarket(CreateMarketMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(MarketResponse)]
    Market { nonce: u64 },
    #[returns(NextNonceResponse)]
    NextNonce {},
    #[returns(ListMarketsResponse)]
    ListMarkets {
        start_after_nonce: Option<u64>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    pub protocol_version: ProtocolVersion,
    pub market_code_id: u64,
    pub market_checksum: HexBinary,
    pub tier_id: TierId,
    pub tier: TierConfig,
    pub oracle: String,
    pub oracle_code_id: u64,
    pub oracle_checksum: HexBinary,
    pub verdict_authority: String,
    pub collateral_denom: String,
    pub oracle_min_initial_bond_floor: Uint128,
    pub oracle_min_answer_timeout_secs: u32,
}

#[cw_serde]
pub struct MarketRecord {
    pub nonce: u64,
    pub market: String,
    pub creator: String,
    pub tier_id: TierId,
    pub question_id: Binary,
    pub question_hash: Binary,
    pub close_ts: u64,
    pub opening_ts: u64,
    pub initial_liquidity: Uint128,
    pub oracle_bounty: Uint128,
    pub created_height: u64,
    pub created_time: u64,
}

#[cw_serde]
pub struct MarketResponse {
    pub market: MarketRecord,
}
#[cw_serde]
pub struct ListMarketsResponse {
    pub markets: Vec<MarketRecord>,
    pub next_start_after_nonce: Option<u64>,
}
#[cw_serde]
pub struct NextNonceResponse {
    pub next_nonce: u64,
}
