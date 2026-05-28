//! Content-derived question-id derivation.
//!
//! Reality.eth (`RealityETH-3.0.sol:325`):
//!
//! ```text
//! question_id = keccak256(content_hash, arbitrator, timeout, min_bond,
//!                         address(this), msg.sender, nonce)
//! ```
//!
//! cw-reality follows the same template. Including the contract address is
//! the cross-deployment-collision defense added in v3 (FM-5 from
//! `docs/reality-eth-lessons.md` §6); we keep it even though CosmWasm contract
//! addresses are already globally unique so future inter-chain integrations
//! cannot blur question identity.

use cosmwasm_std::{Addr, Api, StdResult, Uint128};
use sha2::{Digest, Sha256};

/// Compute the content-hash of a question's text — separate from the
/// question_id so off-chain consumers can cheaply recompute it.
pub fn content_hash(text: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

/// Compute the question_id binding the asker, contract address, and the
/// canonical question parameters.
#[allow(clippy::too_many_arguments)]
pub fn question_id(
    api: &dyn Api,
    contract: &Addr,
    asker: &Addr,
    nonce: u64,
    content_hash: &[u8; 32],
    arbitrator: Option<&Addr>,
    answer_timeout_secs: u32,
    initial_bond: Uint128,
    bond_denom: &str,
    opening_ts: Option<u64>,
) -> StdResult<[u8; 32]> {
    let contract_c = api.addr_canonicalize(contract.as_str())?;
    let asker_c = api.addr_canonicalize(asker.as_str())?;

    let mut hasher = Sha256::new();
    hasher.update(contract_c.as_slice());
    hasher.update(asker_c.as_slice());
    hasher.update(nonce.to_be_bytes());
    hasher.update(content_hash);

    match arbitrator {
        Some(a) => {
            let a_c = api.addr_canonicalize(a.as_str())?;
            hasher.update([1u8]); // tag: arbitrator-present
            hasher.update(a_c.as_slice());
        }
        None => {
            hasher.update([0u8]); // tag: no arbitrator
        }
    }

    hasher.update(answer_timeout_secs.to_be_bytes());
    hasher.update(initial_bond.u128().to_be_bytes());
    hasher.update((bond_denom.len() as u32).to_be_bytes());
    hasher.update(bond_denom.as_bytes());
    hasher.update(opening_ts.unwrap_or(0).to_be_bytes());

    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    Ok(out)
}
