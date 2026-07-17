//! Typed canonical `juno-pm-question/1` construction and oracle identity binding.

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Binary, StdError, StdResult, Uint128};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::error::ContractError;

pub const QUESTION_VERSION: &str = "juno-pm-question/1";
pub const MAX_QUESTION_BYTES: usize = 16_384;
pub const ANSWER_TIMEOUT_SECS: u32 = 86_400;
pub const ARBITRATION_TIMEOUT_SECS: u32 = 1_814_400;
pub const MIN_ORACLE_INITIAL_BOND: u128 = 10_000_000;
pub const MIN_ORACLE_BOUNTY: u128 = 1_000_000;
pub const MIN_CREATION_TO_CLOSE: u64 = 86_400;
pub const MAX_CREATION_TO_CLOSE: u64 = 7_776_000;
pub const MAX_OPENING_DELAY: u64 = 2_592_000;

pub const NO_HEX: &str = "0000000000000000000000000000000000000000000000000000000000000000";
pub const YES_HEX: &str = "0000000000000000000000000000000000000000000000000000000000000001";
pub const INVALID_HEX: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
pub const UNRESOLVED_HEX: &str = "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe";

#[cw_serde]
pub struct SourceInput {
    pub publisher: String,
    pub identifier: String,
    pub url: String,
    pub retrieval: String,
    pub publication_revision_policy: String,
    pub fallback_condition: String,
}

#[cw_serde]
pub struct ObservationInput {
    pub start_ts: u64,
    pub end_ts: u64,
    pub cutoff_ts: u64,
    pub inclusivity: String,
    pub revision_policy: String,
}

#[cw_serde]
pub struct QuestionInput {
    pub title: String,
    pub proposition: String,
    pub definitions: Vec<String>,
    pub invalid_conditions: Vec<String>,
    pub primary_sources: Vec<SourceInput>,
    pub secondary_sources: Vec<SourceInput>,
    pub source_disagreement_policy: String,
    pub observation: ObservationInput,
}

// Fields are deliberately declared in UTF-16/ASCII lexicographic key order.
// With this integer/string/array-only typed value, serde_json's compact encoding
// is RFC 8785 JCS (there are no floats or unordered maps).
#[derive(Serialize)]
struct CanonicalDocument<'a> {
    answer_encoding: AnswerEncoding,
    answer_timeout_secs: u32,
    arbitration_timeout_secs: u32,
    challenge_bond_rule: &'static str,
    close_ts: u64,
    collateral_denom: &'static str,
    definitions: &'a [String],
    invalid_conditions: &'a [String],
    language: &'static str,
    market_controller: &'a str,
    observation: ObservationDocument<'a>,
    opening_ts: u64,
    oracle: &'a str,
    oracle_bond_denom: &'static str,
    oracle_initial_bond: String,
    oracle_question_type: &'static str,
    payouts: Payouts,
    primary_sources: Vec<SourceDocument<'a>>,
    proposition: &'a str,
    question_version: &'static str,
    secondary_sources: Vec<SourceDocument<'a>>,
    source_disagreement_policy: &'a str,
    title: &'a str,
    verdict_authority: &'a str,
}

#[derive(Serialize)]
struct AnswerEncoding {
    invalid_hex: &'static str,
    no_hex: &'static str,
    unknown_policy: &'static str,
    unresolved_hex: &'static str,
    yes_hex: &'static str,
}
#[derive(Serialize)]
struct ObservationDocument<'a> {
    cutoff_ts: u64,
    end_ts: u64,
    inclusivity: &'a str,
    revision_policy: &'a str,
    start_ts: u64,
    timezone: &'static str,
}
#[derive(Serialize)]
struct Payouts {
    invalid: [&'static str; 2],
    no: [&'static str; 2],
    unrecognized: [&'static str; 2],
    unresolved: [&'static str; 2],
    yes: [&'static str; 2],
}
#[derive(Serialize)]
struct SourceDocument<'a> {
    fallback_condition: &'a str,
    identifier: &'a str,
    publication_revision_policy: &'a str,
    publisher: &'a str,
    retrieval: &'a str,
    url: &'a str,
}

fn bounded(value: &str, min: usize, max: usize, field: &str) -> Result<(), ContractError> {
    let len = value.len();
    if len < min || len > max {
        return Err(ContractError::InvalidConfig(format!(
            "{field} must be {min}..={max} UTF-8 bytes"
        )));
    }
    Ok(())
}

fn validate_list(
    values: &[String],
    min: usize,
    max: usize,
    field: &str,
) -> Result<(), ContractError> {
    if values.len() < min || values.len() > max {
        return Err(ContractError::InvalidConfig(format!(
            "{field} must contain {min}..={max} entries"
        )));
    }
    for value in values {
        bounded(value, 1, 512, field)?;
    }
    Ok(())
}

fn validate_sources(values: &[SourceInput], min: usize, field: &str) -> Result<(), ContractError> {
    if values.len() < min || values.len() > 5 {
        return Err(ContractError::InvalidConfig(format!(
            "{field} must contain {min}..=5 entries"
        )));
    }
    for source in values {
        bounded(&source.publisher, 1, 128, "source.publisher")?;
        bounded(&source.identifier, 1, 256, "source.identifier")?;
        bounded(&source.url, 1, 2_048, "source.url")?;
        if !source.url.starts_with("https://") || source.url.chars().any(char::is_whitespace) {
            return Err(ContractError::InvalidConfig(
                "source.url must be an absolute https URI without whitespace".into(),
            ));
        }
        bounded(&source.retrieval, 1, 128, "source.retrieval")?;
        bounded(
            &source.publication_revision_policy,
            1,
            512,
            "source.publication_revision_policy",
        )?;
        bounded(
            &source.fallback_condition,
            1,
            512,
            "source.fallback_condition",
        )?;
    }
    Ok(())
}

fn source_document(source: &SourceInput) -> SourceDocument<'_> {
    SourceDocument {
        fallback_condition: &source.fallback_condition,
        identifier: &source.identifier,
        publication_revision_policy: &source.publication_revision_policy,
        publisher: &source.publisher,
        retrieval: &source.retrieval,
        url: &source.url,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn canonical_question(
    input: &QuestionInput,
    market: &Addr,
    oracle: &Addr,
    governance: &Addr,
    close_ts: u64,
    opening_ts: u64,
    oracle_initial_bond: Uint128,
    creation_ts: u64,
) -> Result<(String, Binary), ContractError> {
    bounded(&input.title, 1, 160, "title")?;
    bounded(&input.proposition, 1, 1_024, "proposition")?;
    validate_list(&input.definitions, 0, 16, "definitions")?;
    validate_list(&input.invalid_conditions, 1, 16, "invalid_conditions")?;
    validate_sources(&input.primary_sources, 1, "primary_sources")?;
    validate_sources(&input.secondary_sources, 0, "secondary_sources")?;
    bounded(
        &input.source_disagreement_policy,
        1,
        1_024,
        "source_disagreement_policy",
    )?;
    bounded(
        &input.observation.revision_policy,
        1,
        512,
        "observation.revision_policy",
    )?;
    bounded(
        &input.observation.inclusivity,
        1,
        32,
        "observation.inclusivity",
    )?;
    if creation_ts >= close_ts
        || close_ts - creation_ts < MIN_CREATION_TO_CLOSE
        || close_ts - creation_ts > MAX_CREATION_TO_CLOSE
        || opening_ts < close_ts
        || opening_ts - close_ts > MAX_OPENING_DELAY
        || input.observation.start_ts > input.observation.end_ts
        || input.observation.end_ts > input.observation.cutoff_ts
        || close_ts > input.observation.cutoff_ts
        || input.observation.cutoff_ts > opening_ts
    {
        return Err(ContractError::InvalidConfig(
            "invalid creation, close, observation, or opening timestamp ordering".into(),
        ));
    }

    let document = CanonicalDocument {
        answer_encoding: AnswerEncoding {
            invalid_hex: INVALID_HEX,
            no_hex: NO_HEX,
            unknown_policy: "neutral",
            unresolved_hex: UNRESOLVED_HEX,
            yes_hex: YES_HEX,
        },
        answer_timeout_secs: ANSWER_TIMEOUT_SECS,
        arbitration_timeout_secs: ARBITRATION_TIMEOUT_SECS,
        challenge_bond_rule: "max(tier_floor,current_oracle_bond)",
        close_ts,
        collateral_denom: "ujuno",
        definitions: &input.definitions,
        invalid_conditions: &input.invalid_conditions,
        language: "en",
        market_controller: market.as_str(),
        observation: ObservationDocument {
            cutoff_ts: input.observation.cutoff_ts,
            end_ts: input.observation.end_ts,
            inclusivity: &input.observation.inclusivity,
            revision_policy: &input.observation.revision_policy,
            start_ts: input.observation.start_ts,
            timezone: "UTC",
        },
        opening_ts,
        oracle: oracle.as_str(),
        oracle_bond_denom: "ujuno",
        oracle_initial_bond: oracle_initial_bond.to_string(),
        oracle_question_type: "bool",
        payouts: Payouts {
            invalid: ["1/2", "1/2"],
            no: ["0", "1"],
            unrecognized: ["1/2", "1/2"],
            unresolved: ["1/2", "1/2"],
            yes: ["1", "0"],
        },
        primary_sources: input.primary_sources.iter().map(source_document).collect(),
        proposition: &input.proposition,
        question_version: QUESTION_VERSION,
        secondary_sources: input
            .secondary_sources
            .iter()
            .map(source_document)
            .collect(),
        source_disagreement_policy: &input.source_disagreement_policy,
        title: &input.title,
        verdict_authority: governance.as_str(),
    };
    let bytes = serde_json::to_vec(&document)
        .map_err(|e| ContractError::InvalidConfig(format!("question JCS encoding failed: {e}")))?;
    if bytes.len() > MAX_QUESTION_BYTES {
        return Err(ContractError::InvalidConfig(
            "canonical question exceeds 16384 bytes".into(),
        ));
    }
    let text = String::from_utf8(bytes.clone())
        .map_err(|_| ContractError::InvalidConfig("canonical question is not UTF-8".into()))?;
    let hash = Binary::from(Sha256::digest(&bytes).to_vec());
    Ok((text, hash))
}

#[allow(clippy::too_many_arguments)]
pub fn question_id(
    api: &dyn Api,
    oracle: &Addr,
    market: &Addr,
    nonce: u64,
    content_hash: &[u8; 32],
    answer_timeout_secs: u32,
    initial_bond: Uint128,
    opening_ts: u64,
) -> StdResult<Binary> {
    let oracle = api.addr_canonicalize(oracle.as_str())?;
    let market = api.addr_canonicalize(market.as_str())?;
    Ok(Binary::from(question_id_from_canonical(
        oracle.as_slice(),
        market.as_slice(),
        nonce,
        content_hash,
        market.as_slice(),
        answer_timeout_secs,
        initial_bond,
        opening_ts,
    )))
}

/// Pure byte-level form used for cross-language vectors, including Juno's
/// 20-byte account and 32-byte contract canonical-address lengths.
#[allow(clippy::too_many_arguments)]
pub fn question_id_from_canonical(
    oracle: &[u8],
    market: &[u8],
    nonce: u64,
    content_hash: &[u8; 32],
    arbitrator: &[u8],
    answer_timeout_secs: u32,
    initial_bond: Uint128,
    opening_ts: u64,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(oracle);
    hasher.update(market);
    hasher.update(nonce.to_be_bytes());
    hasher.update(content_hash);
    hasher.update([1u8]);
    hasher.update(arbitrator);
    hasher.update(answer_timeout_secs.to_be_bytes());
    hasher.update(initial_bond.u128().to_be_bytes());
    hasher.update(("ujuno".len() as u32).to_be_bytes());
    hasher.update(b"ujuno");
    hasher.update(opening_ts.to_be_bytes());
    hasher.finalize().into()
}

pub fn hash_array(hash: &Binary) -> StdResult<[u8; 32]> {
    hash.as_slice()
        .try_into()
        .map_err(|_| StdError::generic_err("question hash must be 32 bytes"))
}
