//! Independent arbitrary-precision state-machine checks for the implemented
//! pre-resolution market surface. Proptest prints its reproducible seed; every
//! assertion also includes the complete action trace up to the failing step.

use binary_market::{
    contract::{execute, instantiate, query, reply},
    msg::{AccountingResponse, ExecuteMsg, InstantiateMsg, PositionResponse, QueryMsg},
    question::{ObservationInput, QuestionInput, SourceInput, INVALID_HEX, NO_HEX, YES_HEX},
    state::Accounting,
};
use cosmwasm_std::{coin, from_json, Addr, Binary, Empty, Timestamp, Uint128};
use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
use cw_reality::msg::{ExecuteMsg as OracleExecuteMsg, InstantiateMsg as OracleInstantiateMsg};
use num_bigint::BigUint;
use num_traits::{ToPrimitive, Zero};
use pm_types::{Outcome, TierId};
use proptest::prelude::*;

const NOW: u64 = 1_799_800_000;
const CLOSE: u64 = 1_800_000_000;
const INITIAL: u128 = 100_000_000;
const CAP: u128 = 200_000_000;
const FEE_BPS: u128 = 200;
const MIN_TRADE: u128 = 10_000;
const USERS: [&str; 3] = ["alice", "bob", "carol"];

#[derive(Clone, Debug)]
enum Action {
    Split { user: usize, amount: u128 },
    Merge { user: usize, amount: u128 },
    Buy { user: usize, yes: bool, gross: u128 },
    Sell { user: usize, yes: bool, net: u128 },
    Force { amount: u128 },
    Close,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ModelPosition {
    yes: BigUint,
    no: BigUint,
}

#[derive(Clone, Debug)]
struct Model {
    principal: BigUint,
    fees: BigUint,
    pool_yes: BigUint,
    pool_no: BigUint,
    total_yes: BigUint,
    total_no: BigUint,
    positions: [ModelPosition; 3],
    forced: BigUint,
    closed: bool,
}

#[derive(Clone, Debug)]
struct BuyEffect {
    fee: BigUint,
    net: BigUint,
    output: BigUint,
    selected_after: BigUint,
    opposite_after: BigUint,
}

#[derive(Clone, Debug)]
struct SellEffect {
    fee: BigUint,
    merge: BigUint,
    input: BigUint,
    selected_after: BigUint,
    opposite_after: BigUint,
}

fn big(value: u128) -> BigUint {
    BigUint::from(value)
}

fn ceil_div(numerator: BigUint, denominator: &BigUint) -> BigUint {
    let quotient = &numerator / denominator;
    if numerator % denominator == BigUint::zero() {
        quotient
    } else {
        quotient + BigUint::from(1u8)
    }
}

impl Model {
    fn new() -> Self {
        let initial = big(INITIAL);
        Self {
            principal: initial.clone(),
            fees: BigUint::zero(),
            pool_yes: initial.clone(),
            pool_no: initial.clone(),
            total_yes: initial.clone(),
            total_no: initial,
            positions: Default::default(),
            forced: BigUint::zero(),
            closed: false,
        }
    }

    fn buy_effect(&self, yes: bool, gross: u128) -> Option<BuyEffect> {
        if self.closed || gross < MIN_TRADE {
            return None;
        }
        let gross = big(gross);
        let fee = ceil_div(&gross * big(FEE_BPS), &big(10_000));
        let net = &gross - &fee;
        let smaller = self.pool_yes.clone().min(self.pool_no.clone());
        if net.is_zero() || net > smaller / big(4) || &self.principal + &net > big(CAP) {
            return None;
        }
        let (selected, opposite) = if yes {
            (&self.pool_yes, &self.pool_no)
        } else {
            (&self.pool_no, &self.pool_yes)
        };
        let opposite_after = opposite + &net;
        let selected_after = ceil_div(selected * opposite, &opposite_after);
        if selected_after.is_zero() || selected + &net <= selected_after {
            return None;
        }
        let output = selected + &net - &selected_after;
        Some(BuyEffect {
            fee,
            net,
            output,
            selected_after,
            opposite_after,
        })
    }

    fn sell_effect(&self, user: usize, yes: bool, net: u128) -> Option<SellEffect> {
        if self.closed || net < MIN_TRADE {
            return None;
        }
        let net = big(net);
        let merge = ceil_div(&net * big(10_000), &big(10_000 - FEE_BPS));
        let smaller = self.pool_yes.clone().min(self.pool_no.clone());
        let (selected, opposite, owned) = if yes {
            (&self.pool_yes, &self.pool_no, &self.positions[user].yes)
        } else {
            (&self.pool_no, &self.pool_yes, &self.positions[user].no)
        };
        if merge > smaller / big(4) || merge >= *opposite {
            return None;
        }
        let opposite_after = opposite - &merge;
        let selected_before_merge = ceil_div(selected * opposite, &opposite_after);
        let input = &merge + selected_before_merge - selected;
        if input.is_zero() || input > *owned {
            return None;
        }
        let selected_after = selected + &input - &merge;
        Some(SellEffect {
            fee: &merge - &net,
            merge,
            input,
            selected_after,
            opposite_after,
        })
    }

    fn apply(&mut self, action: &Action) -> bool {
        match *action {
            Action::Split { user, amount } => {
                let amount = big(amount);
                if self.closed
                    || amount < big(MIN_TRADE)
                    || &amount * big(4) > self.pool_yes.clone().min(self.pool_no.clone())
                    || &self.principal + &amount > big(CAP)
                {
                    return false;
                }
                self.principal += &amount;
                self.total_yes += &amount;
                self.total_no += &amount;
                self.positions[user].yes += &amount;
                self.positions[user].no += amount;
                true
            }
            Action::Merge { user, amount } => {
                let amount = big(amount);
                if amount.is_zero()
                    || amount < big(MIN_TRADE)
                    || &amount * big(4) > self.pool_yes.clone().min(self.pool_no.clone())
                    || self.positions[user].yes < amount
                    || self.positions[user].no < amount
                {
                    return false;
                }
                self.principal -= &amount;
                self.total_yes -= &amount;
                self.total_no -= &amount;
                self.positions[user].yes -= &amount;
                self.positions[user].no -= amount;
                true
            }
            Action::Buy { user, yes, gross } => {
                let Some(effect) = self.buy_effect(yes, gross) else {
                    return false;
                };
                self.principal += &effect.net;
                self.fees += &effect.fee;
                self.total_yes += &effect.net;
                self.total_no += &effect.net;
                if yes {
                    self.pool_yes = effect.selected_after;
                    self.pool_no = effect.opposite_after;
                    self.positions[user].yes += effect.output;
                } else {
                    self.pool_no = effect.selected_after;
                    self.pool_yes = effect.opposite_after;
                    self.positions[user].no += effect.output;
                }
                true
            }
            Action::Sell { user, yes, net } => {
                let Some(effect) = self.sell_effect(user, yes, net) else {
                    return false;
                };
                self.principal -= &effect.merge;
                self.fees += &effect.fee;
                self.total_yes -= &effect.merge;
                self.total_no -= &effect.merge;
                if yes {
                    self.pool_yes = effect.selected_after;
                    self.pool_no = effect.opposite_after;
                    self.positions[user].yes -= effect.input;
                } else {
                    self.pool_no = effect.selected_after;
                    self.pool_yes = effect.opposite_after;
                    self.positions[user].no -= effect.input;
                }
                true
            }
            Action::Force { amount } => {
                if amount == 0 {
                    return false;
                }
                self.forced += big(amount);
                true
            }
            Action::Close => {
                self.closed = true;
                true
            }
        }
    }

    fn assert_invariants(&self, trace: &[Action]) {
        assert_eq!(
            self.total_yes, self.principal,
            "complete-set YES; trace={trace:#?}"
        );
        assert_eq!(
            self.total_no, self.principal,
            "complete-set NO; trace={trace:#?}"
        );
        let users_yes: BigUint = self.positions.iter().map(|p| &p.yes).sum();
        let users_no: BigUint = self.positions.iter().map(|p| &p.no).sum();
        assert_eq!(
            &self.pool_yes + users_yes,
            self.total_yes,
            "YES coverage; trace={trace:#?}"
        );
        assert_eq!(
            &self.pool_no + users_no,
            self.total_no,
            "NO coverage; trace={trace:#?}"
        );
        assert!(
            !self.pool_yes.is_zero() && !self.pool_no.is_zero(),
            "positive pool; trace={trace:#?}"
        );
        assert!(self.principal <= big(CAP), "cap; trace={trace:#?}");
    }
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
        title: "State model?".into(),
        proposition: "Will the state model remain consistent?".into(),
        definitions: vec![],
        invalid_conditions: vec!["Test withdrawn".into()],
        primary_sources: vec![SourceInput {
            publisher: "Juno PM".into(),
            identifier: "tests/state-model".into(),
            url: "https://example.com/state-model".into(),
            retrieval: "HTTPS JSON".into(),
            publication_revision_policy: "Latest before opening controls.".into(),
            fallback_condition: "Unavailable for 72 hours.".into(),
        }],
        secondary_sources: vec![],
        source_disagreement_policy: "Primary controls.".into(),
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
        for account in ["factory", "alice", "bob", "carol"] {
            router
                .bank
                .init_balance(
                    storage,
                    &Addr::unchecked(account),
                    vec![coin(20_000_000_000, "ujuno")],
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
    let code = app.store_code(market_contract());
    let market = app
        .instantiate_contract(
            code,
            factory.clone(),
            &InstantiateMsg {
                factory: factory.to_string(),
                creator: "creator".into(),
                oracle: oracle.to_string(),
                verdict_authority: "governance".into(),
                tier: TierId(1),
                question: question(),
                nonce: 118,
                close_ts: CLOSE,
                opening_ts: CLOSE + 86_400,
                initial_liquidity: Uint128::new(INITIAL),
                oracle_bounty: Uint128::new(1_000_000),
                oracle_initial_bond: Uint128::new(10_000_000),
                answer_timeout_secs: 86_400,
                arbitration_timeout_secs: 1_814_400,
                fee_bps: FEE_BPS as u16,
                min_trade: Uint128::new(MIN_TRADE),
                max_trade_bps: 2_500,
                max_position_per_side: Uint128::MAX,
                collateral_cap: Uint128::new(CAP),
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
    from_json(
        app.wrap()
            .query_wasm_raw(market, b"accounting")
            .unwrap()
            .unwrap(),
    )
    .unwrap()
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

fn execute_action(app: &mut App, market: &Addr, action: &Action) -> bool {
    match *action {
        Action::Split { user, amount } => app
            .execute_contract(
                Addr::unchecked(USERS[user]),
                market.clone(),
                &ExecuteMsg::Split {
                    amount: Uint128::new(amount),
                },
                &[coin(amount, "ujuno")],
            )
            .is_ok(),
        Action::Merge { user, amount } => app
            .execute_contract(
                Addr::unchecked(USERS[user]),
                market.clone(),
                &ExecuteMsg::Merge {
                    amount: Uint128::new(amount),
                },
                &[],
            )
            .is_ok(),
        Action::Buy { user, yes, gross } => app
            .execute_contract(
                Addr::unchecked(USERS[user]),
                market.clone(),
                &ExecuteMsg::Buy {
                    outcome: if yes { Outcome::Yes } else { Outcome::No },
                    min_out: Uint128::zero(),
                    deadline: app.block_info().time.seconds(),
                },
                &[coin(gross, "ujuno")],
            )
            .is_ok(),
        Action::Sell { user, yes, net } => app
            .execute_contract(
                Addr::unchecked(USERS[user]),
                market.clone(),
                &ExecuteMsg::Sell {
                    outcome: if yes { Outcome::Yes } else { Outcome::No },
                    return_amount: Uint128::new(net),
                    max_in: Uint128::MAX,
                    deadline: app.block_info().time.seconds(),
                },
                &[],
            )
            .is_ok(),
        Action::Force { amount } => app
            .send_tokens(
                Addr::unchecked("factory"),
                market.clone(),
                &[coin(amount, "ujuno")],
            )
            .is_ok(),
        Action::Close => {
            app.update_block(|block| block.time = Timestamp::from_seconds(CLOSE));
            true
        }
    }
}

fn assert_contract_matches(app: &App, market: &Addr, model: &Model, trace: &[Action]) {
    let actual = accounting(app, market);
    let eq = |name: &str, actual: u128, expected: &BigUint| {
        assert_eq!(
            actual,
            expected.to_u128().unwrap(),
            "{name}; trace={trace:#?}"
        );
    };
    eq("principal", actual.principal.u128(), &model.principal);
    eq("fees", actual.fees.u128(), &model.fees);
    eq("pool_yes", actual.pool_yes.u128(), &model.pool_yes);
    eq("pool_no", actual.pool_no.u128(), &model.pool_no);
    eq("total_yes", actual.total_yes.u128(), &model.total_yes);
    eq("total_no", actual.total_no.u128(), &model.total_no);
    for (index, user) in USERS.iter().enumerate() {
        let actual = position(app, market, user);
        eq(
            "position_yes",
            actual.yes.u128(),
            &model.positions[index].yes,
        );
        eq("position_no", actual.no.u128(), &model.positions[index].no);
    }
    let bank = app
        .wrap()
        .query_balance(market, "ujuno")
        .unwrap()
        .amount
        .u128();
    eq(
        "bank coverage/no-sweep",
        bank,
        &(&model.principal + &model.fees + &model.forced),
    );
}

fn amount_strategy() -> impl Strategy<Value = u128> {
    prop_oneof![
        Just(0),
        Just(1),
        Just(MIN_TRADE - 1),
        Just(MIN_TRADE),
        Just(MIN_TRADE + 1),
        Just(1_000_000),
        Just(24_999_999),
        Just(25_000_000),
        Just(CAP),
        1u128..=30_000_000u128,
    ]
}
fn action_strategy() -> impl Strategy<Value = Action> {
    prop_oneof![
        (0usize..USERS.len(), amount_strategy())
            .prop_map(|(user, amount)| Action::Split { user, amount }),
        (0usize..USERS.len(), amount_strategy())
            .prop_map(|(user, amount)| Action::Merge { user, amount }),
        (0usize..USERS.len(), any::<bool>(), amount_strategy())
            .prop_map(|(user, yes, gross)| Action::Buy { user, yes, gross }),
        (0usize..USERS.len(), any::<bool>(), amount_strategy())
            .prop_map(|(user, yes, net)| Action::Sell { user, yes, net }),
        amount_strategy().prop_map(|amount| Action::Force { amount }),
        Just(Action::Close),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 32, max_shrink_iters: 2_048, ..ProptestConfig::default() })]
    #[test]
    fn arbitrary_precision_model_matches_every_success_and_rejection(
        actions in prop::collection::vec(action_strategy(), 1..48)
    ) {
        let (mut app, market) = setup();
        let mut model = Model::new();
        let mut trace = Vec::new();
        assert_contract_matches(&app, &market, &model, &trace);
        for action in actions {
            trace.push(action.clone());
            let contract_before = (accounting(&app, &market), USERS.map(|u| position(&app, &market, u)));
            let model_before = model.clone();
            let expected = model.apply(&action);
            let actual = execute_action(&mut app, &market, &action);
            prop_assert_eq!(actual, expected, "accept/reject mismatch; trace={:#?}", trace);
            if !actual {
                prop_assert_eq!(accounting(&app, &market), contract_before.0, "rejection changed accounting; trace={:#?}", trace);
                for (index, user) in USERS.iter().enumerate() {
                    prop_assert_eq!(position(&app, &market, user), contract_before.1[index].clone(), "rejection changed position; trace={:#?}", trace);
                }
                prop_assert_eq!(&model.principal, &model_before.principal, "rejected model transition; trace={:#?}", trace);
            }
            model.assert_invariants(&trace);
            assert_contract_matches(&app, &market, &model, &trace);
        }
    }
}

#[test]
fn aggregate_and_partitioned_complete_sets_are_path_independent() {
    let (mut aggregate, aggregate_market) = setup();
    aggregate
        .execute_contract(
            Addr::unchecked("alice"),
            aggregate_market.clone(),
            &ExecuteMsg::Split {
                amount: Uint128::new(90_000),
            },
            &[coin(90_000, "ujuno")],
        )
        .unwrap();
    aggregate
        .execute_contract(
            Addr::unchecked("alice"),
            aggregate_market.clone(),
            &ExecuteMsg::Merge {
                amount: Uint128::new(90_000),
            },
            &[],
        )
        .unwrap();

    let (mut partitioned, partitioned_market) = setup();
    for amount in [10_000u128, 20_000, 60_000] {
        partitioned
            .execute_contract(
                Addr::unchecked("alice"),
                partitioned_market.clone(),
                &ExecuteMsg::Split {
                    amount: Uint128::new(amount),
                },
                &[coin(amount, "ujuno")],
            )
            .unwrap();
    }
    for amount in [40_000u128, 20_000, 30_000] {
        partitioned
            .execute_contract(
                Addr::unchecked("alice"),
                partitioned_market.clone(),
                &ExecuteMsg::Merge {
                    amount: Uint128::new(amount),
                },
                &[],
            )
            .unwrap();
    }
    assert_eq!(
        accounting(&aggregate, &aggregate_market),
        accounting(&partitioned, &partitioned_market)
    );
    assert_eq!(
        position(&aggregate, &aggregate_market, "alice"),
        position(&partitioned, &partitioned_market, "alice")
    );
}

#[derive(Clone, Copy, Debug)]
enum TerminalPayout {
    Yes,
    No,
    Neutral,
}

impl TerminalPayout {
    fn answer(self) -> Binary {
        Binary::from(
            hex::decode(match self {
                Self::Yes => YES_HEX,
                Self::No => NO_HEX,
                Self::Neutral => INVALID_HEX,
            })
            .unwrap(),
        )
    }
    fn nums(self) -> (BigUint, BigUint) {
        match self {
            Self::Yes => (big(2), BigUint::zero()),
            Self::No => (BigUint::zero(), big(2)),
            Self::Neutral => (big(1), big(1)),
        }
    }
}

fn public_accounting(app: &App, market: &Addr) -> AccountingResponse {
    app.wrap()
        .query_wasm_smart(market, &QueryMsg::Accounting {})
        .unwrap()
}
fn market_bank(app: &App, market: &Addr) -> u128 {
    app.wrap()
        .query_balance(market, "ujuno")
        .unwrap()
        .amount
        .u128()
}

/// Resolve through no challenge, a refunded challenge, a slashed matching
/// verdict, or a slashed timeout. Returns independently expected LP accrual.
fn finalize_for_model(
    app: &mut App,
    market: &Addr,
    payout: TerminalPayout,
    challenge_path: u8,
    trace: &[Action],
) -> BigUint {
    let question: binary_market::msg::QuestionResponse = app
        .wrap()
        .query_wasm_smart(market, &QueryMsg::Question {})
        .unwrap();
    let qid = question.question_id.unwrap();
    let config: binary_market::msg::ConfigResponse = app
        .wrap()
        .query_wasm_smart(market, &QueryMsg::Config {})
        .unwrap();
    let final_answer = payout.answer();
    let initial_answer = if challenge_path == 2 {
        match payout {
            TerminalPayout::Yes => TerminalPayout::No.answer(),
            _ => TerminalPayout::Yes.answer(),
        }
    } else {
        final_answer.clone()
    };

    app.update_block(|b| b.time = Timestamp::from_seconds(CLOSE + 86_400));
    app.execute_contract(
        Addr::unchecked("factory"),
        Addr::unchecked(config.oracle),
        &OracleExecuteMsg::SubmitAnswer {
            question_id: qid.clone(),
            answer: initial_answer,
            current_bond_seen: Some(Uint128::zero()),
        },
        &[coin(10_000_000, "ujuno")],
    )
    .unwrap();
    if challenge_path == 0 {
        app.update_block(|b| b.time = b.time.plus_seconds(86_400));
    } else {
        let before = public_accounting(app, market);
        app.execute_contract(
            Addr::unchecked("carol"),
            market.clone(),
            &ExecuteMsg::Challenge {},
            &[coin(10_000_000, "ujuno")],
        )
        .unwrap();
        let pending = public_accounting(app, market);
        assert_eq!(
            pending.challenge,
            Uint128::new(10_000_000),
            "trace={trace:#?}"
        );
        assert_eq!(
            (pending.principal, pending.fees),
            (before.principal, before.fees)
        );
        if challenge_path == 3 {
            app.update_block(|b| b.time = b.time.plus_seconds(1_814_400));
            app.execute_contract(
                Addr::unchecked("factory"),
                market.clone(),
                &ExecuteMsg::FinalizeStalledChallenge {},
                &[],
            )
            .unwrap();
            app.update_block(|b| b.time = b.time.plus_seconds(86_400));
        } else {
            app.execute_contract(
                Addr::unchecked("governance"),
                market.clone(),
                &ExecuteMsg::GovernanceVerdict {
                    question_id: qid,
                    answer: final_answer,
                    payee: "factory".into(),
                },
                &[],
            )
            .unwrap();
        }
    }
    app.execute_contract(
        Addr::unchecked("factory"),
        market.clone(),
        &ExecuteMsg::Resolve {},
        &[],
    )
    .unwrap();
    let settled = public_accounting(app, market);
    assert_eq!(settled.challenge, Uint128::zero(), "trace={trace:#?}");
    let slashed = challenge_path == 1 || challenge_path == 3;
    assert_eq!(
        settled.lp_accrual,
        if slashed {
            Uint128::new(10_000_000)
        } else {
            Uint128::zero()
        }
    );
    if slashed {
        big(10_000_000)
    } else {
        BigUint::zero()
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 24, max_shrink_iters: 512, ..ProptestConfig::default() })]
    #[test]
    fn challenge_resolution_and_terminal_claims_match_biguint_model(
        alice_split in MIN_TRADE..=2_000_000u128,
        bob_split in MIN_TRADE..=2_000_000u128,
        gross in MIN_TRADE..=2_000_000u128,
        payout_index in 0u8..3,
        challenge_path in 0u8..4,
        lp_first in 1u128..INITIAL,
        force in 0u128..=17,
    ) {
        let (mut app, market) = setup();
        let mut model = Model::new();
        let mut trace = Vec::new();
        for action in [
            Action::Split { user: 0, amount: alice_split },
            Action::Split { user: 1, amount: bob_split },
            Action::Buy { user: 2, yes: gross % 2 == 0, gross },
        ] {
            trace.push(action.clone());
            prop_assert!(model.apply(&action), "model fixture rejected; trace={:#?}", trace);
            prop_assert!(execute_action(&mut app, &market, &action), "contract fixture rejected; trace={:#?}", trace);
            assert_contract_matches(&app, &market, &model, &trace);
        }
        if force > 0 {
            let action = Action::Force { amount: force };
            trace.push(action.clone());
            prop_assert!(model.apply(&action));
            prop_assert!(execute_action(&mut app, &market, &action));
        }
        let payout = match payout_index { 0 => TerminalPayout::Yes, 1 => TerminalPayout::No, _ => TerminalPayout::Neutral };
        let mut accrual = finalize_for_model(&mut app, &market, payout, challenge_path, &trace);
        let resolved = public_accounting(&app, &market);
        prop_assert_eq!(resolved.principal_at_resolution.unwrap().u128(), model.principal.to_u128().unwrap());
        prop_assert_eq!(resolved.fees_at_resolution.unwrap().u128(), model.fees.to_u128().unwrap());
        prop_assert_eq!(resolved.terminal_liability_twice.unwrap().u128(), (&model.principal * big(2)).to_u128().unwrap());
        prop_assert_eq!(resolved.pool_yes_at_resolution.unwrap().u128(), model.pool_yes.to_u128().unwrap());
        prop_assert_eq!(resolved.pool_no_at_resolution.unwrap().u128(), model.pool_no.to_u128().unwrap());

        let (yes_num, no_num) = payout.nums();
        let mut terminal = &model.principal * big(2);
        let mut half_dust = 0u8;
        for (index, user) in USERS.iter().enumerate() {
            let p = &model.positions[index];
            let numerator = &p.yes * &yes_num + &p.no * &no_num;
            let before = app.wrap().query_balance(*user, "ujuno").unwrap().amount.u128();
            app.execute_contract(Addr::unchecked(*user), market.clone(),
                &ExecuteMsg::RedeemPositions {
                    yes: Uint128::new(p.yes.to_u128().unwrap()),
                    no: Uint128::new(p.no.to_u128().unwrap()),
                }, &[]).unwrap();
            let after = app.wrap().query_balance(*user, "ujuno").unwrap().amount.u128();
            prop_assert_eq!(after - before, (&numerator / big(2)).to_u128().unwrap(), "trace={:#?}", trace);
            terminal -= &numerator;
            if (&numerator % big(2)) == big(1) {
                if half_dust == 1 { accrual += big(1); half_dust = 0; } else { half_dust = 1; }
            }
            let actual = public_accounting(&app, &market);
            prop_assert_eq!(actual.terminal_liability_twice.unwrap().u128(), terminal.to_u128().unwrap());
            prop_assert_eq!(actual.lp_accrual.u128(), accrual.to_u128().unwrap());
        }

        let q2 = &model.pool_yes * &yes_num + &model.pool_no * &no_num;
        let supply = big(INITIAL);
        let mut burned = BigUint::zero();
        for amount in [lp_first, INITIAL - lp_first] {
            burned += big(amount);
            let position_paid = ((&q2 * &burned / &supply) / big(2)).to_u128().unwrap();
            let fee_paid = (&model.fees * &burned / &supply).to_u128().unwrap();
            app.execute_contract(Addr::unchecked("creator"), market.clone(),
                &ExecuteMsg::RedeemLp { amount: Uint128::new(amount) }, &[]).unwrap();
            prop_assert_eq!(public_accounting(&app, &market).lp_paid.u128(), position_paid + fee_paid,
                "LP cumulative floor; trace={:#?}", trace);
        }
        terminal -= &q2;
        if (&q2 % big(2)) == big(1) {
            if half_dust == 1 { accrual += big(1); half_dust = 0; } else { half_dust = 1; }
        }
        let after_lp = public_accounting(&app, &market);
        prop_assert!(terminal.is_zero(), "terminal conservation; trace={:#?}", trace);
        prop_assert_eq!(after_lp.terminal_liability_twice, Some(Uint128::zero()));
        prop_assert_eq!(after_lp.neutral_half_dust, half_dust);
        prop_assert_eq!(after_lp.lp_accrual.u128(), accrual.to_u128().unwrap());
        if !accrual.is_zero() {
            app.execute_contract(Addr::unchecked("creator"), market.clone(),
                &ExecuteMsg::ClaimLpAccrual {}, &[]).unwrap();
        }
        let terminal_state = public_accounting(&app, &market);
        prop_assert_eq!(terminal_state.fees, Uint128::zero());
        prop_assert_eq!(terminal_state.challenge, Uint128::zero());
        prop_assert_eq!(terminal_state.lp_accrual, Uint128::zero());
        prop_assert_eq!(terminal_state.total_yes, Uint128::zero());
        prop_assert_eq!(terminal_state.total_no, Uint128::zero());
        prop_assert_eq!(market_bank(&app, &market), force, "forced excess/no sweep; trace={:#?}", trace);

        let before_rejection = (public_accounting(&app, &market), market_bank(&app, &market));
        let rejected = app
            .execute_contract(
                Addr::unchecked("creator"),
                market.clone(),
                &ExecuteMsg::RedeemLp { amount: Uint128::one() },
                &[],
            )
            .is_err();
        prop_assert!(rejected);
        prop_assert_eq!((public_accounting(&app, &market), market_bank(&app, &market)), before_rejection);
    }
}
