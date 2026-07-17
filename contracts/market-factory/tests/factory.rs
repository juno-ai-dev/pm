use binary_market::{
    contract::{
        execute as market_execute, instantiate as market_instantiate, query as market_query,
        reply as market_reply,
    },
    msg::QueryMsg as MarketQueryMsg,
    question::{ObservationInput, QuestionInput, SourceInput},
};
use cosmwasm_std::{coin, Addr, Empty, HexBinary, Uint128};
use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
use cw_reality::{
    contract::{
        execute as oracle_execute, instantiate as oracle_instantiate, query as oracle_query,
    },
    msg::InstantiateMsg as OracleInstantiateMsg,
};
use market_factory::{
    contract::{execute, instantiate, query, reply},
    msg::{
        ConfigResponse, CreateMarketMsg, ExecuteMsg, InstantiateMsg, ListMarketsResponse,
        MarketResponse, QueryMsg, TierConfig,
    },
};
use pm_types::{ProtocolVersion, TierId, UJUNO_DENOM};

const NOW: u64 = 1_799_800_000;
const DAO: &str = "juno18k65at7fkf8elhece0fnhsvuxggqg6cved6trp5fyk3lftfn93xsmpeaac";

fn oracle_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        oracle_execute,
        oracle_instantiate,
        oracle_query,
    ))
}
fn market_contract() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(market_execute, market_instantiate, market_query)
            .with_reply(market_reply),
    )
}
fn factory_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(execute, instantiate, query).with_reply(reply))
}
fn app() -> App {
    AppBuilder::new().build(|router, _, storage| {
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked("creator"),
                vec![coin(400_000_000, UJUNO_DENOM)],
            )
            .unwrap();
    })
}
fn tier() -> TierConfig {
    TierConfig {
        min_initial_liquidity: Uint128::new(100_000_000),
        max_initial_liquidity: Uint128::new(1_000_000_000),
        min_oracle_bounty: Uint128::new(1_000_000),
        max_oracle_bounty: Uint128::new(10_000_000),
        oracle_initial_bond: Uint128::new(10_000_000),
        answer_timeout_secs: 86_400,
        arbitration_timeout_secs: 1_814_400,
        fee_bps: 200,
        min_trade: Uint128::new(10_000),
        max_trade_bps: 2_500,
        collateral_cap: Uint128::new(10_000_000_000),
        challenge_bond: Uint128::new(10_000_000),
    }
}
fn question() -> QuestionInput {
    QuestionInput {
        title: "Will the specified event occur?".into(),
        proposition: "The event occurs according to the primary source.".into(),
        definitions: vec![],
        invalid_conditions: vec!["The source is unavailable by cutoff.".into()],
        primary_sources: vec![SourceInput {
            publisher: "Example".into(),
            identifier: "event".into(),
            url: "https://example.com/event".into(),
            retrieval: "HTTPS JSON".into(),
            publication_revision_policy: "Use latest before cutoff".into(),
            fallback_condition: "Invalid if unavailable".into(),
        }],
        secondary_sources: vec![],
        source_disagreement_policy: "Primary source controls".into(),
        observation: ObservationInput {
            start_ts: NOW + 86_400,
            end_ts: NOW + 86_400,
            cutoff_ts: NOW + 86_400,
            inclusivity: "inclusive".into(),
            revision_policy: "Latest before cutoff".into(),
        },
    }
}
fn create(nonce: u64) -> CreateMarketMsg {
    CreateMarketMsg {
        question: question(),
        nonce,
        close_ts: NOW + 86_400,
        opening_ts: NOW + 86_400,
        initial_liquidity: Uint128::new(100_000_000),
        oracle_bounty: Uint128::new(1_000_000),
    }
}
fn setup() -> (App, Addr) {
    let mut app = app();
    app.update_block(|b| b.time = cosmwasm_std::Timestamp::from_seconds(NOW));
    let oracle_id = app.store_code(oracle_contract());
    let oracle = app
        .instantiate_contract(
            oracle_id,
            Addr::unchecked("deployer"),
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
    let market_id = app.store_code(market_contract());
    let factory_id = app.store_code(factory_contract());
    let oracle_info = app.wrap().query_wasm_code_info(oracle_id).unwrap();
    let factory = app
        .instantiate_contract(
            factory_id,
            Addr::unchecked("deployer"),
            &InstantiateMsg {
                protocol_version: ProtocolVersion::V1,
                market_code_id: market_id,
                tier_id: TierId(1),
                tier: tier(),
                oracle: oracle.to_string(),
                oracle_code_id: oracle_id,
                oracle_checksum: HexBinary::from(oracle_info.checksum),
                verdict_authority: DAO.into(),
                collateral_denom: UJUNO_DENOM.into(),
                oracle_min_initial_bond_floor: Uint128::new(10_000_000),
                oracle_min_answer_timeout_secs: 86_400,
            },
            &[],
            "factory",
            None,
        )
        .unwrap();
    (app, factory)
}

#[test]
fn permissionless_101_juno_creation_registers_only_activated_adminless_child() {
    let (mut app, factory) = setup();
    app.execute_contract(
        Addr::unchecked("creator"),
        factory.clone(),
        &ExecuteMsg::CreateMarket(create(7)),
        &[coin(101_000_000, UJUNO_DENOM)],
    )
    .unwrap();
    let listed: ListMarketsResponse = app
        .wrap()
        .query_wasm_smart(
            &factory,
            &QueryMsg::ListMarkets {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(listed.markets.len(), 1);
    let record = &listed.markets[0];
    assert_eq!(record.id, 1);
    assert_eq!(record.creator, "creator");
    assert!(record.question_id.is_some());
    let info = app.wrap().query_wasm_contract_info(&record.market).unwrap();
    assert_eq!(info.admin, None);
    let state: binary_market::msg::StateResponse = app
        .wrap()
        .query_wasm_smart(&record.market, &MarketQueryMsg::State {})
        .unwrap();
    assert!(state.activated);
    let config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(&factory, &QueryMsg::Config {})
        .unwrap();
    assert_eq!(config.verdict_authority, DAO);
}

#[test]
fn rejects_bad_funds_bounds_and_metadata_without_registry_entry() {
    let (mut app, factory) = setup();
    for funds in [
        vec![],
        vec![coin(100_999_999, UJUNO_DENOM)],
        vec![coin(101_000_000, "uatom")],
        vec![coin(101_000_000, UJUNO_DENOM), coin(1, "uatom")],
    ] {
        assert!(app
            .execute_contract(
                Addr::unchecked("creator"),
                factory.clone(),
                &ExecuteMsg::CreateMarket(create(1)),
                &funds
            )
            .is_err());
    }
    let mut below = create(2);
    below.initial_liquidity = Uint128::new(99_999_999);
    assert!(app
        .execute_contract(
            Addr::unchecked("creator"),
            factory.clone(),
            &ExecuteMsg::CreateMarket(below),
            &[coin(100_999_999, UJUNO_DENOM)]
        )
        .is_err());
    let mut invalid = create(3);
    invalid.question.title.clear();
    assert!(app
        .execute_contract(
            Addr::unchecked("creator"),
            factory.clone(),
            &ExecuteMsg::CreateMarket(invalid),
            &[coin(101_000_000, UJUNO_DENOM)]
        )
        .is_err());
    let listed: ListMarketsResponse = app
        .wrap()
        .query_wasm_smart(
            factory,
            &QueryMsg::ListMarkets {
                start_after: None,
                limit: Some(1000),
            },
        )
        .unwrap();
    assert!(listed.markets.is_empty());
}

#[test]
fn nested_ask_failure_rolls_back_and_pagination_is_bounded_deterministic() {
    let (mut app, factory) = setup();
    let mut duplicate = create(9);
    app.execute_contract(
        Addr::unchecked("creator"),
        factory.clone(),
        &ExecuteMsg::CreateMarket(duplicate.clone()),
        &[coin(101_000_000, UJUNO_DENOM)],
    )
    .unwrap();
    // Same market-owned question cannot collide because each child address is part of its identity.
    duplicate.nonce = 10;
    app.execute_contract(
        Addr::unchecked("creator"),
        factory.clone(),
        &ExecuteMsg::CreateMarket(duplicate),
        &[coin(101_000_000, UJUNO_DENOM)],
    )
    .unwrap();
    let first: ListMarketsResponse = app
        .wrap()
        .query_wasm_smart(
            &factory,
            &QueryMsg::ListMarkets {
                start_after: None,
                limit: Some(1),
            },
        )
        .unwrap();
    let second: ListMarketsResponse = app
        .wrap()
        .query_wasm_smart(
            &factory,
            &QueryMsg::ListMarkets {
                start_after: Some(first.markets[0].id),
                limit: Some(1),
            },
        )
        .unwrap();
    assert_eq!((first.markets[0].id, second.markets[0].id), (1, 2));
    let by_id: MarketResponse = app
        .wrap()
        .query_wasm_smart(&factory, &QueryMsg::Market { id: 2 })
        .unwrap();
    assert_eq!(by_id.market.id, 2);
    let capped: ListMarketsResponse = app
        .wrap()
        .query_wasm_smart(
            factory,
            &QueryMsg::ListMarkets {
                start_after: None,
                limit: Some(u32::MAX),
            },
        )
        .unwrap();
    assert_eq!(capped.markets.len(), 2);
}
