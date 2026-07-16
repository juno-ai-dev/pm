//! Shared prediction-market wire types.
//!
//! These are value and serialization types only. Contract messages and state
//! transitions belong to their respective implementation crates.

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, Uint128};

/// The only collateral denomination supported by the v1 shared types.
pub const UJUNO_DENOM: &str = "ujuno";

/// One of the two tradeable outcomes.
#[cw_serde]
pub enum Outcome {
    Yes,
    No,
}

/// An exact terminal payout vector.
///
/// Numerators and the denominator are `Uint128` so JSON never passes these
/// consensus amounts through a floating-point or JavaScript number.
#[cw_serde]
pub struct Payout {
    pub yes_numerator: Uint128,
    pub no_numerator: Uint128,
    pub denominator: Uint128,
}

impl Payout {
    /// Returns the exact valid-outcome payout vector.
    #[must_use]
    pub fn for_outcome(outcome: Outcome) -> Self {
        match outcome {
            Outcome::Yes => Self {
                yes_numerator: Uint128::one(),
                no_numerator: Uint128::zero(),
                denominator: Uint128::one(),
            },
            Outcome::No => Self {
                yes_numerator: Uint128::zero(),
                no_numerator: Uint128::one(),
                denominator: Uint128::one(),
            },
        }
    }

    /// Returns the exact neutral half/half payout vector.
    #[must_use]
    pub fn neutral() -> Self {
        Self {
            yes_numerator: Uint128::one(),
            no_numerator: Uint128::one(),
            denominator: Uint128::new(2),
        }
    }
}

/// Version of the shared prediction-market wire protocol.
#[cw_serde]
pub enum ProtocolVersion {
    V1,
}

/// Stable identifier for a factory-defined security tier.
///
/// Economic parameters deliberately remain factory configuration rather than
/// being invented by this foundational type.
#[cw_serde]
#[serde(transparent)]
pub struct TierId(pub u16);

/// A native `ujuno` amount, encoded as a decimal JSON string by `Uint128`.
#[cw_serde]
#[serde(transparent)]
pub struct Ujuno(pub Uint128);

/// An absolute Unix timestamp in seconds.
#[cw_serde]
#[serde(transparent)]
pub struct UnixTimestamp(pub u64);

/// Opaque oracle answer bytes, encoded as base64 on the JSON wire.
#[cw_serde]
#[serde(transparent)]
pub struct OracleAnswer(pub Binary);

/// Immutable identity and timing shared by a market and its oracle question.
#[cw_serde]
pub struct Question {
    /// Opaque identifier returned or derived under the pinned oracle protocol.
    pub id: Binary,
    /// Exact question bytes interpreted as UTF-8 text by cw-reality.
    pub text: String,
    /// Last trading boundary, as Unix seconds.
    pub close_time: UnixTimestamp,
    /// Earliest oracle answer boundary, as Unix seconds.
    pub opening_time: UnixTimestamp,
}

/// Schema-only aggregate that keeps every shared public type in one snapshot.
#[cw_serde]
pub struct PublicTypes {
    pub outcome: Outcome,
    pub payout: Payout,
    pub version: ProtocolVersion,
    pub tier: TierId,
    pub question: Question,
    pub amount: Ujuno,
    pub answer: OracleAnswer,
}

/// Deterministic fixtures reusable by future contract test suites.
#[cfg(feature = "test-utils")]
pub mod testing {
    use super::{Binary, Question, UnixTimestamp};

    /// Returns a small question fixture with absolute Unix-second boundaries.
    #[must_use]
    pub fn sample_question() -> Question {
        Question {
            id: Binary::from(b"question-id"),
            text: "Will the event occur?".to_owned(),
            close_time: UnixTimestamp(1_800_000_000),
            opening_time: UnixTimestamp(1_800_000_001),
        }
    }
}
