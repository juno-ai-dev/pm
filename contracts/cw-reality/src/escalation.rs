//! Bond-escalation constants and arithmetic helpers.
//!
//! Reality.eth precedent (`RealityETH-3.0.sol`): every new answer must bond
//! at least 2× the previous answer. Every interior bond is taxed 2.5% on claim;
//! the chain-tip winning bond is exempt. We port the rules literally — see
//! `docs/reality-eth-lessons.md` §3 for the worked example that balances.

use cosmwasm_std::Uint128;

/// Strict bond-doubling multiplier (`bond_new >= 2 * bond_prev`).
pub const BOND_MULTIPLIER: u128 = 2;

/// Interior-bond claim-fee denominator. `bond - bond/40 == bond * 0.975`.
/// Chain-tip (winning) bond is exempt.
pub const BOND_CLAIM_FEE_DENOM: u128 = 40;

/// Hard cap on dispute escalation rounds. At 2× per round starting from
/// `1 ujuno`, round 32 ≈ 4 G JUNO — adequate headroom and bounds proptest budget.
pub const MAX_DISPUTE_ROUNDS: u32 = 32;

/// Default answer timeout (24h) and platform floor (24h) per lessons §5.
pub const DEFAULT_ANSWER_TIMEOUT_SECS: u32 = 24 * 60 * 60;
pub const MIN_ANSWER_TIMEOUT_SECS_FLOOR: u32 = 60 * 60; // 1h — extreme floor; deployments should set 24h

/// Default arbitration request timeout (7 days) per lessons §5.
pub const DEFAULT_ARBITRATION_TIMEOUT_SECS: u32 = 7 * 24 * 60 * 60;

/// Hard ceiling on `answer_timeout_secs` (365 days, matching Reality.eth).
pub const MAX_ANSWER_TIMEOUT_SECS: u32 = 365 * 24 * 60 * 60;

/// Apply the 2.5% interior shave used during claim.
///
/// Matches Reality.eth's integer math: `bond - bond / 40`. For bonds below 40
/// units the shave rounds to zero — this is the published Reality.eth behavior.
pub fn shave_interior_bond(bond: Uint128) -> Uint128 {
    bond - bond.multiply_ratio(1u128, BOND_CLAIM_FEE_DENOM)
}

/// Returns true if `new_bond` satisfies the strict 2× rule against `prev_bond`.
pub fn satisfies_doubling(prev_bond: Uint128, new_bond: Uint128) -> bool {
    match prev_bond.checked_mul(Uint128::from(BOND_MULTIPLIER)) {
        Ok(required) => new_bond >= required,
        Err(_) => false, // overflow at the doubling cap — round limit will reject upstream
    }
}
