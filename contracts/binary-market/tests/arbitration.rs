use binary_market::{
    contract::{execute, instantiate, query, reply},
    msg::{
        AccountingResponse, ChallengeResponse, ConfigResponse, ExecuteMsg, InstantiateMsg,
        LifecycleStatus, QueryMsg, ResolutionResponse, SolvencyResponse, StateResponse,
    },
    question::{ObservationInput, QuestionInput, SourceInput, INVALID_HEX, NO_HEX, YES_HEX},
};
use cosmwasm_std::{
    coin, Addr, Binary, DepsMut, Empty, Env, MessageInfo, Response, StdError, Uint128,
};
use cw_multi_test::{App, AppBuilder, AppResponse, Contract, ContractWrapper, Executor};
use cw_reality::{
    msg::{
        ExecuteMsg as OracleExecuteMsg, InstantiateMsg as OracleInstantiateMsg,
        QueryMsg as OracleQueryMsg, QuestionResponse as OracleQuestionResponse,
    },
    state::State as OracleState,
};
use pm_types::{Payout, TierId};

const CREATION: u64 = 1_799_800_000;
const CLOSE: u64 = 1_800_000_000;
const OPENING: u64 = 1_800_086_400;
const ANSWER_TIMEOUT: u64 = 86_400;
const ARBITRATION_TIMEOUT: u64 = 1_814_400;
const BOND: u128 = 10_000_000;
const DAO: &str = "juno18k65at7fkf8elhece0fnhsvuxggqg6cved6trp5fyk3lftfn93xsmpeaac";

fn bytes(value: &str) -> Binary {
    Binary::from(hex::decode(value).unwrap())
}

fn market_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(execute, instantiate, query).with_reply(reply))
}

fn oracle_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        fault_injecting_oracle_execute,
        cw_reality::contract::instantiate,
        cw_reality::contract::query,
    ))
}

// Configurable by block height so a test can fail one nested transition and
// then prove the byte-identical retry succeeds in the same app.
fn fault_injecting_oracle_execute(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: OracleExecuteMsg,
) -> Result<Response, cw_reality::error::ContractError> {
    let (fail_height, corrupt_height, question_id) = match &msg {
        OracleExecuteMsg::RequestArbitration { question_id, .. } => {
            (10_001, 10_002, Some(question_id.clone()))
        }
        OracleExecuteMsg::SubmitArbitration { question_id, .. } => {
            (10_003, 10_004, Some(question_id.clone()))
        }
        OracleExecuteMsg::CancelArbitration { question_id } => {
            (10_005, 10_006, Some(question_id.clone()))
        }
        _ => (0, 0, None),
    };
    if env.block.height == fail_height {
        return Err(StdError::generic_err("injected oracle execute failure").into());
    }
    let response = cw_reality::contract::execute(deps.branch(), env.clone(), info, msg)?;
    if env.block.height == corrupt_height {
        let question_id = question_id.expect("fault modes have a question id");
        cw_reality::state::QUESTIONS.update(
            deps.storage,
            question_id.as_slice(),
            |question| -> Result<_, StdError> {
                let mut question = question.ok_or_else(|| StdError::not_found("question"))?;
                question.best_answer = Some(Binary::from(vec![0x45]));
                Ok(question)
            },
        )?;
    }
    Ok(response)
}

fn question() -> QuestionInput {
    QuestionInput {
        title: "Arbitration fixture?".into(),
        proposition: "Will the exact fixture resolve YES?".into(),
        definitions: vec!["The exact published byte controls.".into()],
        invalid_conditions: vec!["The source is unavailable.".into()],
        primary_sources: vec![SourceInput {
            publisher: "Fixture Authority".into(),
            identifier: "fixture/final".into(),
            url: "https://example.com/final".into(),
            retrieval: "HTTPS JSON".into(),
            publication_revision_policy: "Corrections before opening control.".into(),
            fallback_condition: "Unavailable for 72 hours.".into(),
        }],
        secondary_sources: vec![],
        source_disagreement_policy: "The primary source controls.".into(),
        observation: ObservationInput {
            start_ts: CLOSE,
            end_ts: OPENING,
            cutoff_ts: OPENING,
            inclusivity: "inclusive".into(),
            revision_policy: "Corrections before opening control.".into(),
        },
    }
}

struct Fixture {
    app: App,
    market: Addr,
    oracle: Addr,
    qid: Binary,
    answer: Binary,
}

fn setup(initial_answer: Binary) -> Fixture {
    let factory = Addr::unchecked("factory");
    let answerer = Addr::unchecked("answerer");
    let mut app = AppBuilder::new().build(|router, _, storage| {
        for (address, amount) in [
            (factory.as_str(), 2_000_000u128),
            (answerer.as_str(), 50_000_000),
            ("challenger", 20_000_000),
            (DAO, 1_000_000),
        ] {
            router
                .bank
                .init_balance(
                    storage,
                    &Addr::unchecked(address),
                    vec![coin(amount, "ujuno"), coin(20_000_000, "uatom")],
                )
                .unwrap();
        }
    });
    app.update_block(|block| block.time = cosmwasm_std::Timestamp::from_seconds(CREATION));
    let oracle_code = app.store_code(oracle_contract());
    let oracle = app
        .instantiate_contract(
            oracle_code,
            factory.clone(),
            &OracleInstantiateMsg {
                admin: None,
                min_initial_bond_floor: Uint128::new(BOND),
                min_answer_timeout_secs: ANSWER_TIMEOUT as u32,
            },
            &[],
            "oracle",
            None,
        )
        .unwrap();
    let market_code = app.store_code(market_contract());
    let market = app
        .instantiate_contract(
            market_code,
            factory.clone(),
            &InstantiateMsg {
                factory: factory.to_string(),
                creator: "creator".into(),
                oracle: oracle.to_string(),
                verdict_authority: DAO.into(),
                tier: TierId(1),
                question: question(),
                nonce: 45,
                close_ts: CLOSE,
                opening_ts: OPENING,
                initial_liquidity: Uint128::new(100),
                oracle_bounty: Uint128::new(1_000_000),
                oracle_initial_bond: Uint128::new(BOND),
                answer_timeout_secs: ANSWER_TIMEOUT as u32,
                arbitration_timeout_secs: ARBITRATION_TIMEOUT as u32,
                fee_bps: 200,
                min_trade: Uint128::one(),
                max_trade_bps: 2_500,
                collateral_cap: Uint128::new(10_000),
                challenge_bond: Uint128::new(BOND),
            },
            &[coin(1_000_100, "ujuno")],
            "market",
            None,
        )
        .unwrap();
    let bound: binary_market::msg::QuestionResponse = app
        .wrap()
        .query_wasm_smart(&market, &QueryMsg::Question {})
        .unwrap();
    let qid = bound.question_id.unwrap();
    app.update_block(|block| block.time = cosmwasm_std::Timestamp::from_seconds(OPENING));
    app.execute_contract(
        answerer,
        oracle.clone(),
        &OracleExecuteMsg::SubmitAnswer {
            question_id: qid.clone(),
            answer: initial_answer.clone(),
            current_bond_seen: Some(Uint128::zero()),
        },
        &[coin(BOND, "ujuno")],
    )
    .unwrap();
    Fixture {
        app,
        market,
        oracle,
        qid,
        answer: initial_answer,
    }
}

fn challenge(f: &mut Fixture) {
    f.app
        .execute_contract(
            Addr::unchecked("challenger"),
            f.market.clone(),
            &ExecuteMsg::Challenge {},
            &[coin(BOND, "ujuno")],
        )
        .unwrap();
}

fn verdict(
    f: &mut Fixture,
    sender: &str,
    answer: Binary,
    payee: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    f.app.execute_contract(
        Addr::unchecked(sender),
        f.market.clone(),
        &ExecuteMsg::GovernanceVerdict {
            question_id: f.qid.clone(),
            answer,
            payee: payee.into(),
        },
        &[],
    )?;
    Ok(())
}

#[derive(Debug, PartialEq)]
struct TransactionSnapshot {
    challenger_ujuno: Uint128,
    market_ujuno: Uint128,
    challenge: ChallengeResponse,
    accounting: AccountingResponse,
    state: StateResponse,
    oracle: OracleQuestionResponse,
}

fn snapshot(f: &Fixture) -> TransactionSnapshot {
    TransactionSnapshot {
        challenger_ujuno: f
            .app
            .wrap()
            .query_balance("challenger", "ujuno")
            .unwrap()
            .amount,
        market_ujuno: f
            .app
            .wrap()
            .query_balance(&f.market, "ujuno")
            .unwrap()
            .amount,
        challenge: f
            .app
            .wrap()
            .query_wasm_smart(&f.market, &QueryMsg::Challenge {})
            .unwrap(),
        accounting: f
            .app
            .wrap()
            .query_wasm_smart(&f.market, &QueryMsg::Accounting {})
            .unwrap(),
        state: f
            .app
            .wrap()
            .query_wasm_smart(&f.market, &QueryMsg::State {})
            .unwrap(),
        oracle: f
            .app
            .wrap()
            .query_wasm_smart(
                &f.oracle,
                &OracleQueryMsg::Question {
                    question_id: f.qid.clone(),
                },
            )
            .unwrap(),
    }
}

fn arbitration_event<'a>(response: &'a AppResponse, action: &str) -> &'a cosmwasm_std::Event {
    response
        .events
        .iter()
        .find(|event| {
            event.ty == "wasm-juno_pm_v1"
                && event
                    .attributes
                    .iter()
                    .any(|attribute| attribute.key == "action" && attribute.value == action)
        })
        .unwrap_or_else(|| panic!("missing arbitration event {action}"))
}

fn attribute(event: &cosmwasm_std::Event, key: &str) -> String {
    event
        .attributes
        .iter()
        .find(|attribute| attribute.key == key)
        .unwrap_or_else(|| panic!("missing event attribute {key}"))
        .value
        .clone()
}

#[test]
fn authority_is_exact_immutable_and_different_verdict_refunds_without_contamination() {
    let mut f = setup(bytes(YES_HEX));
    let config: ConfigResponse = f
        .app
        .wrap()
        .query_wasm_smart(&f.market, &QueryMsg::Config {})
        .unwrap();
    assert_eq!(config.verdict_authority, DAO);

    challenge(&mut f);
    let pending: ChallengeResponse = f
        .app
        .wrap()
        .query_wasm_smart(&f.market, &QueryMsg::Challenge {})
        .unwrap();
    assert_eq!(pending.challenger.as_deref(), Some("challenger"));
    assert_eq!(pending.answer, Some(f.answer.clone()));
    assert_eq!(pending.oracle_bond, Some(Uint128::new(BOND)));
    assert_eq!(pending.started_at, Some(OPENING));
    assert_eq!(pending.deadline, Some(OPENING + ARBITRATION_TIMEOUT));
    let accounting: AccountingResponse = f
        .app
        .wrap()
        .query_wasm_smart(&f.market, &QueryMsg::Accounting {})
        .unwrap();
    assert_eq!(accounting.principal, Uint128::new(100));
    assert_eq!(accounting.fees, Uint128::zero());
    assert_eq!(accounting.challenge, Uint128::new(BOND));
    assert_eq!(accounting.lp_accrual, Uint128::zero());

    let before = f.app.wrap().query_balance("challenger", "ujuno").unwrap();
    verdict(&mut f, DAO, bytes(NO_HEX), "answerer").unwrap();
    let after = f.app.wrap().query_balance("challenger", "ujuno").unwrap();
    assert_eq!(after.amount - before.amount, Uint128::new(BOND));
    let accounting: AccountingResponse = f
        .app
        .wrap()
        .query_wasm_smart(&f.market, &QueryMsg::Accounting {})
        .unwrap();
    assert_eq!(accounting.challenge, Uint128::zero());
    assert_eq!(accounting.lp_accrual, Uint128::zero());
    assert_eq!(accounting.principal, Uint128::new(100));
    assert_eq!(accounting.fees, Uint128::zero());
    let solvency: SolvencyResponse = f
        .app
        .wrap()
        .query_wasm_smart(&f.market, &QueryMsg::Solvency {})
        .unwrap();
    assert_eq!(solvency.forced_excess, Uint128::zero());
    assert_eq!(
        f.app
            .wrap()
            .query_balance(&f.market, "ujuno")
            .unwrap()
            .amount,
        Uint128::new(100)
    );

    f.app
        .execute_contract(
            Addr::unchecked("challenger"),
            f.market.clone(),
            &ExecuteMsg::Challenge {},
            &[coin(BOND, "ujuno")],
        )
        .unwrap_err();
}

#[test]
fn spoofed_callers_funds_bad_payload_state_and_deadline_fail_before_mutation() {
    let spoofed = [
        "dao-member",
        "proposal-module",
        "voting-module",
        "ordinary-eoa",
        "unrelated-contract",
    ];
    for caller in spoofed {
        let mut f = setup(bytes(YES_HEX));
        challenge(&mut f);
        verdict(&mut f, caller, bytes(NO_HEX), "answerer").unwrap_err();
        let pending: ChallengeResponse = f
            .app
            .wrap()
            .query_wasm_smart(&f.market, &QueryMsg::Challenge {})
            .unwrap();
        assert_eq!(pending.challenger.as_deref(), Some("challenger"));
    }

    let mut f = setup(bytes(YES_HEX));
    verdict(&mut f, DAO, bytes(NO_HEX), "answerer").unwrap_err();
    challenge(&mut f);
    for (qid, answer, payee) in [
        (Binary::from(vec![9; 32]), bytes(NO_HEX), "answerer"),
        (f.qid.clone(), Binary::default(), "answerer"),
        (f.qid.clone(), bytes(NO_HEX), ""),
    ] {
        f.app
            .execute_contract(
                Addr::unchecked(DAO),
                f.market.clone(),
                &ExecuteMsg::GovernanceVerdict {
                    question_id: qid,
                    answer,
                    payee: payee.into(),
                },
                &[],
            )
            .unwrap_err();
    }
    f.app
        .execute_contract(
            Addr::unchecked(DAO),
            f.market.clone(),
            &ExecuteMsg::GovernanceVerdict {
                question_id: f.qid.clone(),
                answer: bytes(NO_HEX),
                payee: "answerer".into(),
            },
            &[coin(1, "ujuno")],
        )
        .unwrap_err();
    f.app.update_block(|block| {
        block.time = cosmwasm_std::Timestamp::from_seconds(OPENING + ARBITRATION_TIMEOUT)
    });
    verdict(&mut f, DAO, bytes(NO_HEX), "answerer").unwrap_err();
    let accounting: AccountingResponse = f
        .app
        .wrap()
        .query_wasm_smart(&f.market, &QueryMsg::Accounting {})
        .unwrap();
    assert_eq!(accounting.challenge, Uint128::new(BOND));
}

#[test]
fn identical_verdict_slashes_to_lp_and_noncanonical_resolves_neutral() {
    for answer in [
        bytes(YES_HEX),
        bytes(INVALID_HEX),
        Binary::from(b"other".as_slice()),
    ] {
        let mut f = setup(bytes(YES_HEX));
        challenge(&mut f);
        verdict(&mut f, DAO, answer.clone(), "answerer").unwrap();
        let accounting: AccountingResponse = f
            .app
            .wrap()
            .query_wasm_smart(&f.market, &QueryMsg::Accounting {})
            .unwrap();
        assert_eq!(accounting.challenge, Uint128::zero());
        assert_eq!(
            accounting.lp_accrual,
            if answer == bytes(YES_HEX) {
                Uint128::new(BOND)
            } else {
                Uint128::zero()
            }
        );
        f.app
            .execute_contract(
                Addr::unchecked("anyone"),
                f.market.clone(),
                &ExecuteMsg::Resolve {},
                &[],
            )
            .unwrap();
        let resolution: ResolutionResponse = f
            .app
            .wrap()
            .query_wasm_smart(&f.market, &QueryMsg::Resolution {})
            .unwrap();
        let expected = if answer == bytes(YES_HEX) {
            Payout::for_outcome(pm_types::Outcome::Yes)
        } else {
            Payout::neutral()
        };
        assert_eq!(resolution.payout, Some(expected));
    }
}

#[test]
fn timeout_and_direct_cancellation_synchronize_once_and_second_challenge_stays_rejected() {
    for direct_cancel in [false, true] {
        let mut f = setup(bytes(YES_HEX));
        challenge(&mut f);
        f.app.update_block(|block| {
            block.time = cosmwasm_std::Timestamp::from_seconds(OPENING + ARBITRATION_TIMEOUT)
        });
        if direct_cancel {
            f.app
                .execute_contract(
                    Addr::unchecked("keeper"),
                    f.oracle.clone(),
                    &OracleExecuteMsg::CancelArbitration {
                        question_id: f.qid.clone(),
                    },
                    &[],
                )
                .unwrap();
            // Synchronization may happen in a later block. The cancellation's
            // re-extension remains authoritative and must not strand escrow.
            f.app
                .update_block(|block| block.time = block.time.plus_seconds(17));
        }
        f.app
            .execute_contract(
                Addr::unchecked("anyone"),
                f.market.clone(),
                &ExecuteMsg::FinalizeStalledChallenge {},
                &[],
            )
            .unwrap();
        let accounting: AccountingResponse = f
            .app
            .wrap()
            .query_wasm_smart(&f.market, &QueryMsg::Accounting {})
            .unwrap();
        assert_eq!(accounting.challenge, Uint128::zero());
        assert_eq!(accounting.lp_accrual, Uint128::new(BOND));
        let state: StateResponse = f
            .app
            .wrap()
            .query_wasm_smart(&f.market, &QueryMsg::State {})
            .unwrap();
        assert_eq!(state.status, LifecycleStatus::AwaitingResolution);
        assert!(state.challenge_used);
        let oracle: OracleQuestionResponse = f
            .app
            .wrap()
            .query_wasm_smart(
                &f.oracle,
                &OracleQueryMsg::Question {
                    question_id: f.qid.clone(),
                },
            )
            .unwrap();
        assert_eq!(oracle.state, OracleState::OpenAnswered);
        f.app
            .execute_contract(
                Addr::unchecked("challenger"),
                f.market.clone(),
                &ExecuteMsg::Challenge {},
                &[coin(BOND, "ujuno")],
            )
            .unwrap_err();
        f.app
            .execute_contract(
                Addr::unchecked("anyone"),
                f.market,
                &ExecuteMsg::FinalizeStalledChallenge {},
                &[],
            )
            .unwrap_err();
    }
}

#[test]
fn nested_oracle_failures_and_reply_verification_failures_are_atomic_and_retryable() {
    for height in [10_001, 10_002] {
        let mut f = setup(bytes(YES_HEX));
        let before = snapshot(&f);
        f.app.update_block(|block| block.height = height);
        f.app
            .execute_contract(
                Addr::unchecked("challenger"),
                f.market.clone(),
                &ExecuteMsg::Challenge {},
                &[coin(BOND, "ujuno")],
            )
            .unwrap_err();
        assert_eq!(
            snapshot(&f),
            before,
            "request mode {height} must roll back all market/oracle/bank state"
        );
        f.app.update_block(|block| block.height = 20_000);
        challenge(&mut f); // also proves the reply marker was rolled back
    }

    for height in [10_003, 10_004] {
        let mut f = setup(bytes(YES_HEX));
        challenge(&mut f);
        let before = snapshot(&f);
        f.app.update_block(|block| block.height = height);
        verdict(&mut f, DAO, bytes(NO_HEX), "answerer").unwrap_err();
        assert_eq!(
            snapshot(&f),
            before,
            "submit mode {height} must roll back all market/oracle/bank state"
        );
        f.app.update_block(|block| block.height = 20_000);
        verdict(&mut f, DAO, bytes(NO_HEX), "answerer").unwrap();
    }

    for height in [10_005, 10_006] {
        let mut f = setup(bytes(YES_HEX));
        challenge(&mut f);
        f.app.update_block(|block| {
            block.time = cosmwasm_std::Timestamp::from_seconds(OPENING + ARBITRATION_TIMEOUT);
            block.height = height;
        });
        let before = snapshot(&f);
        f.app
            .execute_contract(
                Addr::unchecked("keeper"),
                f.market.clone(),
                &ExecuteMsg::FinalizeStalledChallenge {},
                &[],
            )
            .unwrap_err();
        assert_eq!(
            snapshot(&f),
            before,
            "cancel mode {height} must roll back all market/oracle/bank state"
        );
        f.app.update_block(|block| block.height = 20_000);
        f.app
            .execute_contract(
                Addr::unchecked("keeper"),
                f.market.clone(),
                &ExecuteMsg::FinalizeStalledChallenge {},
                &[],
            )
            .unwrap();
    }
}

#[test]
fn challenge_funding_matrix_and_dynamic_oracle_bond_are_exact() {
    for funds in [
        vec![coin(BOND, "uatom")],
        vec![coin(BOND, "ujuno"), coin(1, "uatom")],
        vec![coin(BOND - 1, "ujuno")],
        vec![coin(BOND + 1, "ujuno")],
    ] {
        let mut f = setup(bytes(YES_HEX));
        let before = snapshot(&f);
        f.app
            .execute_contract(
                Addr::unchecked("challenger"),
                f.market.clone(),
                &ExecuteMsg::Challenge {},
                &funds,
            )
            .unwrap_err();
        assert_eq!(snapshot(&f), before, "bad funds {funds:?} mutated state");
    }

    let mut f = setup(bytes(YES_HEX));
    f.app
        .execute_contract(
            Addr::unchecked("answerer"),
            f.oracle.clone(),
            &OracleExecuteMsg::SubmitAnswer {
                question_id: f.qid.clone(),
                answer: bytes(NO_HEX),
                current_bond_seen: Some(Uint128::new(BOND)),
            },
            &[coin(BOND * 2, "ujuno")],
        )
        .unwrap();
    f.answer = bytes(NO_HEX);
    f.app
        .execute_contract(
            Addr::unchecked("challenger"),
            f.market.clone(),
            &ExecuteMsg::Challenge {},
            &[coin(BOND, "ujuno")],
        )
        .unwrap_err();
    f.app
        .execute_contract(
            Addr::unchecked("challenger"),
            f.market.clone(),
            &ExecuteMsg::Challenge {},
            &[coin(BOND * 2, "ujuno")],
        )
        .unwrap();
    assert_eq!(snapshot(&f).accounting.challenge, Uint128::new(BOND * 2));
}

#[test]
fn verdict_deadline_finalize_funds_replay_and_pending_snapshot_matrix() {
    let mut f = setup(bytes(YES_HEX));
    challenge(&mut f);
    f.app.update_block(|block| {
        block.time = cosmwasm_std::Timestamp::from_seconds(OPENING + ARBITRATION_TIMEOUT - 1)
    });
    verdict(&mut f, DAO, bytes(NO_HEX), "answerer").unwrap();
    verdict(&mut f, DAO, bytes(NO_HEX), "answerer").unwrap_err();

    let mut f = setup(bytes(YES_HEX));
    challenge(&mut f);
    f.app.update_block(|block| {
        block.time = cosmwasm_std::Timestamp::from_seconds(OPENING + ARBITRATION_TIMEOUT)
    });
    let before = snapshot(&f);
    verdict(&mut f, DAO, bytes(NO_HEX), "answerer").unwrap_err();
    assert_eq!(snapshot(&f), before);
    f.app
        .execute_contract(
            Addr::unchecked("keeper"),
            f.market.clone(),
            &ExecuteMsg::FinalizeStalledChallenge {},
            &[coin(1, "ujuno")],
        )
        .unwrap_err();
    assert_eq!(snapshot(&f), before);

    // A directly cancelled oracle no longer matches the pending snapshot;
    // verdict forwarding rejects without changing either contract.
    f.app
        .execute_contract(
            Addr::unchecked("keeper"),
            f.oracle.clone(),
            &OracleExecuteMsg::CancelArbitration {
                question_id: f.qid.clone(),
            },
            &[],
        )
        .unwrap();
    let mismatched = snapshot(&f);
    verdict(&mut f, DAO, bytes(NO_HEX), "answerer").unwrap_err();
    assert_eq!(snapshot(&f), mismatched);
}

#[test]
fn exact_arbitration_events_cover_refund_identical_and_timeout_slash() {
    let assert_identity = |event: &cosmwasm_std::Event| {
        assert_eq!(attribute(event, "protocol_version"), "1");
        assert_eq!(attribute(event, "factory"), "factory");
        assert!(!attribute(event, "market").is_empty());
        assert!(!attribute(event, "height").is_empty());
        assert!(!attribute(event, "block_time").is_empty());
        assert_eq!(attribute(event, "authority"), DAO);
    };

    for (answer, action, disposition, reason, recipient) in [
        (
            bytes(NO_HEX),
            "challenge_refunded",
            "refunded",
            "different_verdict",
            "challenger",
        ),
        (
            bytes(YES_HEX),
            "challenge_slashed",
            "slashed_to_lp",
            "identical_verdict",
            "creator",
        ),
    ] {
        let mut f = setup(bytes(YES_HEX));
        let requested = f
            .app
            .execute_contract(
                Addr::unchecked("challenger"),
                f.market.clone(),
                &ExecuteMsg::Challenge {},
                &[coin(BOND, "ujuno")],
            )
            .unwrap();
        let request_event = arbitration_event(&requested, "challenge_requested");
        assert_identity(request_event);
        assert_eq!(attribute(request_event, "answer_hex"), YES_HEX);
        assert_eq!(
            attribute(request_event, "arbitration_deadline"),
            (OPENING + ARBITRATION_TIMEOUT).to_string()
        );

        let response = f
            .app
            .execute_contract(
                Addr::unchecked(DAO),
                f.market.clone(),
                &ExecuteMsg::GovernanceVerdict {
                    question_id: f.qid.clone(),
                    answer: answer.clone(),
                    payee: "answerer".into(),
                },
                &[],
            )
            .unwrap();
        let forwarded = arbitration_event(&response, "governance_verdict_forwarded");
        assert_identity(forwarded);
        assert_eq!(
            attribute(forwarded, "answer_hex"),
            hex::encode(answer.as_slice())
        );
        assert_eq!(attribute(forwarded, "answer_base64"), answer.to_base64());
        assert_eq!(attribute(forwarded, "payee"), "answerer");
        assert_eq!(attribute(forwarded, "question_id"), f.qid.to_base64());
        let settled = arbitration_event(&response, action);
        assert_identity(settled);
        assert_eq!(attribute(settled, "recipient"), recipient);
        assert_eq!(attribute(settled, "disposition"), disposition);
        assert_eq!(attribute(settled, "reason"), reason);
        assert_eq!(attribute(settled, "amount"), BOND.to_string());
    }

    for direct in [false, true] {
        let mut f = setup(bytes(YES_HEX));
        challenge(&mut f);
        f.app.update_block(|block| {
            block.time = cosmwasm_std::Timestamp::from_seconds(OPENING + ARBITRATION_TIMEOUT)
        });
        if direct {
            f.app
                .execute_contract(
                    Addr::unchecked("keeper"),
                    f.oracle.clone(),
                    &OracleExecuteMsg::CancelArbitration {
                        question_id: f.qid.clone(),
                    },
                    &[],
                )
                .unwrap();
        }
        let response = f
            .app
            .execute_contract(
                Addr::unchecked("keeper"),
                f.market.clone(),
                &ExecuteMsg::FinalizeStalledChallenge {},
                &[],
            )
            .unwrap();
        let event = arbitration_event(&response, "challenge_slashed");
        assert_identity(event);
        assert_eq!(attribute(event, "recipient"), "creator");
        assert_eq!(attribute(event, "disposition"), "slashed_to_lp");
        assert_eq!(
            attribute(event, "reason"),
            if direct {
                "oracle_already_cancelled"
            } else {
                "arbitration_timeout"
            }
        );
    }
}

#[test]
fn invalid_authority_rejects_instantiation() {
    let mut msg = setup_message_for_validation();
    msg.verdict_authority = "".into();
    let mut deps = cosmwasm_std::testing::mock_dependencies();
    let env = cosmwasm_std::testing::mock_env();
    let info = cosmwasm_std::testing::mock_info("factory", &[coin(1_000_100, "ujuno")]);
    assert!(instantiate(deps.as_mut(), env, info, msg).is_err());
}

fn setup_message_for_validation() -> InstantiateMsg {
    InstantiateMsg {
        factory: "factory".into(),
        creator: "creator".into(),
        oracle: "oracle".into(),
        verdict_authority: DAO.into(),
        tier: TierId(1),
        question: question(),
        nonce: 45,
        close_ts: 1_900_000_000,
        opening_ts: 1_900_086_400,
        initial_liquidity: Uint128::new(100),
        oracle_bounty: Uint128::new(1_000_000),
        oracle_initial_bond: Uint128::new(BOND),
        answer_timeout_secs: ANSWER_TIMEOUT as u32,
        arbitration_timeout_secs: ARBITRATION_TIMEOUT as u32,
        fee_bps: 200,
        min_trade: Uint128::one(),
        max_trade_bps: 2_500,
        collateral_cap: Uint128::new(10_000),
        challenge_bond: Uint128::new(BOND),
    }
}
