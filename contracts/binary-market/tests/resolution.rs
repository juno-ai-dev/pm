use binary_market::{
    contract::{execute, instantiate, query, reply},
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ResolutionResponse, StateResponse},
    question::{
        ObservationInput, QuestionInput, SourceInput, INVALID_HEX, NO_HEX, UNRESOLVED_HEX, YES_HEX,
    },
};
use cosmwasm_std::{coin, Addr, Binary, Empty, Uint128};
use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
use cw_reality::msg::{ExecuteMsg as OracleExecuteMsg, InstantiateMsg as OracleInstantiateMsg};
use pm_types::{Payout, TierId};

const CREATION: u64 = 1_799_800_000;
const CLOSE: u64 = 1_800_000_000;
const OPENING: u64 = 1_800_086_400;
const BOND: u128 = 10_000_000;

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
        title: "Resolution fixture?".into(),
        proposition: "Will the fixture resolve yes?".into(),
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

fn setup(answer: Binary) -> (App, Addr, Addr, Addr) {
    setup_with_splits(answer, &[])
}

fn setup_with_splits(answer: Binary, splits: &[(&str, u128)]) -> (App, Addr, Addr, Addr) {
    let factory = Addr::unchecked("factory");
    let answerer = Addr::unchecked("answerer");
    let mut app = AppBuilder::new().build(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &factory, vec![coin(2_000_000, "ujuno")])
            .unwrap();
        router
            .bank
            .init_balance(storage, &answerer, vec![coin(20_000_000, "ujuno")])
            .unwrap();
        for owner in ["alice", "bob"] {
            router
                .bank
                .init_balance(storage, &Addr::unchecked(owner), vec![coin(1_000, "ujuno")])
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
                min_answer_timeout_secs: 86_400,
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
                verdict_authority: "governance".into(),
                tier: TierId(1),
                question: question(),
                nonce: 12,
                close_ts: CLOSE,
                opening_ts: OPENING,
                initial_liquidity: Uint128::new(100),
                oracle_bounty: Uint128::new(1_000_000),
                oracle_initial_bond: Uint128::new(BOND),
                answer_timeout_secs: 86_400,
                arbitration_timeout_secs: 1_814_400,
                fee_bps: 200,
                min_trade: Uint128::one(),
                max_trade_bps: 2_500,
                max_position_per_side: Uint128::MAX,
                collateral_cap: Uint128::new(10_000),
                challenge_bond: Uint128::new(BOND),
            },
            &[coin(1_000_100, "ujuno")],
            "market",
            None,
        )
        .unwrap();
    for (owner, amount) in splits {
        app.execute_contract(
            Addr::unchecked(*owner),
            market.clone(),
            &ExecuteMsg::Split {
                amount: Uint128::new(*amount),
            },
            &[coin(*amount, "ujuno")],
        )
        .unwrap();
    }
    let bound: binary_market::msg::QuestionResponse = app
        .wrap()
        .query_wasm_smart(&market, &QueryMsg::Question {})
        .unwrap();
    app.update_block(|block| block.time = cosmwasm_std::Timestamp::from_seconds(OPENING));
    app.execute_contract(
        answerer.clone(),
        oracle.clone(),
        &OracleExecuteMsg::SubmitAnswer {
            question_id: bound.question_id.unwrap(),
            answer,
            current_bond_seen: Some(Uint128::zero()),
        },
        &[coin(BOND, "ujuno")],
    )
    .unwrap();
    (app, market, oracle, answerer)
}

fn bytes(hex_value: &str) -> Binary {
    Binary::from(hex::decode(hex_value).unwrap())
}

#[test]
fn exact_yes_no_and_every_noncanonical_class_map_without_transfers() {
    let cases = [
        (bytes(YES_HEX), Payout::for_outcome(pm_types::Outcome::Yes)),
        (bytes(NO_HEX), Payout::for_outcome(pm_types::Outcome::No)),
        (bytes(INVALID_HEX), Payout::neutral()),
        (bytes(UNRESOLVED_HEX), Payout::neutral()),
        (Binary::from(vec![1]), Payout::neutral()),
        (Binary::from(vec![0]), Payout::neutral()),
        (
            Binary::from(b"arbitrary finalized bytes"),
            Payout::neutral(),
        ),
    ];
    for (answer, expected) in cases {
        let (mut app, market, _, resolver) = setup(answer.clone());
        app.update_block(|block| {
            block.time = cosmwasm_std::Timestamp::from_seconds(OPENING + 86_400)
        });
        let before = app.wrap().query_balance(&market, "ujuno").unwrap();
        let response = app
            .execute_contract(
                resolver.clone(),
                market.clone(),
                &ExecuteMsg::Resolve {},
                &[],
            )
            .unwrap();
        let event = response
            .events
            .iter()
            .find(|event| event.ty == "wasm-juno_pm_v1")
            .expect("stable resolution event");
        for key in [
            "question_id",
            "answer_hex",
            "answer_base64",
            "payout_yes_num",
            "payout_no_num",
            "payout_den",
            "principal_at_resolution",
            "terminal_liability_numerator",
        ] {
            assert!(event
                .attributes
                .iter()
                .any(|attribute| attribute.key == key));
        }
        let after = app.wrap().query_balance(&market, "ujuno").unwrap();
        assert_eq!(after, before, "resolution must not move collateral");
        let resolution: ResolutionResponse = app
            .wrap()
            .query_wasm_smart(&market, &QueryMsg::Resolution {})
            .unwrap();
        assert_eq!(resolution.answer, Some(answer));
        assert_eq!(resolution.payout, Some(expected));
        assert_eq!(resolution.principal_at_resolution, Some(Uint128::new(100)));
        let state: StateResponse = app
            .wrap()
            .query_wasm_smart(&market, &QueryMsg::State {})
            .unwrap();
        assert_eq!(state.status, binary_market::msg::LifecycleStatus::Resolved);
        let repeat = app
            .execute_contract(resolver, market.clone(), &ExecuteMsg::Resolve {}, &[])
            .unwrap_err();
        assert!(!repeat.to_string().is_empty());
        let after_repeat: ResolutionResponse = app
            .wrap()
            .query_wasm_smart(market, &QueryMsg::Resolution {})
            .unwrap();
        assert_eq!(after_repeat, resolution);
    }
}

#[test]
fn finality_boundary_is_exact_and_failed_query_leaves_resolution_empty() {
    let (mut app, market, _, resolver) = setup(bytes(YES_HEX));
    app.update_block(|block| block.time = cosmwasm_std::Timestamp::from_seconds(OPENING + 86_399));
    app.execute_contract(
        resolver.clone(),
        market.clone(),
        &ExecuteMsg::Resolve {},
        &[],
    )
    .unwrap_err();
    let unresolved: ResolutionResponse = app
        .wrap()
        .query_wasm_smart(&market, &QueryMsg::Resolution {})
        .unwrap();
    assert_eq!(unresolved.payout, None);
    assert_eq!(unresolved.principal_at_resolution, None);

    app.update_block(|block| block.time = cosmwasm_std::Timestamp::from_seconds(OPENING + 86_400));
    app.execute_contract(resolver, market.clone(), &ExecuteMsg::Resolve {}, &[])
        .unwrap();
    let resolved: ResolutionResponse = app
        .wrap()
        .query_wasm_smart(market, &QueryMsg::Resolution {})
        .unwrap();
    assert_eq!(
        resolved.payout,
        Some(Payout::for_outcome(pm_types::Outcome::Yes))
    );
}

#[test]
fn failed_redemption_send_rolls_back_burn_and_allows_exact_retry() {
    let (mut app, market, _, resolver) = setup_with_splits(bytes(YES_HEX), &[("alice", 2)]);
    app.update_block(|block| block.time = cosmwasm_std::Timestamp::from_seconds(OPENING + 86_400));
    app.execute_contract(resolver, market.clone(), &ExecuteMsg::Resolve {}, &[])
        .unwrap();
    let position = |app: &App| -> binary_market::msg::PositionResponse {
        app.wrap()
            .query_wasm_smart(
                &market,
                &QueryMsg::Position {
                    address: "alice".into(),
                },
            )
            .unwrap()
    };
    let before = position(&app);

    app.init_modules(|router, _, storage| {
        router.bank.init_balance(storage, &market, vec![]).unwrap();
    });
    assert!(app
        .execute_contract(
            Addr::unchecked("alice"),
            market.clone(),
            &ExecuteMsg::RedeemPositions {
                yes: Uint128::new(2),
                no: Uint128::zero(),
            },
            &[],
        )
        .is_err());
    assert_eq!(position(&app), before);

    app.init_modules(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &market, vec![coin(2, "ujuno")])
            .unwrap();
    });
    app.execute_contract(
        Addr::unchecked("alice"),
        market.clone(),
        &ExecuteMsg::RedeemPositions {
            yes: Uint128::new(2),
            no: Uint128::zero(),
        },
        &[],
    )
    .unwrap();
    let after_retry = position(&app);
    assert_eq!(after_retry.yes, Uint128::zero());
    assert_eq!(after_retry.no, Uint128::new(2));
}
