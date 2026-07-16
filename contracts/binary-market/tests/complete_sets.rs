use binary_market::{
    contract::{execute, instantiate, query, reply},
    msg::{ExecuteMsg, InstantiateMsg, PositionResponse, QueryMsg},
    question::{ObservationInput, QuestionInput, SourceInput},
    state::Accounting,
};
use cosmwasm_std::{coin, from_json, Addr, Empty, Uint128};
use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
use cw_reality::msg::InstantiateMsg as OracleInstantiateMsg;
use pm_types::TierId;

const NOW: u64 = 1_799_800_000;
const CLOSE: u64 = 1_800_000_000;

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
        title: "Complete-set test?".into(),
        proposition: "Will the complete-set test pass?".into(),
        definitions: vec![],
        invalid_conditions: vec!["The test is withdrawn.".into()],
        primary_sources: vec![SourceInput {
            publisher: "Juno PM".into(),
            identifier: "tests/complete-sets".into(),
            url: "https://example.com/complete-sets".into(),
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

fn setup(cap: u128) -> (App, Addr) {
    let factory = Addr::unchecked("factory");
    let alice = Addr::unchecked("alice");
    let bob = Addr::unchecked("bob");
    let mut app = AppBuilder::new().build(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &factory, vec![coin(2_000_000, "ujuno")])
            .unwrap();
        router
            .bank
            .init_balance(storage, &alice, vec![coin(10_000, "ujuno")])
            .unwrap();
        router
            .bank
            .init_balance(storage, &bob, vec![coin(10_000, "ujuno")])
            .unwrap();
    });
    app.update_block(|block| block.time = cosmwasm_std::Timestamp::from_seconds(NOW));
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
                governance: "governance".into(),
                tier: TierId(1),
                question: question(),
                nonce: 44,
                close_ts: CLOSE,
                opening_ts: CLOSE + 86_400,
                initial_liquidity: Uint128::new(100),
                oracle_bounty: Uint128::new(1_000_000),
                oracle_initial_bond: Uint128::new(10_000_000),
                answer_timeout_secs: 86_400,
                arbitration_timeout_secs: 1_814_400,
                fee_bps: 200,
                min_trade: Uint128::new(10),
                max_trade_bps: 2_500,
                collateral_cap: Uint128::new(cap),
                challenge_bond: Uint128::new(10_000_000),
            },
            &[coin(1_000_100, "ujuno")],
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

#[test]
fn activation_split_merge_and_cap_boundaries_reconcile() {
    let (mut app, market) = setup(130);
    let initial = accounting(&app, &market);
    assert_eq!(initial.principal, Uint128::new(100));
    assert_eq!(initial.total_yes, initial.principal);
    assert_eq!(initial.total_no, initial.principal);
    assert_eq!(initial.lp_supply, Uint128::new(100));
    assert_eq!(initial.pool_yes, Uint128::new(100));

    for (owner, amount) in [("alice", 10u128), ("bob", 20u128)] {
        app.execute_contract(
            Addr::unchecked(owner),
            market.clone(),
            &ExecuteMsg::Split {
                amount: Uint128::new(amount),
            },
            &[coin(amount, "ujuno")],
        )
        .unwrap();
    }
    let at_cap = accounting(&app, &market);
    assert_eq!(at_cap.principal, Uint128::new(130));
    assert_eq!(at_cap.total_yes, at_cap.principal);
    assert_eq!(at_cap.total_no, at_cap.principal);
    assert_eq!(at_cap.pool_yes, Uint128::new(100));
    assert!(app
        .execute_contract(
            Addr::unchecked("alice"),
            market.clone(),
            &ExecuteMsg::Split {
                amount: Uint128::new(10),
            },
            &[coin(10, "ujuno")],
        )
        .is_err());
    assert_eq!(accounting(&app, &market), at_cap);

    app.execute_contract(
        Addr::unchecked("bob"),
        market.clone(),
        &ExecuteMsg::Merge {
            amount: Uint128::new(10),
        },
        &[],
    )
    .unwrap();
    assert_eq!(position(&app, &market, "bob").yes, Uint128::new(10));
    assert_eq!(accounting(&app, &market).principal, Uint128::new(120));
    assert!(app
        .execute_contract(
            Addr::unchecked("alice"),
            market.clone(),
            &ExecuteMsg::Merge {
                amount: Uint128::new(20),
            },
            &[],
        )
        .is_err());
}

#[test]
fn split_rejects_bad_amount_funds_and_close_without_state_change() {
    let (mut app, market) = setup(1_000);
    let before = accounting(&app, &market);
    for (amount, funds) in [
        (0u128, vec![coin(0, "ujuno")]),
        (9, vec![coin(9, "ujuno")]),
        (10, vec![]),
        (10, vec![coin(10, "uatom")]),
        (10, vec![coin(10, "ujuno"), coin(1, "uatom")]),
    ] {
        assert!(app
            .execute_contract(
                Addr::unchecked("alice"),
                market.clone(),
                &ExecuteMsg::Split {
                    amount: Uint128::new(amount),
                },
                &funds,
            )
            .is_err());
        assert_eq!(accounting(&app, &market), before);
    }
    app.update_block(|block| block.time = cosmwasm_std::Timestamp::from_seconds(CLOSE));
    assert!(app
        .execute_contract(
            Addr::unchecked("alice"),
            market.clone(),
            &ExecuteMsg::Split {
                amount: Uint128::new(10),
            },
            &[coin(10, "ujuno")],
        )
        .is_err());
    assert_eq!(accounting(&app, &market), before);
}

#[test]
fn merge_works_after_close_and_failed_bank_send_rolls_back() {
    let (mut app, market) = setup(1_000);
    app.execute_contract(
        Addr::unchecked("alice"),
        market.clone(),
        &ExecuteMsg::Split {
            amount: Uint128::new(20),
        },
        &[coin(20, "ujuno")],
    )
    .unwrap();
    app.update_block(|block| block.time = cosmwasm_std::Timestamp::from_seconds(CLOSE + 1));
    app.execute_contract(
        Addr::unchecked("alice"),
        market.clone(),
        &ExecuteMsg::Merge {
            amount: Uint128::new(10),
        },
        &[],
    )
    .unwrap();

    let before_accounting = accounting(&app, &market);
    let before_position = position(&app, &market, "alice");
    app.init_modules(|router, _, storage| {
        router.bank.init_balance(storage, &market, vec![]).unwrap();
    });
    assert!(app
        .execute_contract(
            Addr::unchecked("alice"),
            market.clone(),
            &ExecuteMsg::Merge {
                amount: Uint128::new(10),
            },
            &[],
        )
        .is_err());
    assert_eq!(accounting(&app, &market), before_accounting);
    assert_eq!(position(&app, &market, "alice"), before_position);
}

#[test]
fn forced_funds_and_seeded_random_sequences_do_not_create_claims() {
    // Fixed seeds make failures reproducible while exercising varied owners,
    // amounts, split/merge ordering, and forced bank transfers.
    for initial_seed in [1u64, 0x5eed, 0xdead_beef] {
        let (mut app, market) = setup(10_000);
        let mut alice = 0u128;
        let mut bob = 0u128;
        let mut forced_excess = 0u128;
        let mut random = initial_seed;

        for step in 0..96u64 {
            random = random
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            let (owner, balance) = if random & 1 == 0 {
                ("alice", &mut alice)
            } else {
                ("bob", &mut bob)
            };
            let amount = (((random >> 8) % 5) as u128 + 1) * 10;

            if (random >> 16) & 1 == 1 && *balance >= amount {
                app.execute_contract(
                    Addr::unchecked(owner),
                    market.clone(),
                    &ExecuteMsg::Merge {
                        amount: Uint128::new(amount),
                    },
                    &[],
                )
                .unwrap();
                *balance -= amount;
            } else {
                app.execute_contract(
                    Addr::unchecked(owner),
                    market.clone(),
                    &ExecuteMsg::Split {
                        amount: Uint128::new(amount),
                    },
                    &[coin(amount, "ujuno")],
                )
                .unwrap();
                *balance += amount;
            }

            if step % 11 == 0 {
                let forced = ((random >> 24) % 7) as u128 + 1;
                app.send_tokens(
                    Addr::unchecked("factory"),
                    market.clone(),
                    &[coin(forced, "ujuno")],
                )
                .unwrap();
                forced_excess += forced;
            }

            let a = accounting(&app, &market);
            assert_eq!(a.total_yes, a.principal);
            assert_eq!(a.total_no, a.principal);
            assert_eq!(a.pool_yes.u128() + alice + bob, a.total_yes.u128());
            assert_eq!(a.pool_no.u128() + alice + bob, a.total_no.u128());
            assert_eq!(
                app.wrap()
                    .query_balance(market.to_string(), "ujuno")
                    .unwrap()
                    .amount,
                a.principal + Uint128::new(forced_excess)
            );
        }
    }
}
