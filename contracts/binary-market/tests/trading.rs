use binary_market::{
    contract::{execute, instantiate, query, reply},
    msg::{ExecuteMsg, InstantiateMsg, PositionResponse, QueryMsg, QuoteResponse},
    question::{ObservationInput, QuestionInput, SourceInput},
    state::Accounting,
};
use cosmwasm_std::{coin, from_json, Addr, Empty, Timestamp, Uint128};
use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
use cw_reality::msg::InstantiateMsg as OracleInstantiateMsg;
use pm_types::{Outcome, TierId};

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
        title: "Trading test?".into(),
        proposition: "Will the trading test pass?".into(),
        definitions: vec![],
        invalid_conditions: vec!["The test is withdrawn.".into()],
        primary_sources: vec![SourceInput {
            publisher: "Juno PM".into(),
            identifier: "tests/trading".into(),
            url: "https://example.com/trading".into(),
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

fn setup(cap: u128, max_trade_bps: u16) -> (App, Addr) {
    setup_with_position_cap(cap, max_trade_bps, Uint128::MAX)
}

fn setup_with_position_cap(
    cap: u128,
    max_trade_bps: u16,
    max_position_per_side: Uint128,
) -> (App, Addr) {
    let factory = Addr::unchecked("factory");
    let mut app = AppBuilder::new().build(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &factory, vec![coin(500_000_000, "ujuno")])
            .unwrap();
        for trader in ["alice", "bob"] {
            router
                .bank
                .init_balance(
                    storage,
                    &Addr::unchecked(trader),
                    vec![coin(500_000_000, "ujuno")],
                )
                .unwrap();
        }
    });
    app.update_block(|block| block.time = Timestamp::from_seconds(NOW));
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
                max_trade_bps,
                max_position_per_side,
                collateral_cap: Uint128::new(cap),
                challenge_bond: Uint128::new(10_000_000),
            },
            &[coin(INITIAL + 1_000_000, "ujuno")],
            "market",
            None,
        )
        .unwrap();
    (app, market)
}

fn accounting(app: &App, market: &Addr) -> Accounting {
    let raw = app
        .wrap()
        .query_wasm_raw(market, b"accounting")
        .unwrap()
        .expect("accounting exists");
    from_json(raw).unwrap()
}

fn position(app: &App, market: &Addr, owner: &str) -> PositionResponse {
    app.wrap()
        .query_wasm_smart(
            market,
            &QueryMsg::Position {
                address: owner.into(),
            },
        )
        .unwrap()
}

fn buy_quote(app: &App, market: &Addr, outcome: Outcome, gross: u128) -> QuoteResponse {
    app.wrap()
        .query_wasm_smart(
            market,
            &QueryMsg::QuoteBuy {
                outcome,
                gross: Uint128::new(gross),
            },
        )
        .unwrap()
}

fn sell_quote(app: &App, market: &Addr, outcome: Outcome, amount: u128) -> QuoteResponse {
    app.wrap()
        .query_wasm_smart(
            market,
            &QueryMsg::QuoteSell {
                outcome,
                return_amount: Uint128::new(amount),
            },
        )
        .unwrap()
}

fn buy(app: &mut App, market: &Addr, trader: &str, outcome: Outcome, gross: u128) -> QuoteResponse {
    let quote = buy_quote(app, market, outcome.clone(), gross);
    app.execute_contract(
        Addr::unchecked(trader),
        market.clone(),
        &ExecuteMsg::Buy {
            outcome,
            min_out: quote.output,
            deadline: app.block_info().time.seconds(),
        },
        &[coin(gross, "ujuno")],
    )
    .unwrap();
    quote
}

fn assert_reconciles(app: &App, market: &Addr, forced: u128) {
    let a = accounting(app, market);
    let alice = position(app, market, "alice");
    let bob = position(app, market, "bob");
    assert_eq!(a.total_yes, a.principal);
    assert_eq!(a.total_no, a.principal);
    assert_eq!(a.pool_yes + alice.yes + bob.yes, a.total_yes);
    assert_eq!(a.pool_no + alice.no + bob.no, a.total_no);
    assert_eq!(
        app.wrap()
            .query_balance(market.to_string(), "ujuno")
            .unwrap()
            .amount,
        a.principal + a.fees + Uint128::new(forced)
    );
}

#[test]
fn buy_cannot_cross_per_address_outcome_exposure_cap() {
    let (mut app, market) = setup_with_position_cap(200_000_000, 2_500, Uint128::new(1));
    let accounting_before = accounting(&app, &market);
    let position_before = position(&app, &market, "alice");
    assert!(app
        .execute_contract(
            Addr::unchecked("alice"),
            market.clone(),
            &ExecuteMsg::Buy {
                outcome: Outcome::Yes,
                min_out: Uint128::zero(),
                deadline: NOW + 1_000,
            },
            &[coin(1_000_000, "ujuno")],
        )
        .is_err());
    assert_eq!(accounting(&app, &market), accounting_before);
    assert_eq!(position(&app, &market, "alice"), position_before);
}

#[test]
fn documented_buy_sell_vector_matches_quote_and_ledgers() {
    let (mut app, market) = setup(200_000_000, 2_500);
    let buy = buy(&mut app, &market, "alice", Outcome::Yes, 10_000_000);
    assert_eq!(buy.fee, Uint128::new(200_000));
    assert_eq!(buy.net, Uint128::new(9_800_000));
    assert_eq!(buy.output, Uint128::new(18_725_318));
    assert_eq!(buy.reserve_yes_after, Uint128::new(91_074_682));
    assert_eq!(buy.reserve_no_after, Uint128::new(109_800_000));

    let sell = sell_quote(&app, &market, Outcome::Yes, 5_000_000);
    assert_eq!(sell.gross, Uint128::new(5_102_041));
    assert_eq!(sell.fee, Uint128::new(102_041));
    assert_eq!(sell.input, Uint128::new(9_540_206));
    let response = app
        .execute_contract(
            Addr::unchecked("alice"),
            market.clone(),
            &ExecuteMsg::Sell {
                outcome: Outcome::Yes,
                return_amount: Uint128::new(5_000_000),
                max_in: sell.input,
                deadline: NOW,
            },
            &[],
        )
        .unwrap();
    let trade = response
        .events
        .iter()
        .find(|event| event.ty.ends_with("juno_pm_v1"))
        .expect("trade event");
    let attribute = |key: &str| {
        trade
            .attributes
            .iter()
            .find(|attribute| attribute.key == key)
            .map(|attribute| attribute.value.as_str())
    };
    assert_eq!(attribute("action"), Some("trade"));
    assert_eq!(attribute("account"), Some("alice"));
    assert_eq!(attribute("side"), Some("sell"));
    assert_eq!(attribute("outcome"), Some("yes"));
    assert_eq!(attribute("principal_after"), Some("104697959"));
    assert_eq!(attribute("fee_liability_after"), Some("302041"));
    assert!(attribute("caller").is_none());
    assert!(attribute("principal").is_none());
    assert!(attribute("fees").is_none());
    let a = accounting(&app, &market);
    assert_eq!(a.pool_yes, Uint128::new(95_512_847));
    assert_eq!(a.pool_no, Uint128::new(104_697_959));
    assert_eq!(a.principal, Uint128::new(104_697_959));
    assert_eq!(a.fees, Uint128::new(302_041));
    assert_eq!(
        position(&app, &market, "alice").yes,
        Uint128::new(9_185_112)
    );
    assert_reconciles(&app, &market, 0);
}

#[test]
fn both_outcomes_trade_symmetrically() {
    for outcome in [Outcome::Yes, Outcome::No] {
        let (mut app, market) = setup(200_000_000, 2_500);
        let bought = buy(&mut app, &market, "alice", outcome.clone(), 1_000_000);
        let sold = sell_quote(&app, &market, outcome.clone(), 200_000);
        app.execute_contract(
            Addr::unchecked("alice"),
            market.clone(),
            &ExecuteMsg::Sell {
                outcome: outcome.clone(),
                return_amount: sold.net,
                max_in: sold.input,
                deadline: NOW,
            },
            &[],
        )
        .unwrap();
        let p = position(&app, &market, "alice");
        let remaining = match outcome {
            Outcome::Yes => p.yes,
            Outcome::No => p.no,
        };
        assert_eq!(remaining, bought.output - sold.input);
        assert_reconciles(&app, &market, 0);
    }
}

#[test]
fn close_and_deadline_boundaries_are_exact() {
    for (time, deadline, succeeds) in [
        (CLOSE - 1, CLOSE - 1, true),
        (CLOSE - 1, CLOSE - 2, false),
        (CLOSE, CLOSE + 1, false),
        (CLOSE + 1, CLOSE + 1, false),
    ] {
        let (mut app, market) = setup(200_000_000, 2_500);
        app.update_block(|block| block.time = Timestamp::from_seconds(time));
        let result = app.execute_contract(
            Addr::unchecked("alice"),
            market.clone(),
            &ExecuteMsg::Buy {
                outcome: Outcome::Yes,
                min_out: Uint128::zero(),
                deadline,
            },
            &[coin(10_000, "ujuno")],
        );
        assert_eq!(result.is_ok(), succeeds);
    }
}

#[test]
fn stale_quote_slippage_and_failure_guards_are_atomic() {
    let (mut app, market) = setup(110_000_000, 2_500);
    let stale = buy_quote(&app, &market, Outcome::Yes, 1_000_000);
    buy(&mut app, &market, "bob", Outcome::Yes, 1_000_000);
    let before = accounting(&app, &market);
    assert!(app
        .execute_contract(
            Addr::unchecked("alice"),
            market.clone(),
            &ExecuteMsg::Buy {
                outcome: Outcome::Yes,
                min_out: stale.output,
                deadline: NOW,
            },
            &[coin(1_000_000, "ujuno")],
        )
        .is_err());
    assert_eq!(accounting(&app, &market), before);

    for funds in [
        vec![],
        vec![coin(10_000, "uatom")],
        vec![coin(10_000, "ujuno"), coin(1, "uatom")],
    ] {
        assert!(app
            .execute_contract(
                Addr::unchecked("alice"),
                market.clone(),
                &ExecuteMsg::Buy {
                    outcome: Outcome::No,
                    min_out: Uint128::zero(),
                    deadline: NOW,
                },
                &funds,
            )
            .is_err());
        assert_eq!(accounting(&app, &market), before);
    }

    assert!(app
        .execute_contract(
            Addr::unchecked("alice"),
            market.clone(),
            &ExecuteMsg::Sell {
                outcome: Outcome::No,
                return_amount: Uint128::new(10_000),
                max_in: Uint128::MAX,
                deadline: NOW,
            },
            &[],
        )
        .is_err());
    assert_eq!(accounting(&app, &market), before);

    assert!(app
        .execute_contract(
            Addr::unchecked("alice"),
            market.clone(),
            &ExecuteMsg::Sell {
                outcome: Outcome::No,
                return_amount: Uint128::new(10_000),
                max_in: Uint128::MAX,
                deadline: NOW,
            },
            &[coin(10_000, "ujuno")],
        )
        .is_err());
    assert_eq!(accounting(&app, &market), before);

    // Remaining cap is below this buy's net split.
    assert!(app
        .execute_contract(
            Addr::unchecked("alice"),
            market.clone(),
            &ExecuteMsg::Buy {
                outcome: Outcome::No,
                min_out: Uint128::zero(),
                deadline: NOW,
            },
            &[coin(10_000_000, "ujuno")],
        )
        .is_err());
    assert_eq!(accounting(&app, &market), before);
}

#[test]
fn configured_minimum_is_shared_by_quotes_and_execution() {
    let (mut app, market) = setup(200_000_000, 2_500);
    let below = Uint128::new(9_999);
    assert!(app
        .wrap()
        .query_wasm_smart::<QuoteResponse>(
            &market,
            &QueryMsg::QuoteBuy {
                outcome: Outcome::Yes,
                gross: below,
            },
        )
        .is_err());
    assert!(app
        .wrap()
        .query_wasm_smart::<QuoteResponse>(
            &market,
            &QueryMsg::QuoteSell {
                outcome: Outcome::Yes,
                return_amount: below,
            },
        )
        .is_err());

    let before = accounting(&app, &market);
    assert!(app
        .execute_contract(
            Addr::unchecked("alice"),
            market.clone(),
            &ExecuteMsg::Buy {
                outcome: Outcome::Yes,
                min_out: Uint128::zero(),
                deadline: NOW,
            },
            &[coin(below.u128(), "ujuno")],
        )
        .is_err());
    assert!(app
        .execute_contract(
            Addr::unchecked("alice"),
            market.clone(),
            &ExecuteMsg::Sell {
                outcome: Outcome::Yes,
                return_amount: below,
                max_in: Uint128::MAX,
                deadline: NOW,
            },
            &[],
        )
        .is_err());
    assert_eq!(accounting(&app, &market), before);
}

#[test]
fn stale_sell_quote_enforces_max_input_atomically() {
    let (mut app, market) = setup(250_000_000, 2_500);
    buy(&mut app, &market, "alice", Outcome::Yes, 10_000_000);
    let stale = sell_quote(&app, &market, Outcome::Yes, 1_000_000);
    // Buying NO moves the YES sell price against Alice.
    buy(&mut app, &market, "bob", Outcome::No, 10_000_000);
    let before = accounting(&app, &market);
    let position_before = position(&app, &market, "alice");
    assert!(app
        .execute_contract(
            Addr::unchecked("alice"),
            market.clone(),
            &ExecuteMsg::Sell {
                outcome: Outcome::Yes,
                return_amount: stale.net,
                max_in: stale.input,
                deadline: NOW,
            },
            &[],
        )
        .is_err());
    assert_eq!(accounting(&app, &market), before);
    assert_eq!(position(&app, &market, "alice"), position_before);
}

#[test]
fn configured_reserve_ratio_is_enforced_by_query_and_execute() {
    let (mut app, market) = setup(200_000_000, 100);
    assert!(app
        .wrap()
        .query_wasm_smart::<QuoteResponse>(
            &market,
            &QueryMsg::QuoteBuy {
                outcome: Outcome::Yes,
                gross: Uint128::new(1_100_000),
            },
        )
        .is_err());
    let before = accounting(&app, &market);
    assert!(app
        .execute_contract(
            Addr::unchecked("alice"),
            market.clone(),
            &ExecuteMsg::Buy {
                outcome: Outcome::Yes,
                min_out: Uint128::zero(),
                deadline: NOW,
            },
            &[coin(1_100_000, "ujuno")],
        )
        .is_err());
    assert_eq!(accounting(&app, &market), before);
}

#[test]
fn seeded_random_trade_sequences_reconcile_with_forced_funds() {
    for seed in [1u64, 0x5eed, 0xdead_beef] {
        let (mut app, market) = setup(400_000_000, 2_500);
        let mut random = seed;
        let mut forced = 0u128;
        for step in 0..48u64 {
            random = random
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            let trader = if random & 1 == 0 { "alice" } else { "bob" };
            let outcome = if random & 2 == 0 {
                Outcome::Yes
            } else {
                Outcome::No
            };
            let current = position(&app, &market, trader);
            let held = match outcome {
                Outcome::Yes => current.yes,
                Outcome::No => current.no,
            };
            let return_amount = (((random >> 8) % 10) as u128 + 1) * 10_000;
            let sell = sell_quote(&app, &market, outcome.clone(), return_amount);
            if held >= sell.input && random & 4 != 0 {
                app.execute_contract(
                    Addr::unchecked(trader),
                    market.clone(),
                    &ExecuteMsg::Sell {
                        outcome,
                        return_amount: sell.net,
                        max_in: sell.input,
                        deadline: NOW,
                    },
                    &[],
                )
                .unwrap();
            } else {
                let gross = (((random >> 16) % 20) as u128 + 1) * 10_000;
                buy(&mut app, &market, trader, outcome, gross);
            }
            if step % 9 == 0 {
                let amount = ((random >> 24) % 17) as u128 + 1;
                app.send_tokens(
                    Addr::unchecked("factory"),
                    market.clone(),
                    &[coin(amount, "ujuno")],
                )
                .unwrap();
                forced += amount;
            }
            assert_reconciles(&app, &market, forced);
        }
    }
}

#[test]
fn failed_sell_bank_send_rolls_back_all_ledgers() {
    let (mut app, market) = setup(200_000_000, 2_500);
    buy(&mut app, &market, "alice", Outcome::Yes, 1_000_000);
    let quote = sell_quote(&app, &market, Outcome::Yes, 100_000);
    let before_accounting = accounting(&app, &market);
    let before_position = position(&app, &market, "alice");
    app.init_modules(|router, _, storage| {
        router.bank.init_balance(storage, &market, vec![]).unwrap();
    });
    assert!(app
        .execute_contract(
            Addr::unchecked("alice"),
            market.clone(),
            &ExecuteMsg::Sell {
                outcome: Outcome::Yes,
                return_amount: quote.net,
                max_in: quote.input,
                deadline: NOW,
            },
            &[],
        )
        .is_err());
    assert_eq!(accounting(&app, &market), before_accounting);
    assert_eq!(position(&app, &market, "alice"), before_position);
}
