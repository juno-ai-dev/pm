use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, HexBinary, Uint128};
use cw_storage_plus::{Item, Map};
use pm_types::{ProtocolVersion, TierId};

use crate::msg::{CreateMarketMsg, MarketRecord, TierConfig};

#[cw_serde]
pub struct Config {
    pub protocol_version: ProtocolVersion,
    pub market_code_id: u64,
    pub market_checksum: HexBinary,
    pub tier_id: TierId,
    pub tier: TierConfig,
    pub oracle: Addr,
    pub oracle_code_id: u64,
    pub oracle_checksum: HexBinary,
    pub verdict_authority: Addr,
    pub collateral_denom: String,
    pub oracle_min_initial_bond_floor: Uint128,
    pub oracle_min_answer_timeout_secs: u32,
}

#[cw_serde]
pub struct PendingCreation {
    pub creator: Addr,
    pub nonce: u64,
    pub request: CreateMarketMsg,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const PENDING: Item<PendingCreation> = Item::new("pending");
pub const NEXT_NONCE: Item<u64> = Item::new("next_nonce");
pub const MARKETS: Map<u64, MarketRecord> = Map::new("markets");
