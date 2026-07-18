//! Unit tests live alongside their handler modules; this file holds the
//! cross-cutting fixtures and the instantiate sanity test plus the
//! `AskQuestion` / `FundBounty` happy-paths and rejections.

use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{coins, Binary, Uint128};

use crate::contract::{execute, instantiate};
use crate::error::ContractError;
use crate::escalation::{
    MAX_ANSWER_TIMEOUT_SECS, MAX_DISPUTE_ROUNDS, MIN_ANSWER_TIMEOUT_SECS_FLOOR,
};
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{AnswerType, State, QUESTIONS};

#[test]
fn instantiate_happy_path() {
    let mut deps = mock_dependencies();
    let info = mock_info("creator", &[]);
    let env = mock_env();
    let msg = InstantiateMsg {
        admin: None,
        min_initial_bond_floor: Uint128::from(100_000u128),
        min_answer_timeout_secs: 24 * 60 * 60,
    };
    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert!(res
        .attributes
        .iter()
        .any(|a| a.key == "action" && a.value == "instantiate"));
}

#[test]
fn instantiate_rejects_zero_floor() {
    let mut deps = mock_dependencies();
    let info = mock_info("creator", &[]);
    let env = mock_env();
    let msg = InstantiateMsg {
        admin: None,
        min_initial_bond_floor: Uint128::zero(),
        min_answer_timeout_secs: 24 * 60 * 60,
    };
    let err = instantiate(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::ZeroMinInitialBondFloor {});
}

#[test]
fn instantiate_rejects_low_timeout_floor() {
    let mut deps = mock_dependencies();
    let info = mock_info("creator", &[]);
    let env = mock_env();
    let msg = InstantiateMsg {
        admin: None,
        min_initial_bond_floor: Uint128::from(100u128),
        min_answer_timeout_secs: MIN_ANSWER_TIMEOUT_SECS_FLOOR - 1,
    };
    let err = instantiate(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::MinAnswerTimeoutTooLow {});
}

// ---- AskQuestion ----

const MIN_FLOOR: u128 = 100_000;
const DAY: u32 = 24 * 60 * 60;

fn setup() -> cosmwasm_std::OwnedDeps<
    cosmwasm_std::MemoryStorage,
    cosmwasm_std::testing::MockApi,
    cosmwasm_std::testing::MockQuerier,
> {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("creator", &[]);
    instantiate(
        deps.as_mut(),
        env,
        info,
        InstantiateMsg {
            admin: None,
            min_initial_bond_floor: Uint128::from(MIN_FLOOR),
            min_answer_timeout_secs: DAY,
        },
    )
    .unwrap();
    deps
}

fn ask_msg(initial_bond: u128, timeout: u32, nonce: u64) -> ExecuteMsg {
    ExecuteMsg::AskQuestion {
        text: "Did event X happen?".to_string(),
        answer_type: AnswerType::Bool,
        bond_denom: "ujuno".to_string(),
        initial_bond: Uint128::from(initial_bond),
        answer_timeout_secs: timeout,
        arbitrator: None,
        arbitration_timeout_secs: None,
        answer_schema: None,
        opening_ts: None,
        nonce,
    }
}

#[test]
fn ask_question_happy_path_no_bounty() {
    let mut deps = setup();
    let env = mock_env();
    let info = mock_info("alice", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, ask_msg(MIN_FLOOR, DAY, 0)).unwrap();
    let qid_attr = res
        .attributes
        .iter()
        .find(|a| a.key == "question_id")
        .unwrap();
    let qid: [u8; 32] = hex::decode(&qid_attr.value).unwrap().try_into().unwrap();
    let q = QUESTIONS.load(&deps.storage, &qid).unwrap();
    assert_eq!(q.bond_denom, "ujuno");
    assert_eq!(q.initial_bond, Uint128::from(MIN_FLOOR));
    assert_eq!(q.bounty, Uint128::zero());
    assert_eq!(q.round_count, 0);
    assert_eq!(q.state_at(env.block.time.seconds()), State::OpenUnanswered);
}

#[test]
fn ask_question_happy_path_with_bounty() {
    let mut deps = setup();
    let env = mock_env();
    let info = mock_info("alice", &coins(500, "ujuno"));
    let res = execute(deps.as_mut(), env, info, ask_msg(MIN_FLOOR, DAY, 0)).unwrap();
    let qid_attr = res
        .attributes
        .iter()
        .find(|a| a.key == "question_id")
        .unwrap();
    let qid: [u8; 32] = hex::decode(&qid_attr.value).unwrap().try_into().unwrap();
    let q = QUESTIONS.load(&deps.storage, &qid).unwrap();
    assert_eq!(q.bounty, Uint128::from(500u128));
}

#[test]
fn ask_question_rejects_below_initial_bond_floor() {
    let mut deps = setup();
    let env = mock_env();
    let info = mock_info("alice", &[]);
    let err = execute(deps.as_mut(), env, info, ask_msg(MIN_FLOOR - 1, DAY, 0)).unwrap_err();
    assert!(matches!(err, ContractError::InitialBondBelowFloor { .. }));
}

#[test]
fn ask_question_rejects_below_timeout_floor() {
    let mut deps = setup();
    let env = mock_env();
    let info = mock_info("alice", &[]);
    let err = execute(deps.as_mut(), env, info, ask_msg(MIN_FLOOR, DAY - 1, 0)).unwrap_err();
    assert!(matches!(err, ContractError::AnswerTimeoutBelowFloor { .. }));
}

#[test]
fn ask_question_rejects_above_timeout_max() {
    let mut deps = setup();
    let env = mock_env();
    let info = mock_info("alice", &[]);
    let err = execute(
        deps.as_mut(),
        env,
        info,
        ask_msg(MIN_FLOOR, MAX_ANSWER_TIMEOUT_SECS + 1, 0),
    )
    .unwrap_err();
    assert_eq!(err, ContractError::AnswerTimeoutTooHigh {});
}

#[test]
fn ask_question_rejects_wrong_denom_bounty() {
    let mut deps = setup();
    let env = mock_env();
    let info = mock_info("alice", &coins(100, "uatom"));
    let err = execute(deps.as_mut(), env, info, ask_msg(MIN_FLOOR, DAY, 0)).unwrap_err();
    assert!(matches!(err, ContractError::BondDenomMismatch { .. }));
}

#[test]
fn ask_question_rejects_multi_denom_bounty() {
    let mut deps = setup();
    let env = mock_env();
    let info = mock_info(
        "alice",
        &[
            cosmwasm_std::Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(100u128),
            },
            cosmwasm_std::Coin {
                denom: "uatom".to_string(),
                amount: Uint128::from(50u128),
            },
        ],
    );
    let err = execute(deps.as_mut(), env, info, ask_msg(MIN_FLOOR, DAY, 0)).unwrap_err();
    assert!(matches!(err, ContractError::InvalidBondFunds { count: 2 }));
}

#[test]
fn ask_question_nonce_collision_rejected() {
    let mut deps = setup();
    let env = mock_env();
    let info = mock_info("alice", &[]);
    execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        ask_msg(MIN_FLOOR, DAY, 7),
    )
    .unwrap();
    let err = execute(deps.as_mut(), env, info, ask_msg(MIN_FLOOR, DAY, 7)).unwrap_err();
    assert!(matches!(err, ContractError::QuestionAlreadyExists { .. }));
}

#[test]
fn ask_question_different_askers_can_share_nonce() {
    let mut deps = setup();
    let env = mock_env();
    let alice = mock_info("alice", &[]);
    let bob = mock_info("bob", &[]);
    execute(
        deps.as_mut(),
        env.clone(),
        alice,
        ask_msg(MIN_FLOOR, DAY, 0),
    )
    .unwrap();
    // Same nonce, different sender → different question_id.
    execute(deps.as_mut(), env, bob, ask_msg(MIN_FLOOR, DAY, 0)).unwrap();
}

#[test]
fn ask_question_invalid_arbitrator_rejected() {
    let mut deps = setup();
    let env = mock_env();
    let info = mock_info("alice", &[]);
    let mut msg = ask_msg(MIN_FLOOR, DAY, 0);
    if let ExecuteMsg::AskQuestion { arbitrator, .. } = &mut msg {
        // mock_dependencies' MockApi validator rejects bech32-invalid strings
        // and also flags excessively long strings — "" should hit the empty-addr path.
        *arbitrator = Some("".to_string());
    }
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    // MockApi reports `StdError::GenericErr` for an empty address.
    assert!(matches!(err, ContractError::Std(_)));
}

// ---- FundBounty ----

#[test]
fn fund_bounty_happy_path() {
    let mut deps = setup();
    let env = mock_env();
    let alice = mock_info("alice", &[]);
    let res = execute(
        deps.as_mut(),
        env.clone(),
        alice,
        ask_msg(MIN_FLOOR, DAY, 0),
    )
    .unwrap();
    let qid_attr = res
        .attributes
        .iter()
        .find(|a| a.key == "question_id")
        .unwrap();
    let qid_bytes = hex::decode(&qid_attr.value).unwrap();

    let funder = mock_info("patron", &coins(1_000, "ujuno"));
    execute(
        deps.as_mut(),
        env,
        funder,
        ExecuteMsg::FundBounty {
            question_id: Binary::from(qid_bytes.clone()),
        },
    )
    .unwrap();

    let q = QUESTIONS.load(&deps.storage, qid_bytes.as_slice()).unwrap();
    assert_eq!(q.bounty, Uint128::from(1_000u128));
}

#[test]
fn fund_bounty_rejects_wrong_denom() {
    let mut deps = setup();
    let env = mock_env();
    let alice = mock_info("alice", &[]);
    let res = execute(
        deps.as_mut(),
        env.clone(),
        alice,
        ask_msg(MIN_FLOOR, DAY, 0),
    )
    .unwrap();
    let qid_attr = res
        .attributes
        .iter()
        .find(|a| a.key == "question_id")
        .unwrap();
    let qid_bytes = hex::decode(&qid_attr.value).unwrap();

    let funder = mock_info("patron", &coins(100, "uatom"));
    let err = execute(
        deps.as_mut(),
        env,
        funder,
        ExecuteMsg::FundBounty {
            question_id: Binary::from(qid_bytes),
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::BondDenomMismatch { .. }));
}

#[test]
fn fund_bounty_rejects_zero_amount() {
    let mut deps = setup();
    let env = mock_env();
    let alice = mock_info("alice", &[]);
    let res = execute(
        deps.as_mut(),
        env.clone(),
        alice,
        ask_msg(MIN_FLOOR, DAY, 0),
    )
    .unwrap();
    let qid_attr = res
        .attributes
        .iter()
        .find(|a| a.key == "question_id")
        .unwrap();
    let qid_bytes = hex::decode(&qid_attr.value).unwrap();

    let funder = mock_info("patron", &coins(0, "ujuno"));
    let err = execute(
        deps.as_mut(),
        env,
        funder,
        ExecuteMsg::FundBounty {
            question_id: Binary::from(qid_bytes),
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Std(_)));
}

#[test]
fn fund_bounty_unknown_question_rejected() {
    let mut deps = setup();
    let env = mock_env();
    let funder = mock_info("patron", &coins(100, "ujuno"));
    let err = execute(
        deps.as_mut(),
        env,
        funder,
        ExecuteMsg::FundBounty {
            question_id: Binary::from(vec![0u8; 32]),
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::QuestionNotFound { .. }));
}

// ---- SubmitAnswer / DisputeAnswer ----

fn ask_question(
    deps: &mut cosmwasm_std::OwnedDeps<
        cosmwasm_std::MemoryStorage,
        cosmwasm_std::testing::MockApi,
        cosmwasm_std::testing::MockQuerier,
    >,
    asker: &str,
    initial_bond: u128,
    timeout: u32,
    nonce: u64,
) -> Vec<u8> {
    let env = mock_env();
    let info = mock_info(asker, &[]);
    let res = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::AskQuestion {
            text: format!("Question {nonce}"),
            answer_type: AnswerType::Bool,
            bond_denom: "ujuno".to_string(),
            initial_bond: Uint128::from(initial_bond),
            answer_timeout_secs: timeout,
            arbitrator: None,
            arbitration_timeout_secs: None,
            answer_schema: None,
            opening_ts: None,
            nonce,
        },
    )
    .unwrap();
    hex::decode(
        &res.attributes
            .iter()
            .find(|a| a.key == "question_id")
            .unwrap()
            .value,
    )
    .unwrap()
}

#[test]
fn submit_answer_first_round_happy_path() {
    let mut deps = setup();
    let qid = ask_question(&mut deps, "alice", MIN_FLOOR, DAY, 0);
    let env = mock_env();
    let bob = mock_info("bob", &coins(MIN_FLOOR, "ujuno"));
    let res = execute(
        deps.as_mut(),
        env.clone(),
        bob,
        ExecuteMsg::SubmitAnswer {
            question_id: Binary::from(qid.clone()),
            answer: Binary::from(vec![1u8; 32]),
            current_bond_seen: Some(Uint128::zero()),
        },
    )
    .unwrap();
    assert!(res
        .attributes
        .iter()
        .any(|a| a.key == "action" && a.value == "submit_answer"));

    let q = QUESTIONS.load(&deps.storage, qid.as_slice()).unwrap();
    assert_eq!(q.round_count, 1);
    assert_eq!(q.current_bond, Uint128::from(MIN_FLOOR));
    assert_eq!(
        q.finalize_ts,
        Some(env.block.time.seconds() + u64::from(DAY))
    );
    assert_eq!(q.state_at(env.block.time.seconds()), State::OpenAnswered);
}

#[test]
fn submit_answer_below_min_bond_rejected() {
    let mut deps = setup();
    let qid = ask_question(&mut deps, "alice", MIN_FLOOR, DAY, 0);
    let env = mock_env();
    let bob = mock_info("bob", &coins(MIN_FLOOR - 1, "ujuno"));
    let err = execute(
        deps.as_mut(),
        env,
        bob,
        ExecuteMsg::SubmitAnswer {
            question_id: Binary::from(qid),
            answer: Binary::from(vec![1u8; 32]),
            current_bond_seen: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::BondBelowMinimum { .. }));
}

#[test]
fn dispute_answer_doubling_happy_path() {
    let mut deps = setup();
    let qid = ask_question(&mut deps, "alice", MIN_FLOOR, DAY, 0);
    let env = mock_env();

    execute(
        deps.as_mut(),
        env.clone(),
        mock_info("bob", &coins(MIN_FLOOR, "ujuno")),
        ExecuteMsg::SubmitAnswer {
            question_id: Binary::from(qid.clone()),
            answer: Binary::from(vec![1u8; 32]),
            current_bond_seen: None,
        },
    )
    .unwrap();

    execute(
        deps.as_mut(),
        env.clone(),
        mock_info("carol", &coins(MIN_FLOOR * 2, "ujuno")),
        ExecuteMsg::DisputeAnswer {
            question_id: Binary::from(qid.clone()),
            new_answer: Binary::from(vec![2u8; 32]),
            current_bond_seen: Some(Uint128::from(MIN_FLOOR)),
        },
    )
    .unwrap();

    let q = QUESTIONS.load(&deps.storage, qid.as_slice()).unwrap();
    assert_eq!(q.round_count, 2);
    assert_eq!(q.current_bond, Uint128::from(MIN_FLOOR * 2));
}

#[test]
fn dispute_answer_below_double_rejected() {
    let mut deps = setup();
    let qid = ask_question(&mut deps, "alice", MIN_FLOOR, DAY, 0);
    let env = mock_env();
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info("bob", &coins(MIN_FLOOR, "ujuno")),
        ExecuteMsg::SubmitAnswer {
            question_id: Binary::from(qid.clone()),
            answer: Binary::from(vec![1u8; 32]),
            current_bond_seen: None,
        },
    )
    .unwrap();
    let err = execute(
        deps.as_mut(),
        env,
        mock_info("carol", &coins(MIN_FLOOR * 2 - 1, "ujuno")),
        ExecuteMsg::DisputeAnswer {
            question_id: Binary::from(qid),
            new_answer: Binary::from(vec![2u8; 32]),
            current_bond_seen: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::BondMustDouble { .. }));
}

#[test]
fn answer_front_run_guard_trips() {
    let mut deps = setup();
    let qid = ask_question(&mut deps, "alice", MIN_FLOOR, DAY, 0);
    let env = mock_env();
    // First answerer lands a higher-than-expected bond.
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info("bob", &coins(MIN_FLOOR * 2, "ujuno")),
        ExecuteMsg::SubmitAnswer {
            question_id: Binary::from(qid.clone()),
            answer: Binary::from(vec![1u8; 32]),
            current_bond_seen: None,
        },
    )
    .unwrap();
    // Carol expected to see 0 bond but a front-running bob is at 2 * MIN_FLOOR.
    let err = execute(
        deps.as_mut(),
        env,
        mock_info("carol", &coins(MIN_FLOOR * 4, "ujuno")),
        ExecuteMsg::DisputeAnswer {
            question_id: Binary::from(qid),
            new_answer: Binary::from(vec![2u8; 32]),
            current_bond_seen: Some(Uint128::zero()),
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::BondExceedsExpected { .. }));
}

#[test]
fn answer_wrong_denom_rejected() {
    let mut deps = setup();
    let qid = ask_question(&mut deps, "alice", MIN_FLOOR, DAY, 0);
    let env = mock_env();
    let err = execute(
        deps.as_mut(),
        env,
        mock_info("bob", &coins(MIN_FLOOR, "uatom")),
        ExecuteMsg::SubmitAnswer {
            question_id: Binary::from(qid),
            answer: Binary::from(vec![1u8; 32]),
            current_bond_seen: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::BondDenomMismatch { .. }));
}

#[test]
fn answer_zero_bond_rejected() {
    let mut deps = setup();
    let qid = ask_question(&mut deps, "alice", MIN_FLOOR, DAY, 0);
    let env = mock_env();
    let err = execute(
        deps.as_mut(),
        env,
        mock_info("bob", &coins(0, "ujuno")),
        ExecuteMsg::SubmitAnswer {
            question_id: Binary::from(qid),
            answer: Binary::from(vec![1u8; 32]),
            current_bond_seen: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Std(_)));
}

#[test]
fn answer_opening_ts_not_yet_rejected() {
    let mut deps = setup();
    let env = mock_env();
    let now = env.block.time.seconds();
    let alice = mock_info("alice", &[]);
    let res = execute(
        deps.as_mut(),
        env.clone(),
        alice,
        ExecuteMsg::AskQuestion {
            text: "deferred".to_string(),
            answer_type: AnswerType::Bool,
            bond_denom: "ujuno".to_string(),
            initial_bond: Uint128::from(MIN_FLOOR),
            answer_timeout_secs: DAY,
            arbitrator: None,
            arbitration_timeout_secs: None,
            answer_schema: None,
            opening_ts: Some(now + 100),
            nonce: 42,
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
    let err = execute(
        deps.as_mut(),
        env,
        mock_info("bob", &coins(MIN_FLOOR, "ujuno")),
        ExecuteMsg::SubmitAnswer {
            question_id: Binary::from(qid),
            answer: Binary::from(vec![1u8; 32]),
            current_bond_seen: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::InvalidState { .. }));
}

#[test]
fn answer_unknown_question_rejected() {
    let mut deps = setup();
    let env = mock_env();
    let err = execute(
        deps.as_mut(),
        env,
        mock_info("bob", &coins(MIN_FLOOR, "ujuno")),
        ExecuteMsg::SubmitAnswer {
            question_id: Binary::from(vec![0u8; 32]),
            answer: Binary::from(vec![1u8; 32]),
            current_bond_seen: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::QuestionNotFound { .. }));
}

// ---- Claim + Withdraw ----

use crate::hash::{next_history_hash, NULL_HISTORY_HASH};
use crate::msg::HistoryEntry;
use crate::state::BALANCES;

/// Build the on-chain history hash chain for a sequence of (answerer, answer,
/// bond) tuples, in submission order (oldest-first). Returns the final hash
/// (chain tip) plus the newest-first list of `HistoryEntry` shapes the
/// claimer would submit.
fn build_history_chain(
    api: &cosmwasm_std::testing::MockApi,
    denom: &str,
    rounds: &[(&str, &[u8], u128)],
) -> (crate::hash::HistoryHash, Vec<HistoryEntry>) {
    let mut chain = NULL_HISTORY_HASH;
    let mut entries_oldest_first: Vec<HistoryEntry> = Vec::new();
    for (answerer, answer, bond) in rounds {
        let prev = chain;
        let answerer_addr = cosmwasm_std::Addr::unchecked(*answerer);
        chain = next_history_hash(
            api,
            &prev,
            &Binary::from(*answer),
            denom,
            Uint128::from(*bond),
            &answerer_addr,
            false,
        )
        .unwrap();
        entries_oldest_first.push(HistoryEntry {
            prev_hash: Binary::from(prev.to_vec()),
            answer: Binary::from(*answer),
            bond_amount: Uint128::from(*bond),
            answerer: answerer.to_string(),
            is_commitment: false,
        });
    }
    let entries_newest_first: Vec<HistoryEntry> = entries_oldest_first.into_iter().rev().collect();
    (chain, entries_newest_first)
}

/// Reconstruct the 3-round Alice/Bob/Carol worked example from
/// `docs/reality-eth-lessons.md` §3 directly in test state.
fn setup_3_round_dispute() -> (
    cosmwasm_std::OwnedDeps<
        cosmwasm_std::MemoryStorage,
        cosmwasm_std::testing::MockApi,
        cosmwasm_std::testing::MockQuerier,
    >,
    Vec<u8>,
    Vec<HistoryEntry>,
) {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate(
        deps.as_mut(),
        env.clone(),
        mock_info("creator", &[]),
        InstantiateMsg {
            admin: None,
            min_initial_bond_floor: Uint128::one(),
            min_answer_timeout_secs: DAY,
        },
    )
    .unwrap();
    // Alice asks with bounty=100 (asker funds via info.funds at ask time).
    let res = execute(
        deps.as_mut(),
        env.clone(),
        mock_info("alice", &coins(100, "ujuno")),
        ExecuteMsg::AskQuestion {
            text: "Did event X happen?".to_string(),
            answer_type: AnswerType::Bool,
            bond_denom: "ujuno".to_string(),
            initial_bond: Uint128::from(10u128),
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

    // 3 rounds: Alice→A (10), Bob→B (20), Carol→A (40). Carol's answer A
    // becomes the chain-tip = best_answer.
    let answer_a: Vec<u8> = vec![1u8; 32];
    let answer_b: Vec<u8> = vec![2u8; 32];
    let rounds: Vec<(&str, Vec<u8>, u128)> = vec![
        ("alice", answer_a.clone(), 10),
        ("bob", answer_b.clone(), 20),
        ("carol", answer_a.clone(), 40),
    ];
    for (answerer, answer, bond) in &rounds {
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

    // Build the expected history chain off-band to feed to Claim.
    let rounds_ref: Vec<(&str, &[u8], u128)> = rounds
        .iter()
        .map(|(a, ans, b)| (*a, ans.as_slice(), *b))
        .collect();
    let (chain_tip, entries_newest_first) = build_history_chain(&deps.api, "ujuno", &rounds_ref);
    let q = QUESTIONS.load(&deps.storage, qid.as_slice()).unwrap();
    assert_eq!(q.history_hash, chain_tip, "test-built chain matches state");

    (deps, qid, entries_newest_first)
}

#[test]
fn claim_three_round_dispute_balances() {
    let (mut deps, qid, entries) = setup_3_round_dispute();
    // Advance time so the question is Finalized.
    let mut env = mock_env();
    env.block.time = env.block.time.plus_seconds(u64::from(DAY) + 1);
    execute(
        deps.as_mut(),
        env,
        mock_info("anyone", &[]),
        ExecuteMsg::Claim {
            question_id: Binary::from(qid.clone()),
            history_entries: entries,
        },
    )
    .unwrap();

    let q = QUESTIONS.load(&deps.storage, qid.as_slice()).unwrap();
    assert!(q.is_claimed);

    // Expected per lessons §3:
    //   Carol: 100 (bounty) + 50 (takeover residual) = 150
    //   Alice: 10 (own bond) + 10 (takeover_fee) = 20
    //   Bob:   0 (wrong answer)
    let alice = cosmwasm_std::Addr::unchecked("alice");
    let bob = cosmwasm_std::Addr::unchecked("bob");
    let carol = cosmwasm_std::Addr::unchecked("carol");
    let carol_bal: Uint128 = BALANCES
        .may_load(&deps.storage, (&carol, "ujuno"))
        .unwrap()
        .unwrap_or_default();
    let alice_bal: Uint128 = BALANCES
        .may_load(&deps.storage, (&alice, "ujuno"))
        .unwrap()
        .unwrap_or_default();
    let bob_bal: Uint128 = BALANCES
        .may_load(&deps.storage, (&bob, "ujuno"))
        .unwrap()
        .unwrap_or_default();

    assert_eq!(carol_bal, Uint128::from(150u128), "Carol = 150");
    assert_eq!(alice_bal, Uint128::from(20u128), "Alice = 20");
    assert_eq!(bob_bal, Uint128::zero(), "Bob = 0");

    // Conservation: total credits == 100 bounty + 10+20+40 bonds = 170.
    assert_eq!(
        carol_bal + alice_bal + bob_bal,
        Uint128::from(170u128),
        "total payouts conserve bounty + bonds"
    );
}

#[test]
fn claim_round_by_round_equals_all_at_once() {
    // FM-3 — per-claim determinism. Claim entries one at a time vs all at
    // once; final balances must be identical.
    let (mut deps_a, qid_a, entries_a) = setup_3_round_dispute();
    let (mut deps_b, qid_b, entries_b) = setup_3_round_dispute();

    let mut env = mock_env();
    env.block.time = env.block.time.plus_seconds(u64::from(DAY) + 1);

    // Path A: one entry at a time.
    for entry in entries_a {
        execute(
            deps_a.as_mut(),
            env.clone(),
            mock_info("claimer", &[]),
            ExecuteMsg::Claim {
                question_id: Binary::from(qid_a.clone()),
                history_entries: vec![entry],
            },
        )
        .unwrap();
    }

    // Path B: all at once.
    execute(
        deps_b.as_mut(),
        env,
        mock_info("claimer", &[]),
        ExecuteMsg::Claim {
            question_id: Binary::from(qid_b.clone()),
            history_entries: entries_b,
        },
    )
    .unwrap();

    for who in &["alice", "bob", "carol"] {
        let addr = cosmwasm_std::Addr::unchecked(*who);
        let a = BALANCES
            .may_load(&deps_a.storage, (&addr, "ujuno"))
            .unwrap()
            .unwrap_or_default();
        let b = BALANCES
            .may_load(&deps_b.storage, (&addr, "ujuno"))
            .unwrap()
            .unwrap_or_default();
        assert_eq!(a, b, "{who} balances must match across claim shapes");
    }

    let q_a = QUESTIONS.load(&deps_a.storage, qid_a.as_slice()).unwrap();
    let q_b = QUESTIONS.load(&deps_b.storage, qid_b.as_slice()).unwrap();
    assert_eq!(q_a.is_claimed, q_b.is_claimed);
    assert!(q_a.is_claimed);
}

#[test]
fn claim_before_finalize_rejected() {
    let (mut deps, qid, entries) = setup_3_round_dispute();
    let env = mock_env(); // not advanced — still OpenAnswered
    let err = execute(
        deps.as_mut(),
        env,
        mock_info("anyone", &[]),
        ExecuteMsg::Claim {
            question_id: Binary::from(qid),
            history_entries: entries,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::InvalidState { .. }));
}

#[test]
fn claim_bad_history_hash_rejected() {
    let (mut deps, qid, mut entries) = setup_3_round_dispute();
    let mut env = mock_env();
    env.block.time = env.block.time.plus_seconds(u64::from(DAY) + 1);
    // Tamper with the topmost entry's bond amount.
    entries[0].bond_amount = Uint128::from(99u128);
    let err = execute(
        deps.as_mut(),
        env,
        mock_info("anyone", &[]),
        ExecuteMsg::Claim {
            question_id: Binary::from(qid),
            history_entries: entries,
        },
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ContractError::HistoryHashMismatch { step: 0 }
    ));
}

#[test]
fn withdraw_drains_balance() {
    let (mut deps, qid, entries) = setup_3_round_dispute();
    let mut env = mock_env();
    env.block.time = env.block.time.plus_seconds(u64::from(DAY) + 1);
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info("anyone", &[]),
        ExecuteMsg::Claim {
            question_id: Binary::from(qid.clone()),
            history_entries: entries,
        },
    )
    .unwrap();

    let res = execute(
        deps.as_mut(),
        env,
        mock_info("carol", &[]),
        ExecuteMsg::Withdraw {
            denom: "ujuno".to_string(),
        },
    )
    .unwrap();
    // One BankMsg dispatched with carol's 150 ujuno.
    assert_eq!(res.messages.len(), 1);
    match &res.messages[0].msg {
        cosmwasm_std::CosmosMsg::Bank(cosmwasm_std::BankMsg::Send { to_address, amount }) => {
            assert_eq!(to_address, "carol");
            assert_eq!(amount, &coins(150, "ujuno"));
        }
        _ => panic!("expected BankMsg::Send"),
    }
    let carol_bal_after: Option<Uint128> = BALANCES
        .may_load(
            &deps.storage,
            (&cosmwasm_std::Addr::unchecked("carol"), "ujuno"),
        )
        .unwrap();
    assert!(carol_bal_after.is_none(), "balance entry removed");
}

#[test]
fn withdraw_no_balance_rejected() {
    let mut deps = setup();
    let env = mock_env();
    let err = execute(
        deps.as_mut(),
        env,
        mock_info("nobody", &[]),
        ExecuteMsg::Withdraw {
            denom: "ujuno".to_string(),
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::NothingToWithdraw {});
}

// ---- Queries ----

use crate::contract::query as query_entry;
use crate::msg::{
    BalanceResponse, FinalAnswerResponse, QueryMsg, QuestionResponse, QuestionsListResponse,
};

#[test]
fn query_question_returns_state() {
    let mut deps = setup();
    let qid = ask_question(&mut deps, "alice", MIN_FLOOR, DAY, 0);
    let env = mock_env();
    let bin = query_entry(
        deps.as_ref(),
        env.clone(),
        QueryMsg::Question {
            question_id: Binary::from(qid.clone()),
        },
    )
    .unwrap();
    let resp: QuestionResponse = cosmwasm_std::from_json(bin).unwrap();
    assert_eq!(resp.state, State::OpenUnanswered);
}

#[test]
fn query_final_answer_before_finalize_errors() {
    let mut deps = setup();
    let qid = ask_question(&mut deps, "alice", MIN_FLOOR, DAY, 0);
    let env = mock_env();
    let err = query_entry(
        deps.as_ref(),
        env,
        QueryMsg::FinalAnswer {
            question_id: Binary::from(qid),
        },
    )
    .unwrap_err();
    assert!(format!("{err}").contains("not finalized"));
}

#[test]
fn query_final_answer_after_finalize_returns() {
    let (deps, qid, _) = setup_3_round_dispute();
    let mut env = mock_env();
    env.block.time = env.block.time.plus_seconds(u64::from(DAY) + 1);
    let bin = query_entry(
        deps.as_ref(),
        env,
        QueryMsg::FinalAnswer {
            question_id: Binary::from(qid),
        },
    )
    .unwrap();
    let resp: FinalAnswerResponse = cosmwasm_std::from_json(bin).unwrap();
    assert_eq!(resp.final_answer.as_slice(), vec![1u8; 32]);
    assert_eq!(resp.final_bond, Uint128::from(40u128));
}

#[test]
fn query_final_answer_if_matches_passes_when_guarantees_met() {
    let (deps, qid, _) = setup_3_round_dispute();
    let mut env = mock_env();
    env.block.time = env.block.time.plus_seconds(u64::from(DAY) + 1);
    let bin = query_entry(
        deps.as_ref(),
        env,
        QueryMsg::FinalAnswerIfMatches {
            question_id: Binary::from(qid),
            min_bond: Some(Uint128::from(40u128)),
            min_timeout_secs: Some(DAY),
            required_arbitrator: None,
            required_denom: Some("ujuno".to_string()),
        },
    )
    .unwrap();
    let resp: FinalAnswerResponse = cosmwasm_std::from_json(bin).unwrap();
    assert_eq!(resp.final_bond, Uint128::from(40u128));
}

#[test]
fn query_final_answer_if_matches_fails_min_bond() {
    let (deps, qid, _) = setup_3_round_dispute();
    let mut env = mock_env();
    env.block.time = env.block.time.plus_seconds(u64::from(DAY) + 1);
    let err = query_entry(
        deps.as_ref(),
        env,
        QueryMsg::FinalAnswerIfMatches {
            question_id: Binary::from(qid),
            min_bond: Some(Uint128::from(100u128)),
            min_timeout_secs: None,
            required_arbitrator: None,
            required_denom: None,
        },
    )
    .unwrap_err();
    assert!(format!("{err}").contains("guarantees not met"));
}

#[test]
fn query_final_answer_if_matches_fails_denom_mismatch() {
    let (deps, qid, _) = setup_3_round_dispute();
    let mut env = mock_env();
    env.block.time = env.block.time.plus_seconds(u64::from(DAY) + 1);
    let err = query_entry(
        deps.as_ref(),
        env,
        QueryMsg::FinalAnswerIfMatches {
            question_id: Binary::from(qid),
            min_bond: None,
            min_timeout_secs: None,
            required_arbitrator: None,
            required_denom: Some("uatom".to_string()),
        },
    )
    .unwrap_err();
    assert!(format!("{err}").contains("guarantees not met"));
}

#[test]
fn query_list_paginates() {
    let mut deps = setup();
    for n in 0..5 {
        ask_question(&mut deps, "alice", MIN_FLOOR, DAY, n);
    }
    let env = mock_env();
    let bin = query_entry(
        deps.as_ref(),
        env,
        QueryMsg::List {
            start_after: None,
            limit: Some(3),
            status: Some(State::OpenUnanswered),
        },
    )
    .unwrap();
    let resp: QuestionsListResponse = cosmwasm_std::from_json(bin).unwrap();
    assert_eq!(resp.questions.len(), 3);
    for q in &resp.questions {
        assert_eq!(q.state, State::OpenUnanswered);
    }
}

// ---- cw20 Receive ----

use crate::msg::ReceiveAction;
use cosmwasm_std::to_json_binary;

#[test]
fn receive_ask_question_via_cw20() {
    let mut deps = setup();
    let env = mock_env();
    let cw20_contract = mock_info("cw20token", &[]);
    let bonder = "alice".to_string();
    let action = ReceiveAction::AskQuestion {
        text: "cw20-denominated question".to_string(),
        answer_type: AnswerType::Bool,
        initial_bond: Uint128::from(MIN_FLOOR),
        answer_timeout_secs: DAY,
        arbitrator: None,
        arbitration_timeout_secs: None,
        answer_schema: None,
        opening_ts: None,
        nonce: 0,
    };
    let wrapper = cw20::Cw20ReceiveMsg {
        sender: bonder.clone(),
        amount: Uint128::zero(), // Ask with no bounty top-up
        msg: to_json_binary(&action).unwrap(),
    };
    let err = execute(
        deps.as_mut(),
        env.clone(),
        cw20_contract.clone(),
        ExecuteMsg::Receive(wrapper),
    )
    .unwrap_err();
    // Zero amount on cw20 path is rejected.
    assert!(matches!(err, ContractError::Std(_)));

    // Now with a non-zero bounty (becomes part of question.bounty).
    let action = ReceiveAction::AskQuestion {
        text: "cw20 question 2".to_string(),
        answer_type: AnswerType::Bool,
        initial_bond: Uint128::from(MIN_FLOOR),
        answer_timeout_secs: DAY,
        arbitrator: None,
        arbitration_timeout_secs: None,
        answer_schema: None,
        opening_ts: None,
        nonce: 1,
    };
    let wrapper = cw20::Cw20ReceiveMsg {
        sender: bonder.clone(),
        amount: Uint128::from(500u128),
        msg: to_json_binary(&action).unwrap(),
    };
    let res = execute(
        deps.as_mut(),
        env,
        cw20_contract,
        ExecuteMsg::Receive(wrapper),
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
    let q = QUESTIONS.load(&deps.storage, qid.as_slice()).unwrap();
    assert_eq!(q.bond_denom, "cw20token");
    assert_eq!(q.bounty, Uint128::from(500u128));
    assert_eq!(q.asker, cosmwasm_std::Addr::unchecked("alice"));
}

#[test]
fn receive_submit_answer_via_cw20() {
    let mut deps = setup();
    let env = mock_env();
    let cw20_contract = mock_info("cw20token", &[]);
    // Alice asks a cw20-denominated question first.
    let ask_action = ReceiveAction::AskQuestion {
        text: "cw20 q".to_string(),
        answer_type: AnswerType::Bool,
        initial_bond: Uint128::from(MIN_FLOOR),
        answer_timeout_secs: DAY,
        arbitrator: None,
        arbitration_timeout_secs: None,
        answer_schema: None,
        opening_ts: None,
        nonce: 0,
    };
    let ask_wrapper = cw20::Cw20ReceiveMsg {
        sender: "alice".to_string(),
        amount: Uint128::from(MIN_FLOOR), // initial bounty (must be > 0 on cw20 path)
        msg: to_json_binary(&ask_action).unwrap(),
    };
    let res = execute(
        deps.as_mut(),
        env.clone(),
        cw20_contract.clone(),
        ExecuteMsg::Receive(ask_wrapper),
    )
    .unwrap();
    let qid_bytes = hex::decode(
        &res.attributes
            .iter()
            .find(|a| a.key == "question_id")
            .unwrap()
            .value,
    )
    .unwrap();

    // Bob answers via cw20 path.
    let answer_action = ReceiveAction::SubmitAnswer {
        question_id: Binary::from(qid_bytes.clone()),
        answer: Binary::from(vec![1u8; 32]),
        current_bond_seen: None,
    };
    let answer_wrapper = cw20::Cw20ReceiveMsg {
        sender: "bob".to_string(),
        amount: Uint128::from(MIN_FLOOR),
        msg: to_json_binary(&answer_action).unwrap(),
    };
    execute(
        deps.as_mut(),
        env,
        cw20_contract,
        ExecuteMsg::Receive(answer_wrapper),
    )
    .unwrap();
    let q = QUESTIONS.load(&deps.storage, qid_bytes.as_slice()).unwrap();
    assert_eq!(q.round_count, 1);
    assert_eq!(q.current_bond, Uint128::from(MIN_FLOOR));
}

#[test]
fn receive_wrong_cw20_token_for_existing_question_rejected() {
    let mut deps = setup();
    let env = mock_env();
    // First create a ujuno-denominated question via the native path.
    let qid = ask_question(&mut deps, "alice", MIN_FLOOR, DAY, 0);
    // Bob tries to answer via a cw20 path — the cw20 contract address
    // becomes the denom, which won't match the question's bond_denom.
    let cw20_contract = mock_info("cw20token", &[]);
    let answer_action = ReceiveAction::SubmitAnswer {
        question_id: Binary::from(qid),
        answer: Binary::from(vec![1u8; 32]),
        current_bond_seen: None,
    };
    let wrapper = cw20::Cw20ReceiveMsg {
        sender: "bob".to_string(),
        amount: Uint128::from(MIN_FLOOR),
        msg: to_json_binary(&answer_action).unwrap(),
    };
    let err = execute(
        deps.as_mut(),
        env,
        cw20_contract,
        ExecuteMsg::Receive(wrapper),
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::BondDenomMismatch { .. }));
}

#[test]
fn query_balance_returns_credited_amount() {
    let (mut deps, qid, entries) = setup_3_round_dispute();
    let mut env = mock_env();
    env.block.time = env.block.time.plus_seconds(u64::from(DAY) + 1);
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info("anyone", &[]),
        ExecuteMsg::Claim {
            question_id: Binary::from(qid),
            history_entries: entries,
        },
    )
    .unwrap();
    let bin = query_entry(
        deps.as_ref(),
        env,
        QueryMsg::Balance {
            address: "carol".to_string(),
            denom: "ujuno".to_string(),
        },
    )
    .unwrap();
    let resp: BalanceResponse = cosmwasm_std::from_json(bin).unwrap();
    assert_eq!(resp.amount, Uint128::from(150u128));
}

// ---- Arbitration ----

fn ask_question_with_arb(
    deps: &mut cosmwasm_std::OwnedDeps<
        cosmwasm_std::MemoryStorage,
        cosmwasm_std::testing::MockApi,
        cosmwasm_std::testing::MockQuerier,
    >,
    asker: &str,
    arbitrator: Option<&str>,
    nonce: u64,
) -> Vec<u8> {
    let env = mock_env();
    let info = mock_info(asker, &[]);
    let res = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::AskQuestion {
            text: format!("Question {nonce}"),
            answer_type: AnswerType::Bool,
            bond_denom: "ujuno".to_string(),
            initial_bond: Uint128::from(MIN_FLOOR),
            answer_timeout_secs: DAY,
            arbitrator: arbitrator.map(String::from),
            arbitration_timeout_secs: Some(7 * DAY),
            answer_schema: None,
            opening_ts: None,
            nonce,
        },
    )
    .unwrap();
    hex::decode(
        &res.attributes
            .iter()
            .find(|a| a.key == "question_id")
            .unwrap()
            .value,
    )
    .unwrap()
}

#[test]
fn request_arbitration_happy_path() {
    let mut deps = setup();
    let qid = ask_question_with_arb(&mut deps, "alice", Some("arb"), 0);
    let env = mock_env();
    // Need at least one answer.
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info("bob", &coins(MIN_FLOOR, "ujuno")),
        ExecuteMsg::SubmitAnswer {
            question_id: Binary::from(qid.clone()),
            answer: Binary::from(vec![1u8; 32]),
            current_bond_seen: None,
        },
    )
    .unwrap();
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info("arb", &[]),
        ExecuteMsg::RequestArbitration {
            question_id: Binary::from(qid.clone()),
            current_bond_seen: None,
        },
    )
    .unwrap();
    let q = QUESTIONS.load(&deps.storage, qid.as_slice()).unwrap();
    assert!(q.is_pending_arbitration);
    assert_eq!(
        q.arbitration_deadline,
        Some(env.block.time.seconds() + 7 * u64::from(DAY))
    );
    assert_eq!(
        q.state_at(env.block.time.seconds()),
        State::PendingArbitration
    );
}

#[test]
fn request_arbitration_no_arbitrator_rejected() {
    let mut deps = setup();
    let qid = ask_question(&mut deps, "alice", MIN_FLOOR, DAY, 0);
    let env = mock_env();
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info("bob", &coins(MIN_FLOOR, "ujuno")),
        ExecuteMsg::SubmitAnswer {
            question_id: Binary::from(qid.clone()),
            answer: Binary::from(vec![1u8; 32]),
            current_bond_seen: None,
        },
    )
    .unwrap();
    let err = execute(
        deps.as_mut(),
        env,
        mock_info("anyone", &[]),
        ExecuteMsg::RequestArbitration {
            question_id: Binary::from(qid),
            current_bond_seen: None,
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::NoArbitrator {});
}

#[test]
fn request_arbitration_non_arbitrator_rejected() {
    let mut deps = setup();
    let qid = ask_question_with_arb(&mut deps, "alice", Some("arb"), 0);
    let env = mock_env();
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info("bob", &coins(MIN_FLOOR, "ujuno")),
        ExecuteMsg::SubmitAnswer {
            question_id: Binary::from(qid.clone()),
            answer: Binary::from(vec![1u8; 32]),
            current_bond_seen: None,
        },
    )
    .unwrap();
    let err = execute(
        deps.as_mut(),
        env,
        mock_info("attacker", &[]),
        ExecuteMsg::RequestArbitration {
            question_id: Binary::from(qid),
            current_bond_seen: None,
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::NotArbitrator {});
}

#[test]
fn request_arbitration_without_answer_rejected() {
    let mut deps = setup();
    let qid = ask_question_with_arb(&mut deps, "alice", Some("arb"), 0);
    let env = mock_env();
    // audit issue #2 fix — no answer submitted yet.
    let err = execute(
        deps.as_mut(),
        env,
        mock_info("arb", &[]),
        ExecuteMsg::RequestArbitration {
            question_id: Binary::from(qid),
            current_bond_seen: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::InvalidState { .. }));
}

#[test]
fn cancel_arbitration_by_arbitrator() {
    let mut deps = setup();
    let qid = ask_question_with_arb(&mut deps, "alice", Some("arb"), 0);
    let env = mock_env();
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info("bob", &coins(MIN_FLOOR, "ujuno")),
        ExecuteMsg::SubmitAnswer {
            question_id: Binary::from(qid.clone()),
            answer: Binary::from(vec![1u8; 32]),
            current_bond_seen: None,
        },
    )
    .unwrap();
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info("arb", &[]),
        ExecuteMsg::RequestArbitration {
            question_id: Binary::from(qid.clone()),
            current_bond_seen: None,
        },
    )
    .unwrap();
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info("arb", &[]),
        ExecuteMsg::CancelArbitration {
            question_id: Binary::from(qid.clone()),
        },
    )
    .unwrap();
    let q = QUESTIONS.load(&deps.storage, qid.as_slice()).unwrap();
    assert!(!q.is_pending_arbitration);
    assert_eq!(q.arbitration_deadline, None);
    // finalize_ts re-extended from now, not restored.
    assert_eq!(
        q.finalize_ts,
        Some(env.block.time.seconds() + u64::from(DAY))
    );
}

#[test]
fn cancel_arbitration_anyone_after_deadline() {
    let mut deps = setup();
    let qid = ask_question_with_arb(&mut deps, "alice", Some("arb"), 0);
    let mut env = mock_env();
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info("bob", &coins(MIN_FLOOR, "ujuno")),
        ExecuteMsg::SubmitAnswer {
            question_id: Binary::from(qid.clone()),
            answer: Binary::from(vec![1u8; 32]),
            current_bond_seen: None,
        },
    )
    .unwrap();
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info("arb", &[]),
        ExecuteMsg::RequestArbitration {
            question_id: Binary::from(qid.clone()),
            current_bond_seen: None,
        },
    )
    .unwrap();
    // Advance past arbitration_deadline (7 days).
    env.block.time = env.block.time.plus_seconds(7 * 24 * 60 * 60 + 1);
    execute(
        deps.as_mut(),
        env,
        mock_info("rando", &[]),
        ExecuteMsg::CancelArbitration {
            question_id: Binary::from(qid.clone()),
        },
    )
    .unwrap();
    let q = QUESTIONS.load(&deps.storage, qid.as_slice()).unwrap();
    assert!(!q.is_pending_arbitration);
}

#[test]
fn cancel_arbitration_unauthorized_before_deadline() {
    let mut deps = setup();
    let qid = ask_question_with_arb(&mut deps, "alice", Some("arb"), 0);
    let env = mock_env();
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info("bob", &coins(MIN_FLOOR, "ujuno")),
        ExecuteMsg::SubmitAnswer {
            question_id: Binary::from(qid.clone()),
            answer: Binary::from(vec![1u8; 32]),
            current_bond_seen: None,
        },
    )
    .unwrap();
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info("arb", &[]),
        ExecuteMsg::RequestArbitration {
            question_id: Binary::from(qid.clone()),
            current_bond_seen: None,
        },
    )
    .unwrap();
    let err = execute(
        deps.as_mut(),
        env,
        mock_info("rando", &[]),
        ExecuteMsg::CancelArbitration {
            question_id: Binary::from(qid),
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});
}

#[test]
fn submit_arbitration_happy_path() {
    let mut deps = setup();
    let qid = ask_question_with_arb(&mut deps, "alice", Some("arb"), 0);
    let env = mock_env();
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info("bob", &coins(MIN_FLOOR, "ujuno")),
        ExecuteMsg::SubmitAnswer {
            question_id: Binary::from(qid.clone()),
            answer: Binary::from(vec![1u8; 32]),
            current_bond_seen: None,
        },
    )
    .unwrap();
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info("arb", &[]),
        ExecuteMsg::RequestArbitration {
            question_id: Binary::from(qid.clone()),
            current_bond_seen: None,
        },
    )
    .unwrap();
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info("arb", &[]),
        ExecuteMsg::SubmitArbitration {
            question_id: Binary::from(qid.clone()),
            winning_answer: Binary::from(vec![1u8; 32]),
            payee: "verdict_payee".to_string(),
        },
    )
    .unwrap();
    let q = QUESTIONS.load(&deps.storage, qid.as_slice()).unwrap();
    assert!(!q.is_pending_arbitration);
    assert_eq!(q.finalize_ts, Some(env.block.time.seconds()));
    assert_eq!(q.state_at(env.block.time.seconds()), State::Finalized);
    assert_eq!(q.best_answer.as_ref().unwrap().as_slice(), vec![1u8; 32]);
}

#[test]
fn submit_arbitration_unresolved_sentinel() {
    let mut deps = setup();
    let qid = ask_question_with_arb(&mut deps, "alice", Some("arb"), 0);
    let env = mock_env();
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info("bob", &coins(MIN_FLOOR, "ujuno")),
        ExecuteMsg::SubmitAnswer {
            question_id: Binary::from(qid.clone()),
            answer: Binary::from(vec![1u8; 32]),
            current_bond_seen: None,
        },
    )
    .unwrap();
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info("arb", &[]),
        ExecuteMsg::RequestArbitration {
            question_id: Binary::from(qid.clone()),
            current_bond_seen: None,
        },
    )
    .unwrap();
    let res = execute(
        deps.as_mut(),
        env,
        mock_info("arb", &[]),
        ExecuteMsg::SubmitArbitration {
            question_id: Binary::from(qid),
            winning_answer: Binary::from(crate::state::UNRESOLVED_ANSWER_BYTES.to_vec()),
            payee: "verdict_payee".to_string(),
        },
    )
    .unwrap();
    assert!(res
        .attributes
        .iter()
        .any(|a| a.key == "unresolved" && a.value == "true"));
}

#[test]
fn submit_arbitration_not_pending_rejected() {
    let mut deps = setup();
    let qid = ask_question_with_arb(&mut deps, "alice", Some("arb"), 0);
    let env = mock_env();
    execute(
        deps.as_mut(),
        env.clone(),
        mock_info("bob", &coins(MIN_FLOOR, "ujuno")),
        ExecuteMsg::SubmitAnswer {
            question_id: Binary::from(qid.clone()),
            answer: Binary::from(vec![1u8; 32]),
            current_bond_seen: None,
        },
    )
    .unwrap();
    // Skip RequestArbitration, go straight to submit.
    let err = execute(
        deps.as_mut(),
        env,
        mock_info("arb", &[]),
        ExecuteMsg::SubmitArbitration {
            question_id: Binary::from(qid),
            winning_answer: Binary::from(vec![1u8; 32]),
            payee: "verdict_payee".to_string(),
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::InvalidState { .. }));
}

#[test]
fn answer_round_cap_hit() {
    // Drive round_count up to MAX_DISPUTE_ROUNDS, then assert the next call
    // rejects. Starting bond = 1, doubling each round. At round 32 the bond
    // would need to be 2^32 = ~4.29 G; well within Uint128.
    // Need an instantiation with a smaller floor so we can start at 1.
    let env = mock_env();
    let mut deps = mock_dependencies();
    instantiate(
        deps.as_mut(),
        env.clone(),
        mock_info("creator", &[]),
        InstantiateMsg {
            admin: None,
            min_initial_bond_floor: Uint128::one(),
            min_answer_timeout_secs: DAY,
        },
    )
    .unwrap();
    let alice = mock_info("alice", &[]);
    let res = execute(
        deps.as_mut(),
        env.clone(),
        alice,
        ExecuteMsg::AskQuestion {
            text: "cap test".to_string(),
            answer_type: AnswerType::Bool,
            bond_denom: "ujuno".to_string(),
            initial_bond: Uint128::one(),
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
    let mut bond: u128 = 1;
    for round in 0..MAX_DISPUTE_ROUNDS {
        execute(
            deps.as_mut(),
            env.clone(),
            mock_info("bidder", &coins(bond, "ujuno")),
            ExecuteMsg::SubmitAnswer {
                question_id: Binary::from(qid.clone()),
                answer: Binary::from(vec![(round % 256) as u8; 32]),
                current_bond_seen: None,
            },
        )
        .unwrap();
        bond *= 2;
    }
    let err = execute(
        deps.as_mut(),
        env,
        mock_info("bidder", &coins(bond, "ujuno")),
        ExecuteMsg::SubmitAnswer {
            question_id: Binary::from(qid),
            answer: Binary::from(vec![0u8; 32]),
            current_bond_seen: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::RoundCapReached { .. }));
}
