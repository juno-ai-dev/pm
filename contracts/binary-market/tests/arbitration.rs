use binary_market::{
    contract::{execute, instantiate, query, reply},
    msg::{
        AccountingResponse, ChallengeResponse, ConfigResponse, ExecuteMsg, InstantiateMsg,
        LifecycleStatus, QueryMsg, ResolutionResponse, SolvencyResponse, StateResponse,
    },
    question::{ObservationInput, QuestionInput, SourceInput, INVALID_HEX, NO_HEX, YES_HEX},
};
use cosmwasm_std::{coin, Addr, Binary, Empty, Uint128};
use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
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
        cw_reality::contract::execute,
        cw_reality::contract::instantiate,
        cw_reality::contract::query,
    ))
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
            (answerer.as_str(), 20_000_000),
            ("challenger", 20_000_000),
            (DAO, 1_000_000),
        ] {
            router
                .bank
                .init_balance(
                    storage,
                    &Addr::unchecked(address),
                    vec![coin(amount, "ujuno")],
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
    assert!(pending.refundable);
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
