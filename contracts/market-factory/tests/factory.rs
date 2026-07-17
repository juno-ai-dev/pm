use binary_market::{
    contract::{
        execute as market_execute, instantiate as market_instantiate, query as market_query,
        reply as market_reply,
    },
    msg::{ConfigResponse as ChildConfig, QueryMsg as MarketQueryMsg},
    question::{self, ObservationInput, QuestionInput, SourceInput},
};
use cosmwasm_std::{
    coin, from_json, to_json_binary, Addr, Binary, Deps, DepsMut, Empty, Env, HexBinary,
    MessageInfo, Response, StdError, StdResult, Uint128,
};
use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
use cw_reality::{
    contract::{
        execute as oracle_execute, instantiate as oracle_instantiate, query as oracle_query,
    },
    error::ContractError as OracleError,
    msg::{ExecuteMsg as OracleExecuteMsg, InstantiateMsg as OracleInstantiateMsg},
};
use market_factory::{
    contract::{execute, instantiate, query, reply},
    msg::{
        ConfigResponse, CreateMarketMsg, ExecuteMsg, InstantiateMsg, ListMarketsResponse,
        MarketResponse, NextNonceResponse, QueryMsg, TierConfig,
    },
};
use pm_types::{ProtocolVersion, TierId, UJUNO_DENOM};

const NOW: u64 = 1_799_800_000;
const DAO: &str = "juno18k65at7fkf8elhece0fnhsvuxggqg6cved6trp5fyk3lftfn93xsmpeaac";
const PRINCIPAL: u128 = 100_000_000;
const BOUNTY: u128 = 1_000_000;

fn oracle_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        oracle_execute,
        oracle_instantiate,
        oracle_query,
    ))
}

fn reject_ask(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: OracleExecuteMsg,
) -> Result<Response, OracleError> {
    if matches!(msg, OracleExecuteMsg::AskQuestion { .. }) {
        return Err(StdError::generic_err("ask disabled").into());
    }
    oracle_execute(deps, env, info, msg)
}

fn rejecting_oracle_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        reject_ask,
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

fn spoof_identity_query(deps: Deps, env: Env, parsed: MarketQueryMsg) -> StdResult<Binary> {
    if matches!(parsed, MarketQueryMsg::Identity {}) {
        return to_json_binary(&binary_market::msg::IdentityResponse {
            protocol_version: ProtocolVersion::V1,
            factory: "spoofed-factory".into(),
            market: env.contract.address.to_string(),
            nonce: 0,
            question_id: Some(Binary::from(vec![7; 32])),
        });
    }
    market_query(deps, env, parsed)
}

fn spoof_identity_nonce_query(deps: Deps, env: Env, parsed: MarketQueryMsg) -> StdResult<Binary> {
    if matches!(parsed, MarketQueryMsg::Identity {}) {
        let mut identity: binary_market::msg::IdentityResponse =
            from_json(market_query(deps, env.clone(), parsed)?)?;
        identity.nonce = identity.nonce.saturating_add(1);
        return to_json_binary(&identity);
    }
    market_query(deps, env, parsed)
}

fn spoof_config_query(deps: Deps, env: Env, parsed: MarketQueryMsg) -> StdResult<Binary> {
    if matches!(parsed, MarketQueryMsg::Config {}) {
        let mut config: ChildConfig =
            from_json(market_query(deps, env.clone(), MarketQueryMsg::Config {})?)?;
        config.oracle = "wrong-oracle".into();
        config.fee_bps = 999;
        return to_json_binary(&config);
    }
    market_query(deps, env, parsed)
}

fn spoof_question_query(deps: Deps, env: Env, parsed: MarketQueryMsg) -> StdResult<Binary> {
    if matches!(parsed, MarketQueryMsg::Question {}) {
        let mut value: binary_market::msg::QuestionResponse = from_json(market_query(
            deps,
            env.clone(),
            MarketQueryMsg::Question {},
        )?)?;
        value.nonce = value.nonce.saturating_add(1);
        return to_json_binary(&value);
    }
    market_query(deps, env, parsed)
}

fn spoof_market_contract(
    query_fn: fn(Deps, Env, MarketQueryMsg) -> StdResult<Binary>,
) -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(market_execute, market_instantiate, query_fn).with_reply(market_reply),
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
                vec![coin(20_000_000_000, UJUNO_DENOM), coin(10, "uatom")],
            )
            .unwrap();
    })
}

fn tier() -> TierConfig {
    TierConfig {
        min_initial_liquidity: Uint128::new(100_000_000),
        max_initial_liquidity: Uint128::new(200_000_000),
        min_oracle_bounty: Uint128::new(1_000_000),
        max_oracle_bounty: Uint128::new(1_000_000),
        oracle_initial_bond: Uint128::new(10_000_000),
        answer_timeout_secs: question::ANSWER_TIMEOUT_SECS,
        arbitration_timeout_secs: question::ARBITRATION_TIMEOUT_SECS,
        fee_bps: 200,
        min_trade: Uint128::new(10_000),
        max_trade_bps: 2_500,
        max_position_per_side: Uint128::new(20_000_000),
        collateral_cap: Uint128::new(200_000_000),
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

fn create() -> CreateMarketMsg {
    CreateMarketMsg {
        question: question(),
        close_ts: NOW + 86_400,
        opening_ts: NOW + 86_400,
        initial_liquidity: Uint128::new(PRINCIPAL),
        oracle_bounty: Uint128::new(BOUNTY),
    }
}

struct Codes {
    oracle_id: u64,
    oracle_checksum: HexBinary,
    market_id: u64,
    market_checksum: HexBinary,
    factory_id: u64,
}

fn store(
    app: &mut App,
    oracle: Box<dyn Contract<Empty>>,
    market: Box<dyn Contract<Empty>>,
) -> Codes {
    let oracle_id = app.store_code(oracle);
    let market_id = app.store_code(market);
    let factory_id = app.store_code(factory_contract());
    Codes {
        oracle_id,
        oracle_checksum: app.wrap().query_wasm_code_info(oracle_id).unwrap().checksum,
        market_id,
        market_checksum: app.wrap().query_wasm_code_info(market_id).unwrap().checksum,
        factory_id,
    }
}

fn oracle_msg() -> OracleInstantiateMsg {
    OracleInstantiateMsg {
        admin: None,
        min_initial_bond_floor: Uint128::new(10_000_000),
        min_answer_timeout_secs: question::ANSWER_TIMEOUT_SECS,
    }
}

fn factory_msg(codes: &Codes, oracle: &Addr) -> InstantiateMsg {
    InstantiateMsg {
        protocol_version: ProtocolVersion::V1,
        market_code_id: codes.market_id,
        market_checksum: codes.market_checksum.clone(),
        tier_id: TierId(1),
        tier: tier(),
        oracle: oracle.to_string(),
        oracle_code_id: codes.oracle_id,
        oracle_checksum: codes.oracle_checksum.clone(),
        verdict_authority: DAO.into(),
        collateral_denom: UJUNO_DENOM.into(),
        oracle_min_initial_bond_floor: Uint128::new(10_000_000),
        oracle_min_answer_timeout_secs: question::ANSWER_TIMEOUT_SECS,
    }
}

fn setup_custom(
    oracle_contract: Box<dyn Contract<Empty>>,
    market_contract: Box<dyn Contract<Empty>>,
) -> (App, Addr) {
    let mut app = app();
    app.update_block(|b| b.time = cosmwasm_std::Timestamp::from_seconds(NOW));
    let codes = store(&mut app, oracle_contract, market_contract);
    let oracle = app
        .instantiate_contract(
            codes.oracle_id,
            Addr::unchecked("deployer"),
            &oracle_msg(),
            &[],
            "oracle",
            None,
        )
        .unwrap();
    let factory = app
        .instantiate_contract(
            codes.factory_id,
            Addr::unchecked("deployer"),
            &factory_msg(&codes, &oracle),
            &[],
            "factory",
            None,
        )
        .unwrap();
    (app, factory)
}

fn setup() -> (App, Addr) {
    setup_custom(oracle_contract(), market_contract())
}

fn execute_create(
    app: &mut App,
    factory: &Addr,
    request: CreateMarketMsg,
) -> Result<cw_multi_test::AppResponse, Box<dyn std::error::Error>> {
    Ok(app.execute_contract(
        Addr::unchecked("creator"),
        factory.clone(),
        &ExecuteMsg::CreateMarket(request.clone()),
        &[coin(
            request.initial_liquidity.u128() + request.oracle_bounty.u128(),
            UJUNO_DENOM,
        )],
    )?)
}

fn list(app: &App, factory: &Addr, limit: Option<u32>) -> ListMarketsResponse {
    app.wrap()
        .query_wasm_smart(
            factory,
            &QueryMsg::ListMarkets {
                start_after_nonce: None,
                limit,
            },
        )
        .unwrap()
}

#[test]
fn exact_profile_factory_nonce_and_identity_rich_events() {
    let (mut app, factory) = setup();
    let initial: NextNonceResponse = app
        .wrap()
        .query_wasm_smart(&factory, &QueryMsg::NextNonce {})
        .unwrap();
    assert_eq!(initial.next_nonce, 0);

    let response = execute_create(&mut app, &factory, create()).unwrap();
    let listed = list(&app, &factory, None);
    assert_eq!(listed.markets.len(), 1);
    assert_eq!(listed.next_start_after_nonce, None);
    let record = &listed.markets[0];
    assert_eq!(record.nonce, 0);
    assert_eq!(record.creator, "creator");
    assert_eq!(record.question_id.len(), 32);
    assert_eq!(record.question_hash.len(), 32);

    let info = app.wrap().query_wasm_contract_info(&record.market).unwrap();
    assert_eq!(info.admin, None);
    let factory_info = app.wrap().query_wasm_contract_info(&factory).unwrap();
    assert_eq!(factory_info.admin, None);

    let state: binary_market::msg::StateResponse = app
        .wrap()
        .query_wasm_smart(&record.market, &MarketQueryMsg::State {})
        .unwrap();
    assert!(state.activated);
    let config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(&factory, &QueryMsg::Config {})
        .unwrap();
    assert_eq!(config.tier, tier());
    assert_eq!(config.verdict_authority, DAO);
    assert_eq!(config.collateral_denom, UJUNO_DENOM);
    assert!(!config.market_checksum.is_empty());
    assert_eq!(info.code_id, config.market_code_id);

    for action in ["market_created", "market_activated"] {
        let event = response
            .events
            .iter()
            .find(|event| {
                event.ty == "wasm-juno_pm_v1"
                    && event
                        .attributes
                        .iter()
                        .any(|a| a.key == "action" && a.value == action)
            })
            .expect("canonical event");
        for key in [
            "protocol_version",
            "factory",
            "market",
            "height",
            "block_time",
            "creator",
        ] {
            assert!(
                event.attributes.iter().any(|a| a.key == key),
                "missing {key} from {action}"
            );
        }
    }
    let activated = response
        .events
        .iter()
        .find(|event| {
            event.ty == "wasm-juno_pm_v1"
                && event
                    .attributes
                    .iter()
                    .any(|a| a.key == "action" && a.value == "market_activated")
        })
        .unwrap();
    for key in [
        "lp",
        "question_id",
        "question_hash",
        "close_ts",
        "opening_ts",
    ] {
        assert!(activated.attributes.iter().any(|a| a.key == key));
    }

    let next: NextNonceResponse = app
        .wrap()
        .query_wasm_smart(&factory, &QueryMsg::NextNonce {})
        .unwrap();
    assert_eq!(next.next_nonce, 1);
}

#[test]
fn accepts_only_even_100_to_200_juno_principal_and_exact_bounty() {
    for principal in [100_000_000, 100_000_002, 200_000_000] {
        let (mut app, factory) = setup();
        let mut request = create();
        request.initial_liquidity = Uint128::new(principal);
        execute_create(&mut app, &factory, request).unwrap();
    }
    for principal in [0, 99_999_999, 100_000_001, 200_000_001, 200_000_002] {
        let (mut app, factory) = setup();
        let mut request = create();
        request.initial_liquidity = Uint128::new(principal);
        assert!(execute_create(&mut app, &factory, request).is_err());
        assert!(list(&app, &factory, None).markets.is_empty());
    }
    for bounty in [0, 999_999, 1_000_001, 10_000_000] {
        let (mut app, factory) = setup();
        let mut request = create();
        request.oracle_bounty = Uint128::new(bounty);
        assert!(execute_create(&mut app, &factory, request).is_err());
    }
}

#[test]
fn rejects_every_non_exact_fund_shape_without_consuming_nonce() {
    let cases = [
        vec![],
        vec![coin(100_999_999, UJUNO_DENOM)],
        vec![coin(101_000_001, UJUNO_DENOM)],
        vec![coin(101_000_000, "uatom")],
        vec![coin(101_000_000, UJUNO_DENOM), coin(1, "uatom")],
    ];
    for funds in cases {
        let (mut app, factory) = setup();
        assert!(app
            .execute_contract(
                Addr::unchecked("creator"),
                factory.clone(),
                &ExecuteMsg::CreateMarket(create()),
                &funds,
            )
            .is_err());
        let next: NextNonceResponse = app
            .wrap()
            .query_wasm_smart(&factory, &QueryMsg::NextNonce {})
            .unwrap();
        assert_eq!(next.next_nonce, 0);
        assert!(list(&app, &factory, None).markets.is_empty());
    }
}

#[test]
fn rejects_invalid_question_fields_and_all_timestamp_orderings() {
    let mut bad = Vec::new();
    let mut r = create();
    r.question.title.clear();
    bad.push(r);
    let mut r = create();
    r.question.proposition.clear();
    bad.push(r);
    let mut r = create();
    r.question.invalid_conditions.clear();
    bad.push(r);
    let mut r = create();
    r.question.primary_sources.clear();
    bad.push(r);
    let mut r = create();
    r.question.primary_sources[0].publisher.clear();
    bad.push(r);
    let mut r = create();
    r.question.primary_sources[0].identifier.clear();
    bad.push(r);
    let mut r = create();
    r.question.primary_sources[0].url = "http://example.com".into();
    bad.push(r);
    let mut r = create();
    r.question.primary_sources[0].retrieval.clear();
    bad.push(r);
    let mut r = create();
    r.question.primary_sources[0]
        .publication_revision_policy
        .clear();
    bad.push(r);
    let mut r = create();
    r.question.primary_sources[0].fallback_condition.clear();
    bad.push(r);
    let mut r = create();
    r.question.source_disagreement_policy.clear();
    bad.push(r);
    let mut r = create();
    r.question.observation.inclusivity.clear();
    bad.push(r);
    let mut r = create();
    r.question.observation.revision_policy.clear();
    bad.push(r);
    let mut r = create();
    r.close_ts = NOW;
    bad.push(r);
    let mut r = create();
    r.close_ts = NOW + question::MIN_CREATION_TO_CLOSE - 1;
    bad.push(r);
    let mut r = create();
    r.close_ts = NOW + question::MAX_CREATION_TO_CLOSE + 1;
    r.opening_ts = r.close_ts;
    r.question.observation.start_ts = r.close_ts;
    r.question.observation.end_ts = r.close_ts;
    r.question.observation.cutoff_ts = r.close_ts;
    bad.push(r);
    let mut r = create();
    r.opening_ts = r.close_ts - 1;
    bad.push(r);
    let mut r = create();
    r.opening_ts = r.close_ts + question::MAX_OPENING_DELAY + 1;
    r.question.observation.cutoff_ts = r.opening_ts;
    bad.push(r);
    let mut r = create();
    r.question.observation.start_ts = r.question.observation.end_ts + 1;
    bad.push(r);
    let mut r = create();
    r.question.observation.end_ts = r.question.observation.cutoff_ts + 1;
    bad.push(r);
    let mut r = create();
    r.question.observation.cutoff_ts = r.close_ts - 1;
    bad.push(r);
    let mut r = create();
    r.opening_ts += 1;
    r.question.observation.cutoff_ts = r.opening_ts + 1;
    bad.push(r);

    for request in bad {
        let (mut app, factory) = setup();
        assert!(execute_create(&mut app, &factory, request).is_err());
        assert!(list(&app, &factory, None).markets.is_empty());
    }
}

#[test]
fn instantiate_rejects_every_profile_deviation_and_weak_oracle() {
    type Mutator = fn(&mut InstantiateMsg);
    let mutators: Vec<Mutator> = vec![
        |m| m.protocol_version = ProtocolVersion::V1, // replaced below with a separate invalid-code check
        |m| m.tier_id = TierId(2),
        |m| m.tier.min_initial_liquidity = Uint128::new(99_999_999),
        |m| m.tier.max_initial_liquidity = Uint128::new(200_000_001),
        |m| m.tier.collateral_cap = Uint128::new(200_000_001),
        |m| m.tier.min_oracle_bounty = Uint128::new(999_999),
        |m| m.tier.max_oracle_bounty = Uint128::new(1_000_001),
        |m| m.tier.oracle_initial_bond = Uint128::new(9_999_999),
        |m| m.tier.answer_timeout_secs += 1,
        |m| m.tier.arbitration_timeout_secs += 1,
        |m| m.tier.fee_bps = 201,
        |m| m.tier.min_trade = Uint128::new(9_999),
        |m| m.tier.max_trade_bps = 2_499,
        |m| m.tier.max_position_per_side = Uint128::new(20_000_001),
        |m| m.tier.challenge_bond = Uint128::new(9_999_999),
        |m| m.oracle_min_initial_bond_floor = Uint128::new(9_999_999),
        |m| m.oracle_min_answer_timeout_secs -= 1,
        |m| m.collateral_denom = "uatom".into(),
        |m| m.verdict_authority = "other-dao".into(),
        |m| m.market_code_id = 0,
        |m| m.oracle_code_id = 0,
        |m| m.market_checksum = HexBinary::default(),
        |m| m.oracle_checksum = HexBinary::default(),
    ];

    for (index, mutate) in mutators.into_iter().enumerate().skip(1) {
        let mut app = app();
        let codes = store(&mut app, oracle_contract(), market_contract());
        let oracle = app
            .instantiate_contract(
                codes.oracle_id,
                Addr::unchecked("deployer"),
                &oracle_msg(),
                &[],
                "oracle",
                None,
            )
            .unwrap();
        let mut msg = factory_msg(&codes, &oracle);
        mutate(&mut msg);
        assert!(
            app.instantiate_contract(
                codes.factory_id,
                Addr::unchecked("deployer"),
                &msg,
                &[],
                format!("factory-{index}"),
                None
            )
            .is_err(),
            "profile mutation {index} accepted"
        );
    }

    for weak in [
        OracleInstantiateMsg {
            admin: None,
            min_initial_bond_floor: Uint128::new(9_999_999),
            min_answer_timeout_secs: question::ANSWER_TIMEOUT_SECS,
        },
        OracleInstantiateMsg {
            admin: None,
            min_initial_bond_floor: Uint128::new(10_000_000),
            min_answer_timeout_secs: question::ANSWER_TIMEOUT_SECS - 1,
        },
        OracleInstantiateMsg {
            admin: Some("stored-admin".into()),
            min_initial_bond_floor: Uint128::new(10_000_000),
            min_answer_timeout_secs: question::ANSWER_TIMEOUT_SECS,
        },
    ] {
        let mut app = app();
        let codes = store(&mut app, oracle_contract(), market_contract());
        let oracle = app
            .instantiate_contract(
                codes.oracle_id,
                Addr::unchecked("deployer"),
                &weak,
                &[],
                "weak-oracle",
                None,
            )
            .unwrap();
        assert!(app
            .instantiate_contract(
                codes.factory_id,
                Addr::unchecked("deployer"),
                &factory_msg(&codes, &oracle),
                &[],
                "factory",
                None
            )
            .is_err());
    }
}

#[test]
fn rejects_wrong_oracle_code_checksum_admin_and_market_checksum() {
    let mut app = app();
    let codes = store(&mut app, oracle_contract(), market_contract());
    let other_oracle_id = app.store_code(oracle_contract());
    let oracle = app
        .instantiate_contract(
            codes.oracle_id,
            Addr::unchecked("deployer"),
            &oracle_msg(),
            &[],
            "oracle",
            None,
        )
        .unwrap();

    let mut wrong_code = factory_msg(&codes, &oracle);
    wrong_code.oracle_code_id = other_oracle_id;
    wrong_code.oracle_checksum = app
        .wrap()
        .query_wasm_code_info(other_oracle_id)
        .unwrap()
        .checksum;
    assert!(app
        .instantiate_contract(
            codes.factory_id,
            Addr::unchecked("deployer"),
            &wrong_code,
            &[],
            "wrong-code",
            None
        )
        .is_err());

    let mut wrong_checksum = factory_msg(&codes, &oracle);
    wrong_checksum.oracle_checksum = HexBinary::from(vec![7; 32]);
    assert!(app
        .instantiate_contract(
            codes.factory_id,
            Addr::unchecked("deployer"),
            &wrong_checksum,
            &[],
            "wrong-checksum",
            None
        )
        .is_err());

    let mut wrong_market_checksum = factory_msg(&codes, &oracle);
    wrong_market_checksum.market_checksum = HexBinary::from(vec![8; 32]);
    assert!(app
        .instantiate_contract(
            codes.factory_id,
            Addr::unchecked("deployer"),
            &wrong_market_checksum,
            &[],
            "wrong-market-checksum",
            None
        )
        .is_err());

    let administered = app
        .instantiate_contract(
            codes.oracle_id,
            Addr::unchecked("deployer"),
            &oracle_msg(),
            &[],
            "admin-oracle",
            Some("chain-admin".into()),
        )
        .unwrap();
    assert!(app
        .instantiate_contract(
            codes.factory_id,
            Addr::unchecked("deployer"),
            &factory_msg(&codes, &administered),
            &[],
            "admin-factory",
            None
        )
        .is_err());

    assert!(app
        .instantiate_contract(
            codes.factory_id,
            Addr::unchecked("creator"),
            &factory_msg(&codes, &oracle),
            &[coin(1, UJUNO_DENOM)],
            "funded-factory",
            None
        )
        .is_err());
}

#[test]
fn nested_ask_failure_rolls_back_child_funds_registry_and_nonce() {
    let (mut app, factory) = setup_custom(rejecting_oracle_contract(), market_contract());
    let creator = Addr::unchecked("creator");
    let before = app.wrap().query_balance(&creator, UJUNO_DENOM).unwrap();
    assert!(execute_create(&mut app, &factory, create()).is_err());
    let after = app.wrap().query_balance(&creator, UJUNO_DENOM).unwrap();
    assert_eq!(before, after);
    assert!(list(&app, &factory, None).markets.is_empty());
    let next: NextNonceResponse = app
        .wrap()
        .query_wasm_smart(&factory, &QueryMsg::NextNonce {})
        .unwrap();
    assert_eq!(next.next_nonce, 0);
}

#[test]
fn wrong_child_identity_config_or_question_reverts_everything() {
    for child in [
        spoof_market_contract(spoof_identity_query),
        spoof_market_contract(spoof_identity_nonce_query),
        spoof_market_contract(spoof_config_query),
        spoof_market_contract(spoof_question_query),
    ] {
        let (mut app, factory) = setup_custom(oracle_contract(), child);
        let creator = Addr::unchecked("creator");
        let before = app.wrap().query_balance(&creator, UJUNO_DENOM).unwrap();
        assert!(execute_create(&mut app, &factory, create()).is_err());
        assert_eq!(
            before,
            app.wrap().query_balance(&creator, UJUNO_DENOM).unwrap()
        );
        assert!(list(&app, &factory, None).markets.is_empty());
        let next: NextNonceResponse = app
            .wrap()
            .query_wasm_smart(&factory, &QueryMsg::NextNonce {})
            .unwrap();
        assert_eq!(next.next_nonce, 0);
    }
}

#[test]
fn identity_nonce_mismatch_is_rejected_atomically() {
    let (mut app, factory) = setup_custom(
        oracle_contract(),
        spoof_market_contract(spoof_identity_nonce_query),
    );
    let creator = Addr::unchecked("creator");
    let before = app.wrap().query_balance(&creator, UJUNO_DENOM).unwrap();
    let result = execute_create(&mut app, &factory, create());
    assert!(result.is_err(), "identity nonce mismatch was accepted");
    assert_eq!(
        before,
        app.wrap().query_balance(&creator, UJUNO_DENOM).unwrap()
    );
    assert!(list(&app, &factory, None).markets.is_empty());
    let next: NextNonceResponse = app
        .wrap()
        .query_wasm_smart(&factory, &QueryMsg::NextNonce {})
        .unwrap();
    assert_eq!(next.next_nonce, 0);
}

#[test]
fn pagination_default_max_zero_rejection_and_next_cursor() {
    let (mut app, factory) = setup();
    for _ in 0..101 {
        execute_create(&mut app, &factory, create()).unwrap();
    }
    let first: ListMarketsResponse = app
        .wrap()
        .query_wasm_smart(
            &factory,
            &QueryMsg::ListMarkets {
                start_after_nonce: None,
                limit: Some(2),
            },
        )
        .unwrap();
    assert_eq!(
        first.markets.iter().map(|m| m.nonce).collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert_eq!(first.next_start_after_nonce, Some(1));
    let second: ListMarketsResponse = app
        .wrap()
        .query_wasm_smart(
            &factory,
            &QueryMsg::ListMarkets {
                start_after_nonce: first.next_start_after_nonce,
                limit: Some(2),
            },
        )
        .unwrap();
    assert_eq!(
        second.markets.iter().map(|m| m.nonce).collect::<Vec<_>>(),
        vec![2, 3]
    );
    assert_eq!(second.next_start_after_nonce, Some(3));

    let default = list(&app, &factory, None);
    assert_eq!(default.markets.len(), 50);
    assert_eq!(default.next_start_after_nonce, Some(49));
    let capped = list(&app, &factory, Some(u32::MAX));
    assert_eq!(capped.markets.len(), 100);
    assert_eq!(capped.next_start_after_nonce, Some(99));
    assert!(app
        .wrap()
        .query_wasm_smart::<ListMarketsResponse>(
            &factory,
            &QueryMsg::ListMarkets {
                start_after_nonce: None,
                limit: Some(0)
            }
        )
        .is_err());

    let by_nonce: MarketResponse = app
        .wrap()
        .query_wasm_smart(&factory, &QueryMsg::Market { nonce: 2 })
        .unwrap();
    assert_eq!(by_nonce.market.nonce, 2);
}
