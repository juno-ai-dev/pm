//! Property tests for bond accounting invariants.
//!
//! Astroport's `pair_concentrated` is the in-repo proptest model
//! (`research-notes-cw-prior-art.md` §5). cw-reality's natural invariants:
//!
//! 1. **Bond strictly doubles** on every successful escalation.
//! 2. **Claim conservation**: `sum_of_credits + burn == bounty + sum_of_bonds`
//!    where `burn` is the 2.5% interior shave.
//! 3. **Per-claim determinism** (FM-3): claiming N entries at once produces
//!    the same final balances as claiming them one-by-one across N calls.
//! 4. **`shave_interior_bond` invariant**: shaved amount equals
//!    `bond - bond/40`.

use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{coins, Binary, Uint128};
use proptest::prelude::*;

use crate::contract::{execute, instantiate};
use crate::escalation::shave_interior_bond;
use crate::hash::{next_history_hash, HistoryHash, NULL_HISTORY_HASH};
use crate::msg::{ExecuteMsg, HistoryEntry, InstantiateMsg};
use crate::state::{AnswerType, BALANCES, QUESTIONS};

const DAY: u32 = 24 * 60 * 60;

/// Build an in-memory chain of rounds + the expected on-chain history hash.
fn submit_rounds_and_build_history(
    deps: &mut cosmwasm_std::OwnedDeps<
        cosmwasm_std::MemoryStorage,
        cosmwasm_std::testing::MockApi,
        cosmwasm_std::testing::MockQuerier,
    >,
    rounds: &[(String, Vec<u8>, u128)],
) -> (Vec<u8>, Vec<HistoryEntry>, HistoryHash) {
    let env = mock_env();
    // Each rounds.0 is the answerer name.
    // First we need an asker who funds the bounty. Use a fixed bounty so
    // accounting checks are deterministic.
    let res = execute(
        deps.as_mut(),
        env.clone(),
        mock_info("asker", &coins(100, "ujuno")),
        ExecuteMsg::AskQuestion {
            text: "prop".to_string(),
            answer_type: AnswerType::Bool,
            bond_denom: "ujuno".to_string(),
            initial_bond: Uint128::from(rounds[0].2),
            answer_timeout_secs: DAY,
            arbitrator: None,
            arbitration_timeout_secs: None,
            answer_schema: None,
            opening_ts: None,
            nonce: 0,
        },
    )
    .unwrap();
    let qid = hex::decode(
        &res.attributes
            .iter()
            .find(|a| a.key == "question_id")
            .unwrap()
            .value,
    )
    .unwrap();

    for (answerer, answer, bond) in rounds {
        execute(
            deps.as_mut(),
            env.clone(),
            mock_info(answerer, &coins(*bond, "ujuno")),
            ExecuteMsg::SubmitAnswer {
                question_id: Binary::from(qid.clone()),
                answer: Binary::from(answer.clone()),
                current_bond_seen: None,
            },
        )
        .unwrap();
    }

    // Rebuild the chain off-band.
    let mut chain = NULL_HISTORY_HASH;
    let mut entries_oldest: Vec<HistoryEntry> = Vec::new();
    for (answerer, answer, bond) in rounds {
        let prev = chain;
        let answerer_addr = cosmwasm_std::Addr::unchecked(answerer.as_str());
        chain = next_history_hash(
            &deps.api,
            &prev,
            &Binary::from(answer.clone()),
            "ujuno",
            Uint128::from(*bond),
            &answerer_addr,
            false,
        )
        .unwrap();
        entries_oldest.push(HistoryEntry {
            prev_hash: Binary::from(prev.to_vec()),
            answer: Binary::from(answer.clone()),
            bond_amount: Uint128::from(*bond),
            answerer: answerer.clone(),
            is_commitment: false,
        });
    }
    let entries_newest: Vec<HistoryEntry> = entries_oldest.into_iter().rev().collect();
    (qid, entries_newest, chain)
}

fn fresh_deps_with_min_floor_one() -> cosmwasm_std::OwnedDeps<
    cosmwasm_std::MemoryStorage,
    cosmwasm_std::testing::MockApi,
    cosmwasm_std::testing::MockQuerier,
> {
    let mut deps = mock_dependencies();
    instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info("creator", &[]),
        InstantiateMsg {
            admin: None,
            min_initial_bond_floor: Uint128::one(),
            min_answer_timeout_secs: DAY,
        },
    )
    .unwrap();
    deps
}

/// Strategy: pick a number of rounds in [2, 10], a starting bond, and a
/// boolean per round indicating whether the new answer is the same (right)
/// or different (wrong) than the previous winning answer.
fn dispute_strategy() -> impl Strategy<Value = (u32, u128, Vec<bool>)> {
    (2u32..=10u32, 1u128..=1_000u128).prop_flat_map(|(rounds, initial_bond)| {
        proptest::collection::vec(proptest::bool::ANY, rounds as usize)
            .prop_map(move |flips| (rounds, initial_bond, flips))
    })
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        max_global_rejects: 4096,
        .. ProptestConfig::default()
    })]

    /// Conservation: bounty + sum_of_bonds == sum_of_credits + burn
    #[test]
    fn claim_conserves_bond_total((num_rounds, initial_bond, flips) in dispute_strategy()) {
        let mut deps = fresh_deps_with_min_floor_one();
        // Build the round sequence.
        let mut rounds: Vec<(String, Vec<u8>, u128)> = Vec::with_capacity(num_rounds as usize);
        let mut bond = initial_bond;
        // Each round alternates a fresh answerer; answer toggles based on flips.
        // Answer "right" = 1, "wrong" = 2.
        let mut current_answer_byte: u8 = 1;
        for (i, flip) in flips.iter().enumerate() {
            if *flip { current_answer_byte = if current_answer_byte == 1 { 2 } else { 1 }; }
            rounds.push((
                format!("addr{i}"),
                vec![current_answer_byte; 32],
                bond,
            ));
            bond = bond.checked_mul(2).unwrap_or(u128::MAX);
        }

        let (qid, entries, _) = submit_rounds_and_build_history(&mut deps, &rounds);
        let mut env = mock_env();
        env.block.time = env.block.time.plus_seconds(u64::from(DAY) + 1);
        execute(
            deps.as_mut(),
            env,
            mock_info("claimer", &[]),
            ExecuteMsg::Claim {
                question_id: Binary::from(qid.clone()),
                history_entries: entries,
            },
        ).unwrap();

        // Sum credits across all addresses involved.
        let mut total_credits = Uint128::zero();
        let mut seen: std::collections::HashSet<String> = Default::default();
        for (a, _, _) in &rounds { seen.insert(a.clone()); }
        for who in seen {
            let addr = cosmwasm_std::Addr::unchecked(&who);
            let bal: Uint128 = BALANCES.may_load(&deps.storage, (&addr, "ujuno")).unwrap().unwrap_or_default();
            total_credits = total_credits.checked_add(bal).unwrap();
        }
        // Plus any bounty refund (Reality.eth does not refund bounty if no
        // right answer — drains to burn). The asker's address can also be
        // credited if the asker answered.

        let total_bonds: u128 = rounds.iter().map(|(_, _, b)| *b).sum();
        let bounty = 100u128;
        let in_total = Uint128::from(total_bonds + bounty);

        // Burn = the 2.5% interior shave. Reconstruct expected burn:
        // every interior bond (i.e. every round bond except the chain-tip
        // which equals question.current_bond) is shaved by bond/40.
        // The chain-tip is the last round.
        let burn: u128 = rounds.iter().enumerate().map(|(i, (_, _, b))| {
            if i + 1 == rounds.len() { 0 } else { b / 40 }
        }).sum();

        // If no right answer exists at the end (i.e. final answer's byte
        // pattern never matched any later answer), payee may still be set —
        // actually our model: winning answer = last submitted. So one
        // payee always exists.
        // Validate: in_total == out_total + burn.
        // But: there's a subtle case where bonds don't get credited if no
        // earlier right-answerer takeover happens — e.g. if all interior
        // wrong-bonds go to the chain-tip payee.
        let expected_out = in_total - Uint128::from(burn);
        prop_assert_eq!(total_credits, expected_out,
            "rounds={:?} burn={} expected_out={} total_credits={}",
            rounds, burn, expected_out, total_credits
        );

        let _q = QUESTIONS.load(&deps.storage, qid.as_slice()).unwrap();
    }

    /// Per-claim determinism (FM-3): split == whole.
    #[test]
    fn claim_partial_equals_full((num_rounds, initial_bond, flips) in dispute_strategy()) {
        // Build both deps; reuse same input.
        let mut deps_a = fresh_deps_with_min_floor_one();
        let mut deps_b = fresh_deps_with_min_floor_one();
        let mut rounds: Vec<(String, Vec<u8>, u128)> = Vec::with_capacity(num_rounds as usize);
        let mut bond = initial_bond;
        let mut current_answer_byte: u8 = 1;
        for (i, flip) in flips.iter().enumerate() {
            if *flip { current_answer_byte = if current_answer_byte == 1 { 2 } else { 1 }; }
            rounds.push((format!("addr{i}"), vec![current_answer_byte; 32], bond));
            bond = bond.checked_mul(2).unwrap_or(u128::MAX);
        }
        let (qid_a, entries_a, _) = submit_rounds_and_build_history(&mut deps_a, &rounds);
        let (qid_b, entries_b, _) = submit_rounds_and_build_history(&mut deps_b, &rounds);

        let mut env = mock_env();
        env.block.time = env.block.time.plus_seconds(u64::from(DAY) + 1);

        // A: one-by-one.
        for entry in entries_a {
            execute(deps_a.as_mut(), env.clone(), mock_info("claimer", &[]),
                ExecuteMsg::Claim {
                    question_id: Binary::from(qid_a.clone()),
                    history_entries: vec![entry],
                }
            ).unwrap();
        }
        // B: all at once.
        execute(deps_b.as_mut(), env, mock_info("claimer", &[]),
            ExecuteMsg::Claim {
                question_id: Binary::from(qid_b.clone()),
                history_entries: entries_b,
            }
        ).unwrap();

        let mut seen: std::collections::HashSet<String> = Default::default();
        for (a, _, _) in &rounds { seen.insert(a.clone()); }
        for who in seen {
            let addr = cosmwasm_std::Addr::unchecked(&who);
            let ba: Uint128 = BALANCES.may_load(&deps_a.storage, (&addr, "ujuno")).unwrap().unwrap_or_default();
            let bb: Uint128 = BALANCES.may_load(&deps_b.storage, (&addr, "ujuno")).unwrap().unwrap_or_default();
            prop_assert_eq!(ba, bb, "{} balances differ split vs whole: {} vs {}", who, ba, bb);
        }
    }

    /// `shave_interior_bond` invariant.
    #[test]
    fn shave_is_bond_minus_fortieth(bond in 0u128..=u128::MAX/2) {
        let b = Uint128::from(bond);
        let shaved = shave_interior_bond(b);
        prop_assert_eq!(shaved, b - b.multiply_ratio(1u128, 40u128));
        prop_assert!(shaved <= b);
        prop_assert!(b - shaved <= b.multiply_ratio(1u128, 40u128));
    }
}
