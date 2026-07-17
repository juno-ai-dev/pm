use binary_market::{
    contract::{execute, instantiate, query, reply},
    msg::{InstantiateMsg, QueryMsg, QuestionResponse, StateResponse},
    question::{
        canonical_question, question_id_from_canonical, ObservationInput, QuestionInput,
        SourceInput,
    },
};
use cosmwasm_std::{coin, Addr, Empty, Uint128};
use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
use cw_reality::{
    msg::{
        InstantiateMsg as OracleInstantiateMsg, QueryMsg as OracleQueryMsg,
        QuestionResponse as OracleQuestionResponse,
    },
    state::{AnswerType, State as OracleState},
};
use pm_types::TierId;

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
        title: "Example outcome?".into(),
        proposition: "Will the published example outcome be yes?".into(),
        definitions: vec!["Published means present in the named JSON feed.".into()],
        invalid_conditions: vec!["The source is permanently unavailable.".into()],
        primary_sources: vec![SourceInput {
            publisher: "Example Authority".into(),
            identifier: "dataset/example/final".into(),
            url: "https://example.com/final".into(),
            retrieval: "HTTPS JSON".into(),
            publication_revision_policy: "Corrections before opening control.".into(),
            fallback_condition: "Unavailable for 72 hours.".into(),
        }],
        secondary_sources: vec![],
        source_disagreement_policy: "The first available primary source controls.".into(),
        observation: ObservationInput {
            start_ts: 1_800_000_000,
            end_ts: 1_800_086_400,
            cutoff_ts: 1_800_086_400,
            inclusivity: "inclusive".into(),
            revision_policy: "Corrections published before opening control.".into(),
        },
    }
}

#[test]
fn python_jcs_and_canonical_address_length_vectors_match() {
    let (text, hash) = canonical_question(
        &question(),
        &Addr::unchecked("market"),
        &Addr::unchecked("oracle"),
        &Addr::unchecked("governance"),
        1_800_000_000,
        1_800_086_400,
        Uint128::new(10_000_000),
        1_799_800_000,
    )
    .unwrap();
    assert_eq!(
        text,
        include_str!("fixtures/question-python-jcs.json").trim_end()
    );
    assert_eq!(
        hex::encode(hash.as_slice()),
        "32d83b69bf05ee6756537ae42aec171a9766de7b17e84a921cb646ddf7677866"
    );
    let content_hash: [u8; 32] = hash.as_slice().try_into().unwrap();
    for (length, expected) in [
        (
            20usize,
            "60705ad7ffcc9b3447d35053343d1e89b6559c5bb42da0d1fddf095fbfe5f1e9",
        ),
        (
            32usize,
            "f1dd19b4b83108b4073d0a2469f93bb0c88cc850436774aeed73e60cc20b3e3e",
        ),
    ] {
        let oracle: Vec<u8> = (0..length as u8).collect();
        let market: Vec<u8> = (0..length as u8).map(|n| 255 - n).collect();
        let id = question_id_from_canonical(
            &oracle,
            &market,
            7,
            &content_hash,
            &market,
            86_400,
            Uint128::new(10_000_000),
            1_800_086_400,
        );
        assert_eq!(hex::encode(id), expected);
    }

    let canonical = vec![3u8; 32];
    let id = question_id_from_canonical(
        &canonical,
        &canonical,
        7,
        &content_hash,
        &canonical,
        86_400,
        Uint128::new(10_000_000),
        1_800_086_400,
    );
    let collision = question_id_from_canonical(
        &canonical,
        &canonical,
        7,
        &content_hash,
        &canonical,
        86_400,
        Uint128::new(10_000_000),
        1_800_086_400,
    );
    let distinct_nonce = question_id_from_canonical(
        &canonical,
        &canonical,
        8,
        &content_hash,
        &canonical,
        86_400,
        Uint128::new(10_000_000),
        1_800_086_400,
    );
    assert_eq!(id, collision, "the oracle rejects this duplicate ID");
    assert_ne!(
        id, distinct_nonce,
        "a distinct nonce must avoid the collision"
    );
}

#[test]
fn instantiate_asks_and_exact_reply_activates_atomically() {
    let factory = Addr::unchecked("factory");
    let mut app: App = AppBuilder::new().build(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &factory, vec![coin(2_000_000, "ujuno")])
            .unwrap();
    });
    app.update_block(|block| block.time = cosmwasm_std::Timestamp::from_seconds(1_799_800_000));
    let oracle_code = app.store_code(oracle_contract());
    let oracle = app
        .instantiate_contract(
            oracle_code,
            factory.clone(),
            &OracleInstantiateMsg {
                admin: None,
                min_initial_bond_floor: Uint128::new(10_000_000),
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
                nonce: 0,
                close_ts: 1_800_000_000,
                opening_ts: 1_800_086_400,
                initial_liquidity: Uint128::new(100),
                oracle_bounty: Uint128::new(1_000_000),
                oracle_initial_bond: Uint128::new(10_000_000),
                answer_timeout_secs: 86_400,
                arbitration_timeout_secs: 1_814_400,
                fee_bps: 200,
                min_trade: Uint128::one(),
                max_trade_bps: 2_500,
                collateral_cap: Uint128::new(10_000),
                challenge_bond: Uint128::new(10_000_000),
            },
            &[coin(1_000_100, "ujuno")],
            "market",
            None,
        )
        .unwrap();

    let state: StateResponse = app
        .wrap()
        .query_wasm_smart(&market, &QueryMsg::State {})
        .unwrap();
    assert!(state.activated);
    let bound: QuestionResponse = app
        .wrap()
        .query_wasm_smart(&market, &QueryMsg::Question {})
        .unwrap();
    let id = bound.question_id.clone().expect("activation stores id");
    let oracle_question: OracleQuestionResponse = app
        .wrap()
        .query_wasm_smart(
            &oracle,
            &OracleQueryMsg::Question {
                question_id: id.clone(),
            },
        )
        .unwrap();
    assert_eq!(oracle_question.question_id, id);
    assert_eq!(oracle_question.state, OracleState::OpenUnanswered);
    assert_eq!(oracle_question.question.asker, market);
    assert_eq!(oracle_question.question.answer_type, AnswerType::Bool);
    assert_eq!(
        oracle_question.question.text.as_bytes(),
        bound.text.as_bytes()
    );
    assert_eq!(oracle_question.question.bounty, Uint128::new(1_000_000));
}

#[test]
fn rejected_oracle_ask_rolls_back_question_and_funds() {
    let factory = Addr::unchecked("factory");
    let mut app: App = AppBuilder::new().build(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &factory, vec![coin(2_000_000, "ujuno")])
            .unwrap();
    });
    app.update_block(|block| block.time = cosmwasm_std::Timestamp::from_seconds(1_799_800_000));
    let oracle_code = app.store_code(oracle_contract());
    let oracle = app
        .instantiate_contract(
            oracle_code,
            factory.clone(),
            &OracleInstantiateMsg {
                admin: None,
                min_initial_bond_floor: Uint128::new(20_000_000),
                min_answer_timeout_secs: 86_400,
            },
            &[],
            "rejecting oracle",
            None,
        )
        .unwrap();
    let market_code = app.store_code(market_contract());
    let before = app.wrap().query_balance(&factory, "ujuno").unwrap();
    let err = app
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
                nonce: 0,
                close_ts: 1_800_000_000,
                opening_ts: 1_800_086_400,
                initial_liquidity: Uint128::new(100),
                oracle_bounty: Uint128::new(1_000_000),
                oracle_initial_bond: Uint128::new(10_000_000),
                answer_timeout_secs: 86_400,
                arbitration_timeout_secs: 1_814_400,
                fee_bps: 200,
                min_trade: Uint128::one(),
                max_trade_bps: 2_500,
                collateral_cap: Uint128::new(10_000),
                challenge_bond: Uint128::new(10_000_000),
            },
            &[coin(1_000_100, "ujuno")],
            "market",
            None,
        )
        .unwrap_err();
    assert!(err.to_string().contains("initial_bond"));
    assert_eq!(app.wrap().query_balance(&factory, "ujuno").unwrap(), before);
    let questions: cw_reality::msg::QuestionsListResponse = app
        .wrap()
        .query_wasm_smart(
            oracle,
            &OracleQueryMsg::List {
                start_after: None,
                limit: None,
                status: None,
            },
        )
        .unwrap();
    assert!(questions.questions.is_empty());
}
