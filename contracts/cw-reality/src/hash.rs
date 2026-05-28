//! History-hash chain helpers.
//!
//! Reality.eth (`RealityETH-3.0.sol:481-493`) records every answer as a step
//! in a hash chain:
//!
//! ```text
//! H_n = keccak256(H_{n-1} || answer || bond || answerer || is_commitment)
//! ```
//!
//! cw-reality ports this with sha256 (cosmwasm-std native) and **includes the
//! bond denom** in the hash input. Without that, an attacker who swaps denoms
//! mid-round can confuse the claim replay. Addresses are canonicalized so the
//! bytes are stable across the prefix.

use cosmwasm_std::{Addr, Api, Binary, StdResult, Uint128};
use sha2::{Digest, Sha256};

/// 32-byte history-hash digest. Zero hash marks the empty chain (genesis).
pub type HistoryHash = [u8; 32];

pub const NULL_HISTORY_HASH: HistoryHash = [0u8; 32];

/// Compute the next history-hash step.
///
/// Inputs are concatenated big-endian:
///   `prev_hash || answer_bytes || denom_len_be || denom_bytes || amount_be_bytes || answerer_canonical || is_commitment`
pub fn next_history_hash(
    api: &dyn Api,
    prev_hash: &HistoryHash,
    answer: &Binary,
    bond_denom: &str,
    bond_amount: Uint128,
    answerer: &Addr,
    is_commitment: bool,
) -> StdResult<HistoryHash> {
    let answerer_canonical = api.addr_canonicalize(answerer.as_str())?;

    let mut hasher = Sha256::new();
    hasher.update(prev_hash);
    hasher.update(answer.as_slice());
    hasher.update((bond_denom.len() as u32).to_be_bytes());
    hasher.update(bond_denom.as_bytes());
    hasher.update(bond_amount.u128().to_be_bytes());
    hasher.update(answerer_canonical.as_slice());
    hasher.update([u8::from(is_commitment)]);

    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    Ok(out)
}
