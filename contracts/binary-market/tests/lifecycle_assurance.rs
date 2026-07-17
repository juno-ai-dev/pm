use binary_market::{
    contract::{execute, instantiate, query, reply},
    msg::{
        AccountingResponse, ExecuteMsg, InstantiateMsg, PositionResponse, QueryMsg,
        ResolutionResponse, SolvencyResponse, StateResponse,
    },
    question::{ObservationInput, QuestionInput, SourceInput, INVALID_HEX, NO_HEX, YES_HEX},
};
use cosmwasm_std::{coin, Addr, Binary, Coin, Empty, Timestamp, Uint128};
use cw_multi_test::{App, AppBuilder, AppResponse, Contract, ContractWrapper, Executor};
use cw_reality::msg::{ExecuteMsg as OracleExecuteMsg, InstantiateMsg as OracleInstantiateMsg};
use pm_types::{Outcome, Payout, TierId};
use serde_json::json;

const CREATION: u64 = 1_799_800_000;
const CLOSE: u64 = 1_800_000_000;
const OPENING: u64 = 1_800_086_400;
const BOND: u128 = 10_000_000;
const INITIAL: u128 = 100_000_000;
const BOUNTY: u128 = 1_000_000;

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

fn question(nonce: u64) -> QuestionInput {
    QuestionInput {
        title: format!("Lifecycle assurance market {nonce}?"),
        proposition: "Will the published lifecycle fixture be YES?".into(),
        definitions: vec!["The exact published byte controls.".into()],
        invalid_conditions: vec!["The source is unavailable.".into()],
        primary_sources: vec![SourceInput {
            publisher: "Fixture Authority".into(),
            identifier: format!("lifecycle/{nonce}/final"),
            url: "https://example.com/lifecycle".into(),
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

struct Harness {
    app: App,
    oracle: Addr,
    market_code: u64,
}

impl Harness {
    fn new() -> Self {
        let mut app = AppBuilder::new().build(|router, _, storage| {
            for (address, amount) in [
                ("factory", 500_000_000u128),
                ("answerer", 100_000_000),
                ("alice", 500_000_000),
                ("forcer", 100),
            ] {
                router
                    .bank
                    .init_balance(
                        storage,
                        &Addr::unchecked(address),
                        vec![coin(amount, "ujuno"), coin(1_000_000, "uatom")],
                    )
                    .unwrap();
            }
        });
        app.update_block(|block| block.time = Timestamp::from_seconds(CREATION));
        let oracle_code = app.store_code(oracle_contract());
        let oracle = app
            .instantiate_contract(
                oracle_code,
                Addr::unchecked("factory"),
                &OracleInstantiateMsg {
                    admin: None,
                    min_initial_bond_floor: Uint128::new(BOND),
                    min_answer_timeout_secs: 86_400,
                },
                &[],
                "shared frozen oracle",
                None,
            )
            .unwrap();
        let market_code = app.store_code(market_contract());
        Self {
            app,
            oracle,
            market_code,
        }
    }

    // This intentionally models the market's current factory boundary only. The
    // factory registry/CreateMarket journey remains owned by issue #17.
    fn activate_market(&mut self, nonce: u64) -> Addr {
        let market = self
            .app
            .instantiate_contract(
                self.market_code,
                Addr::unchecked("factory"),
                &InstantiateMsg {
                    factory: "factory".into(),
                    creator: "creator".into(),
                    oracle: self.oracle.to_string(),
                    verdict_authority: "dao-core".into(),
                    tier: TierId(1),
                    question: question(nonce),
                    nonce,
                    close_ts: CLOSE,
                    opening_ts: OPENING,
                    initial_liquidity: Uint128::new(INITIAL),
                    oracle_bounty: Uint128::new(BOUNTY),
                    oracle_initial_bond: Uint128::new(BOND),
                    answer_timeout_secs: 86_400,
                    arbitration_timeout_secs: 1_814_400,
                    fee_bps: 200,
                    min_trade: Uint128::new(10_000),
                    max_trade_bps: 2_500,
                    collateral_cap: Uint128::new(200_000_000),
                    challenge_bond: Uint128::new(BOND),
                },
                &[coin(INITIAL + BOUNTY, "ujuno")],
                format!("market-{nonce}"),
                None,
            )
            .unwrap();
        let state: StateResponse = self.query(&market, &QueryMsg::State {});
        assert!(state.activated);
        assert_eq!(
            self.app
                .wrap()
                .query_balance(&market, "ujuno")
                .unwrap()
                .amount,
            Uint128::new(INITIAL)
        );
        self.assert_reconciles(&market, 0);
        market
    }

    fn query<T: serde::de::DeserializeOwned>(&self, market: &Addr, msg: &QueryMsg) -> T {
        self.app.wrap().query_wasm_smart(market, msg).unwrap()
    }

    fn position(&self, market: &Addr) -> PositionResponse {
        self.query(
            market,
            &QueryMsg::Position {
                address: "alice".into(),
            },
        )
    }

    fn accounting(&self, market: &Addr) -> AccountingResponse {
        self.query(market, &QueryMsg::Accounting {})
    }

    fn assert_reconciles(&self, market: &Addr, forced: u128) {
        let accounting = self.accounting(market);
        let solvency: SolvencyResponse = self.query(market, &QueryMsg::Solvency {});
        let bank = self
            .app
            .wrap()
            .query_balance(market, "ujuno")
            .unwrap()
            .amount;
        assert_eq!(solvency.bank_balance, bank);
        let expected_principal = accounting
            .terminal_liability_twice
            .map_or(accounting.principal, |terminal| {
                (terminal + Uint128::from(accounting.neutral_half_dust)) / Uint128::new(2)
            });
        assert_eq!(solvency.principal_liability, expected_principal);
        assert_eq!(solvency.fee_liability, accounting.fees);
        assert_eq!(solvency.challenge_liability, accounting.challenge);
        assert_eq!(solvency.lp_accrual_liability, accounting.lp_accrual);
        assert_eq!(solvency.forced_excess, Uint128::new(forced));
        assert_eq!(bank, solvency.accounted_total + solvency.forced_excess);
    }

    fn trade(&mut self, market: &Addr) {
        let quote: binary_market::msg::QuoteResponse = self.query(
            market,
            &QueryMsg::QuoteBuy {
                outcome: Outcome::Yes,
                gross: Uint128::new(10_000_000),
            },
        );
        let bought = self
            .app
            .execute_contract(
                Addr::unchecked("alice"),
                market.clone(),
                &ExecuteMsg::Buy {
                    outcome: Outcome::Yes,
                    min_out: quote.output,
                    deadline: CREATION,
                },
                &[coin(10_000_000, "ujuno")],
            )
            .unwrap();
        assert_event_accounting(&bought, self.accounting(market));

        let sell: binary_market::msg::QuoteResponse = self.query(
            market,
            &QueryMsg::QuoteSell {
                outcome: Outcome::Yes,
                return_amount: Uint128::new(5_000_000),
            },
        );
        let sold = self
            .app
            .execute_contract(
                Addr::unchecked("alice"),
                market.clone(),
                &ExecuteMsg::Sell {
                    outcome: Outcome::Yes,
                    return_amount: Uint128::new(5_000_000),
                    max_in: sell.input,
                    deadline: CREATION,
                },
                &[],
            )
            .unwrap();
        assert_event_accounting(&sold, self.accounting(market));
        self.assert_reconciles(market, 0);
    }

    fn answer_resolve_redeem(
        &mut self,
        market: &Addr,
        answer: Binary,
        payout: Payout,
        forced: u128,
    ) {
        let bound: binary_market::msg::QuestionResponse =
            self.query(market, &QueryMsg::Question {});
        self.app
            .execute_contract(
                Addr::unchecked("answerer"),
                self.oracle.clone(),
                &OracleExecuteMsg::SubmitAnswer {
                    question_id: bound.question_id.unwrap(),
                    answer: answer.clone(),
                    current_bond_seen: Some(Uint128::zero()),
                },
                &[coin(BOND, "ujuno")],
            )
            .unwrap();
        self.app
            .update_block(|block| block.time = block.time.plus_seconds(86_400));
        self.app
            .execute_contract(
                Addr::unchecked("keeper"),
                market.clone(),
                &ExecuteMsg::Resolve {},
                &[],
            )
            .unwrap();
        let resolution: ResolutionResponse = self.query(market, &QueryMsg::Resolution {});
        assert_eq!(resolution.answer, Some(answer));
        assert_eq!(resolution.payout, Some(payout));

        let before = self.position(market);
        let response = self
            .app
            .execute_contract(
                Addr::unchecked("alice"),
                market.clone(),
                &ExecuteMsg::RedeemPositions {
                    yes: before.yes,
                    no: before.no,
                },
                &[],
            )
            .unwrap();
        let after = self.position(market);
        assert_eq!((after.yes, after.no), (Uint128::zero(), Uint128::zero()));
        let event = protocol_event(&response, "positions_redeemed");
        assert_eq!(attribute(event, "yes_burned"), before.yes.to_string());
        assert_eq!(attribute(event, "no_burned"), before.no.to_string());
        self.assert_reconciles(market, forced);
    }
}

fn bytes(value: &str) -> Binary {
    Binary::from(hex::decode(value).unwrap())
}

fn protocol_event<'a>(response: &'a AppResponse, action: &str) -> &'a cosmwasm_std::Event {
    response
        .events
        .iter()
        .find(|event| {
            event.ty == "wasm-juno_pm_v1"
                && event
                    .attributes
                    .iter()
                    .any(|a| a.key == "action" && a.value == action)
        })
        .unwrap_or_else(|| panic!("missing protocol event {action}"))
}

fn attribute(event: &cosmwasm_std::Event, key: &str) -> String {
    event
        .attributes
        .iter()
        .find(|a| a.key == key)
        .unwrap_or_else(|| panic!("missing {key}"))
        .value
        .clone()
}

fn assert_event_accounting(response: &AppResponse, accounting: AccountingResponse) {
    let event = protocol_event(response, "trade");
    assert_eq!(
        attribute(event, "principal_after"),
        accounting.principal.to_string()
    );
    assert_eq!(
        attribute(event, "fee_liability_after"),
        accounting.fees.to_string()
    );
}

#[test]
fn three_market_yes_no_neutral_lifecycles_reconcile_and_never_cross_contaminate() {
    let mut h = Harness::new();
    let yes = h.activate_market(19);
    let no = h.activate_market(20);
    let neutral = h.activate_market(21);
    for market in [&yes, &no, &neutral] {
        h.trade(market);
    }

    let no_before = (h.accounting(&no), h.position(&no));
    h.app
        .send_tokens(Addr::unchecked("forcer"), yes.clone(), &[coin(7, "ujuno")])
        .unwrap();
    h.assert_reconciles(&yes, 7);
    assert_eq!((h.accounting(&no), h.position(&no)), no_before);

    h.app
        .update_block(|block| block.time = Timestamp::from_seconds(OPENING));
    h.answer_resolve_redeem(&yes, bytes(YES_HEX), Payout::for_outcome(Outcome::Yes), 7);
    assert_eq!((h.accounting(&no), h.position(&no)), no_before);
    h.answer_resolve_redeem(&no, bytes(NO_HEX), Payout::for_outcome(Outcome::No), 0);
    h.answer_resolve_redeem(&neutral, bytes(INVALID_HEX), Payout::neutral(), 0);
}

#[test]
fn frozen_contracts_and_closed_wire_protocol_have_no_admin_migrate_pause_or_sweep() {
    let mut h = Harness::new();
    let market = h.activate_market(22);
    assert_eq!(
        h.app
            .wrap()
            .query_wasm_contract_info(&h.oracle)
            .unwrap()
            .admin,
        None
    );
    assert_eq!(
        h.app
            .wrap()
            .query_wasm_contract_info(&market)
            .unwrap()
            .admin,
        None
    );

    let before = h.app.wrap().query_all_balances(&market).unwrap();
    for unsupported in [
        json!({"pause": {}}),
        json!({"sweep": {"recipient": "creator"}}),
    ] {
        h.app
            .execute_contract(
                Addr::unchecked("creator"),
                market.clone(),
                &unsupported,
                &[],
            )
            .unwrap_err();
        assert_eq!(h.app.wrap().query_all_balances(&market).unwrap(), before);
    }
    h.app
        .migrate_contract(
            Addr::unchecked("factory"),
            market.clone(),
            &json!({}),
            h.market_code,
        )
        .unwrap_err();
    assert_eq!(h.app.wrap().query_all_balances(&market).unwrap(), before);

    // Even coins in an unrelated denomination cannot be extracted by a hidden
    // maintenance path; the only public actions are the typed ExecuteMsg variants.
    h.app
        .send_tokens(
            Addr::unchecked("forcer"),
            market.clone(),
            &[Coin::new(3, "uatom")],
        )
        .unwrap();
    let balances = h.app.wrap().query_all_balances(market).unwrap();
    assert!(balances.contains(&coin(3, "uatom")));
}
