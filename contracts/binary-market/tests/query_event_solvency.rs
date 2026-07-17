use binary_market::{
    contract::{execute, instantiate, query, reply},
    msg::{
        AccountingResponse, ChallengeResponse, ExecuteMsg, IdentityResponse, InstantiateMsg,
        LpPositionResponse, PositionResponse, QueryMsg, QuoteResponse, ResolutionResponse,
        SolvencyResponse,
    },
    question::{ObservationInput, QuestionInput, SourceInput},
    state::{self, Accounting, Config, Lifecycle},
};
use cosmwasm_std::{coin, Addr, Binary, DepsMut, Empty, Env, MessageInfo, Response, Uint128};
use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
use cw_reality::msg::InstantiateMsg as OracleInstantiateMsg;
use pm_types::{Outcome, Payout, ProtocolVersion, TierId};
use serde_json::{json, Value};

const NOW: u64 = 1_799_800_000;
const CLOSE: u64 = 1_800_000_000;
const INITIAL: u128 = 100_000_000;

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
        title: "Query contract?".into(),
        proposition: "Will direct queries remain authoritative?".into(),
        definitions: vec![],
        invalid_conditions: vec!["The test is withdrawn.".into()],
        primary_sources: vec![SourceInput {
            publisher: "Juno PM".into(),
            identifier: "tests/query-event-solvency".into(),
            url: "https://example.com/query".into(),
            retrieval: "HTTPS JSON".into(),
            publication_revision_policy: "Latest before opening controls.".into(),
            fallback_condition: "Unavailable for 72 hours.".into(),
        }],
        secondary_sources: vec![],
        source_disagreement_policy: "The primary source controls.".into(),
        observation: ObservationInput {
            start_ts: CLOSE,
            end_ts: CLOSE + 86_400,
            cutoff_ts: CLOSE + 86_400,
            inclusivity: "inclusive".into(),
            revision_policy: "Corrections before opening control.".into(),
        },
    }
}

fn setup() -> (App, Addr) {
    let factory = Addr::unchecked("factory");
    let mut app = AppBuilder::new().build(|router, _, storage| {
        for (owner, amount) in [("factory", 500_000_000), ("alice", 500_000_000)] {
            router
                .bank
                .init_balance(
                    storage,
                    &Addr::unchecked(owner),
                    vec![coin(amount, "ujuno")],
                )
                .unwrap();
        }
    });
    app.update_block(|block| {
        block.height = 12_345;
        block.time = cosmwasm_std::Timestamp::from_seconds(NOW);
    });
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
                nonce: 45,
                close_ts: CLOSE,
                opening_ts: CLOSE + 86_400,
                initial_liquidity: Uint128::new(INITIAL),
                oracle_bounty: Uint128::new(1_000_000),
                oracle_initial_bond: Uint128::new(10_000_000),
                answer_timeout_secs: 86_400,
                arbitration_timeout_secs: 1_814_400,
                fee_bps: 200,
                min_trade: Uint128::new(10_000),
                max_trade_bps: 2_500,
                max_position_per_side: Uint128::MAX,
                collateral_cap: Uint128::new(200_000_000),
                challenge_bond: Uint128::new(10_000_000),
            },
            &[coin(INITIAL + 1_000_000, "ujuno")],
            "market",
            None,
        )
        .unwrap();
    (app, market)
}

fn event<'a>(response: &'a cw_multi_test::AppResponse, action: &str) -> &'a cosmwasm_std::Event {
    response
        .events
        .iter()
        .find(|event| {
            event.ty.ends_with("juno_pm_v1")
                && event
                    .attributes
                    .iter()
                    .any(|attribute| attribute.key == "action" && attribute.value == action)
        })
        .unwrap_or_else(|| panic!("missing {action} event"))
}

fn attr<'a>(event: &'a cosmwasm_std::Event, key: &str) -> &'a str {
    event
        .attributes
        .iter()
        .find(|attribute| attribute.key == key)
        .unwrap_or_else(|| panic!("missing {key}"))
        .value
        .as_str()
}

#[test]
fn missing_records_have_documented_zero_or_none_semantics_and_identity_is_canonical() {
    let (app, market) = setup();
    let missing: PositionResponse = app
        .wrap()
        .query_wasm_smart(
            &market,
            &QueryMsg::Position {
                address: "never-seen".into(),
            },
        )
        .unwrap();
    assert_eq!(missing.address, "never-seen");
    assert_eq!(
        (missing.yes, missing.no),
        (Uint128::zero(), Uint128::zero())
    );

    let challenge: ChallengeResponse = app
        .wrap()
        .query_wasm_smart(&market, &QueryMsg::Challenge {})
        .unwrap();
    assert_eq!(challenge.challenger, None);
    assert_eq!(challenge.answer, None);
    assert_eq!(challenge.answer_hex, None);
    assert_eq!(challenge.answer_base64, None);
    assert_eq!(challenge.oracle_bond, None);
    assert_eq!(challenge.challenge_bond, Uint128::zero());
    assert_eq!(challenge.oracle_snapshot, None);

    let resolution: ResolutionResponse = app
        .wrap()
        .query_wasm_smart(&market, &QueryMsg::Resolution {})
        .unwrap();
    assert_eq!(resolution.answer, None);
    assert_eq!(resolution.answer_hex, None);
    assert_eq!(resolution.answer_base64, None);
    assert_eq!(resolution.payout, None);
    assert_eq!(resolution.terminal_liability_twice, None);

    let lp: LpPositionResponse = app
        .wrap()
        .query_wasm_smart(&market, &QueryMsg::LpPosition {})
        .unwrap();
    assert_eq!(lp.owner, "creator");
    assert_eq!(lp.supply, Uint128::new(INITIAL));
    assert_eq!(lp.burned, Uint128::zero());
    assert_eq!(lp.later_accrual, Uint128::zero());

    let identity: IdentityResponse = app
        .wrap()
        .query_wasm_smart(&market, &QueryMsg::Identity {})
        .unwrap();
    assert_eq!(identity.protocol_version, ProtocolVersion::V1);
    assert_eq!(identity.factory, "factory");
    assert_eq!(identity.market, market);
    assert_eq!(identity.nonce, 45);
    assert!(identity.question_id.is_some());

    let invalid = app.wrap().query_wasm_smart::<PositionResponse>(
        &market,
        &QueryMsg::Position {
            address: "x".into(),
        },
    );
    assert!(
        invalid.is_err(),
        "invalid addresses must not look like zero balances"
    );
}

#[test]
fn quote_wire_is_exact_and_execute_event_reconciles_byte_for_byte() {
    let (mut app, market) = setup();
    let quote: QuoteResponse = app
        .wrap()
        .query_wasm_smart(
            &market,
            &QueryMsg::QuoteBuy {
                outcome: Outcome::Yes,
                gross: Uint128::new(10_000_000),
            },
        )
        .unwrap();
    assert_eq!(quote.height, app.block_info().height);
    assert_eq!(quote.block_time, NOW);
    assert_eq!(quote.gross, Uint128::new(10_000_000));
    assert_eq!(quote.net, Uint128::new(9_800_000));
    assert_eq!(quote.fee, Uint128::new(200_000));
    assert_eq!(quote.output, Uint128::new(18_725_318));
    assert_eq!(quote.min_out, Some(quote.output));
    assert_eq!(quote.max_in, None);
    assert_eq!(
        quote.average_price.numerator.to_string(),
        quote.gross.to_string()
    );
    assert_eq!(
        quote.average_price.denominator.to_string(),
        quote.output.to_string()
    );
    assert_eq!(quote.fee_rate.numerator.to_string(), quote.fee.to_string());
    assert_eq!(
        quote.fee_rate.denominator.to_string(),
        quote.gross.to_string()
    );

    let wire: Value = app
        .wrap()
        .query_wasm_smart(
            &market,
            &QueryMsg::QuoteBuy {
                outcome: Outcome::Yes,
                gross: Uint128::new(10_000_000),
            },
        )
        .unwrap();
    assert_eq!(wire["block_time"], json!(NOW));
    assert_eq!(wire["gross"], json!("10000000"));
    assert_eq!(
        wire["average_price"],
        json!({"numerator":"10000000","denominator":"18725318"})
    );
    assert_eq!(wire["min_out"], json!("18725318"));
    assert!(wire.get("max_in").is_none());
    for forbidden in [
        "probability",
        "time",
        "average_price_bps",
        "price_impact_bps",
    ] {
        assert!(
            wire.get(forbidden).is_none(),
            "forbidden/legacy field {forbidden}"
        );
    }

    let response = app
        .execute_contract(
            Addr::unchecked("alice"),
            market.clone(),
            &ExecuteMsg::Buy {
                outcome: Outcome::Yes,
                min_out: quote.min_out.unwrap(),
                deadline: quote.block_time,
            },
            &[coin(quote.gross.u128(), "ujuno")],
        )
        .unwrap();
    let trade = event(&response, "trade");
    assert_eq!(attr(trade, "protocol_version"), "1");
    assert_eq!(attr(trade, "factory"), "factory");
    assert_eq!(attr(trade, "market"), market);
    assert_eq!(attr(trade, "height"), quote.height.to_string());
    assert_eq!(attr(trade, "block_time"), quote.block_time.to_string());
    for (key, expected) in [
        ("gross", quote.gross),
        ("net", quote.net),
        ("fee", quote.fee),
        ("input", quote.input),
        ("output", quote.output),
        ("reserve_yes_before", quote.reserve_yes_before),
        ("reserve_no_before", quote.reserve_no_before),
        ("reserve_yes_after", quote.reserve_yes_after),
        ("reserve_no_after", quote.reserve_no_after),
    ] {
        assert_eq!(attr(trade, key), expected.to_string());
    }

    let sell: QuoteResponse = app
        .wrap()
        .query_wasm_smart(
            &market,
            &QueryMsg::QuoteSell {
                outcome: Outcome::Yes,
                return_amount: Uint128::new(5_000_000),
            },
        )
        .unwrap();
    assert_eq!(sell.min_out, None);
    assert_eq!(sell.max_in, Some(sell.input));
    let sold = app
        .execute_contract(
            Addr::unchecked("alice"),
            market.clone(),
            &ExecuteMsg::Sell {
                outcome: Outcome::Yes,
                return_amount: sell.output,
                max_in: sell.max_in.unwrap(),
                deadline: sell.block_time,
            },
            &[],
        )
        .unwrap();
    let trade = event(&sold, "trade");
    for (key, expected) in [
        ("gross", sell.gross),
        ("net", sell.net),
        ("fee", sell.fee),
        ("input", sell.input),
        ("output", sell.output),
        ("reserve_yes_before", sell.reserve_yes_before),
        ("reserve_no_before", sell.reserve_no_before),
        ("reserve_yes_after", sell.reserve_yes_after),
        ("reserve_no_after", sell.reserve_no_after),
    ] {
        assert_eq!(attr(trade, key), expected.to_string());
    }
}

#[test]
fn solvency_tracks_forced_excess_without_assigning_a_claimant() {
    let (mut app, market) = setup();
    let baseline: SolvencyResponse = app
        .wrap()
        .query_wasm_smart(&market, &QueryMsg::Solvency {})
        .unwrap();
    assert_eq!(
        baseline.principal_or_terminal_liability,
        Uint128::new(INITIAL)
    );
    assert_eq!(baseline.accounted_liability, Uint128::new(INITIAL));
    assert_eq!(baseline.forced_excess, Uint128::zero());
    assert_eq!(baseline.shortfall, Uint128::zero());
    assert!(baseline.solvent);

    app.send_tokens(
        Addr::unchecked("alice"),
        market.clone(),
        &[coin(777, "ujuno")],
    )
    .unwrap();
    let forced: SolvencyResponse = app
        .wrap()
        .query_wasm_smart(&market, &QueryMsg::Solvency {})
        .unwrap();
    assert_eq!(
        forced.bank_balance,
        baseline.bank_balance + Uint128::new(777)
    );
    assert_eq!(forced.accounted_liability, baseline.accounted_liability);
    assert_eq!(forced.forced_excess, Uint128::new(777));
    assert_eq!(forced.shortfall, Uint128::zero());
    assert!(forced.solvent);
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct SeedMsg {
    bank_liability: Uint128,
}

fn seed_instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: SeedMsg,
) -> Result<Response, binary_market::error::ContractError> {
    let config = Config {
        protocol_version: ProtocolVersion::V1,
        factory: Addr::unchecked("factory"),
        creator: Addr::unchecked("creator"),
        initial_lp: Addr::unchecked("creator"),
        oracle: Addr::unchecked("oracle"),
        verdict_authority: Addr::unchecked("governance"),
        tier: TierId(1),
        collateral_denom: "ujuno".into(),
        close_ts: 100,
        opening_ts: 200,
        initial_liquidity: Uint128::new(3),
        oracle_bounty: Uint128::zero(),
        oracle_initial_bond: Uint128::one(),
        answer_timeout_secs: 1,
        arbitration_timeout_secs: 1,
        fee_bps: 200,
        min_trade: Uint128::one(),
        max_trade_bps: 2_500,
        max_position_per_side: Uint128::MAX,
        collateral_cap: Uint128::new(1_000),
        challenge_bond: Uint128::new(11),
        yes_answer: Binary::from(vec![1; 32]),
        no_answer: Binary::from(vec![0; 32]),
        invalid_answer: Binary::from(vec![255; 32]),
        unresolved_answer: Binary::from(vec![254; 32]),
        question: "seed".into(),
        question_hash: Binary::from(vec![9; 32]),
        nonce: 7,
    };
    state::CONFIG.save(deps.storage, &config)?;
    state::LIFECYCLE.save(
        deps.storage,
        &Lifecycle {
            activated: true,
            payout: Some(Payout::neutral()),
            resolution_answer: Some(Binary::from(vec![255; 32])),
            resolution_height: Some(8),
            resolution_time: Some(9),
            challenge_used: true,
        },
    )?;
    state::ACCOUNTING.save(
        deps.storage,
        &Accounting {
            principal: Uint128::new(20),
            fees: Uint128::new(5),
            challenge: Uint128::new(11),
            pool_yes: Uint128::new(2),
            pool_no: Uint128::new(3),
            total_yes: Uint128::new(20),
            total_no: Uint128::new(20),
            lp_supply: Uint128::new(3),
            lp_burned: Uint128::new(3),
            lp_paid: Uint128::new(4),
            neutral_half_dust: 1,
            lp_accrual: Uint128::new(7),
            principal_at_resolution: Some(Uint128::new(20)),
            fees_at_resolution: Some(Uint128::new(5)),
            terminal_liability_twice: Some(msg.bank_liability.checked_mul(Uint128::new(2))?),
            pool_yes_at_resolution: Some(Uint128::new(2)),
            pool_no_at_resolution: Some(Uint128::new(3)),
            total_yes_at_resolution: Some(Uint128::new(20)),
            total_no_at_resolution: Some(Uint128::new(20)),
        },
    )?;
    Ok(Response::new())
}

#[test]
fn terminal_snapshots_burn_accrual_neutral_dust_and_deficit_are_explicit() {
    let mut app = AppBuilder::new().build(|router, _, storage| {
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked("creator"),
                vec![coin(10, "ujuno")],
            )
            .unwrap();
    });
    let code = app.store_code(Box::new(ContractWrapper::new(
        execute,
        seed_instantiate,
        query,
    )));
    let market = app
        .instantiate_contract(
            code,
            Addr::unchecked("creator"),
            &SeedMsg {
                bank_liability: Uint128::new(13),
            },
            &[coin(10, "ujuno")],
            "deficit market",
            None,
        )
        .unwrap();
    let accounting: AccountingResponse = app
        .wrap()
        .query_wasm_smart(&market, &QueryMsg::Accounting {})
        .unwrap();
    assert_eq!(accounting.lp_burned, accounting.lp_supply);
    assert_eq!(accounting.lp_paid, Uint128::new(4));
    assert_eq!(accounting.lp_accrual, Uint128::new(7));
    assert_eq!(accounting.neutral_half_dust, 1);
    assert_eq!(accounting.principal_at_resolution, Some(Uint128::new(20)));
    assert_eq!(accounting.pool_yes_at_resolution, Some(Uint128::new(2)));
    assert_eq!(accounting.pool_no_at_resolution, Some(Uint128::new(3)));

    let resolution: ResolutionResponse = app
        .wrap()
        .query_wasm_smart(&market, &QueryMsg::Resolution {})
        .unwrap();
    assert_eq!(resolution.answer_hex, Some("ff".repeat(32)));
    assert_eq!(
        resolution.answer_base64,
        resolution.answer.as_ref().map(Binary::to_base64)
    );
    assert_eq!(resolution.terminal_liability_twice, Some(Uint128::new(26)));
    assert_eq!(resolution.pool_yes_at_resolution, Some(Uint128::new(2)));

    let solvency: SolvencyResponse = app
        .wrap()
        .query_wasm_smart(&market, &QueryMsg::Solvency {})
        .unwrap();
    assert_eq!(solvency.bank_balance, Uint128::new(10));
    assert_eq!(solvency.principal_or_terminal_liability, Uint128::new(13));
    assert_eq!(solvency.fee_liability, Uint128::new(5));
    assert_eq!(solvency.challenge_liability, Uint128::new(11));
    assert_eq!(solvency.lp_whole_coin_accrual, Uint128::new(7));
    assert_eq!(solvency.accounted_liability, Uint128::new(36));
    assert_eq!(solvency.forced_excess, Uint128::zero());
    assert_eq!(solvency.shortfall, Uint128::new(26));
    assert!(!solvency.solvent);
}
