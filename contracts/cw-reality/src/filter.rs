//! Local mirror of cw-filter's wire-format types.
//!
//! We do not depend on the cw-filter crate directly — its transitive
//! `cw-jsonfilter` pulls in `alloy-rpc-types-eth` which conflicts in this
//! workspace. Pattern mirrored from
//! `dao-contracts/contracts/proposal/dao-proposal-wavs/src/filter.rs`.
//!
//! Canonical implementation: `dao-contracts/contracts/external/cw-filter/`.

use cosmwasm_schema::cw_serde;
use cosmwasm_std::CosmosMsg;
use serde_json::Value;

#[cw_serde]
pub enum FilterQueryMsg {
    /// Run the supplied filter against the supplied CosmosMsg envelope.
    Filter { filter: Value, msg: CosmosMsg },
}

#[cw_serde]
pub enum FilterResponse {
    /// Filter accepted the message.
    Pass {},
    /// Filter rejected this specific message. Reject the question/answer cleanly.
    Fail { reason: String },
    /// Filter signaled protocol corruption (malformed filter, version drift, etc.).
    /// Reject and surface so off-chain consumers can distinguish from `Fail`.
    Fatal { reason: String },
}

/// Per-question configuration of the answer-schema filter callout.
///
/// Stored on the `Question` at ask time. The contract address is captured
/// at ask time so that later cw-filter migrations cannot brick existing
/// questions (lessons §7.4). If `None`, no schema validation is performed
/// and any payload of the declared `AnswerType` is accepted.
#[cw_serde]
pub struct AnswerSchemaFilter {
    /// cw-filter contract address used for this question. Captured at ask
    /// time so the question's validation surface is stable.
    pub contract: String,
    /// JSON filter spec to run against every submitted answer.
    pub filter: Value,
}
