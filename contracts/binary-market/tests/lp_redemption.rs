use binary_market::{
    contract::{execute, query},
    error::ContractError,
    msg::{AccountingResponse, ExecuteMsg, QueryMsg},
    state::{self, Accounting, Config, Lifecycle},
};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, Binary, DepsMut, Empty, Env, MessageInfo, Response, Uint128,
};
use cw_multi_test::{App, AppBuilder, ContractWrapper, Executor};
use pm_types::{Outcome, Payout, ProtocolVersion, TierId};

fn config() -> Config {
    Config {
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
        oracle_bounty: Uint128::new(1_000_000),
        oracle_initial_bond: Uint128::new(10_000_000),
        answer_timeout_secs: 86_400,
        arbitration_timeout_secs: 1_814_400,
        fee_bps: 200,
        min_trade: Uint128::one(),
        max_trade_bps: 2_500,
        collateral_cap: Uint128::new(10_000),
        challenge_bond: Uint128::new(10_000_000),
        yes_answer: Binary::from(vec![1; 32]),
        no_answer: Binary::from(vec![0; 32]),
        invalid_answer: Binary::from(vec![255; 32]),
        unresolved_answer: Binary::from(vec![254; 32]),
        question: "fixture".into(),
        question_hash: Binary::from(vec![9; 32]),
        nonce: 7,
    }
}

fn terminal_numerator(yes: u128, no: u128, payout: &Payout) -> u128 {
    let numerator = yes * payout.yes_numerator.u128() + no * payout.no_numerator.u128();
    numerator * (2 / payout.denominator.u128())
}

fn resolved(
    payout: Payout,
    pool_yes: u128,
    pool_no: u128,
    supply: u128,
    fees: u128,
    accrual: u128,
    half_dust: u8,
) -> cosmwasm_std::OwnedDeps<
    cosmwasm_std::MemoryStorage,
    cosmwasm_std::testing::MockApi,
    cosmwasm_std::testing::MockQuerier,
> {
    let mut deps = mock_dependencies();
    state::CONFIG.save(&mut deps.storage, &config()).unwrap();
    state::LIFECYCLE
        .save(
            &mut deps.storage,
            &Lifecycle {
                activated: true,
                payout: Some(payout.clone()),
                resolution_answer: Some(Binary::from(vec![1; 32])),
                resolution_height: Some(10),
                resolution_time: Some(300),
                challenge_used: false,
            },
        )
        .unwrap();
    let q2 = terminal_numerator(pool_yes, pool_no, &payout);
    // Twenty numerator units remain reserved for traders, proving LP claims are subordinate.
    state::ACCOUNTING
        .save(
            &mut deps.storage,
            &Accounting {
                principal: Uint128::new((q2 + 20).div_ceil(2)),
                fees: Uint128::new(fees),
                challenge: Uint128::zero(),
                pool_yes: Uint128::new(pool_yes),
                pool_no: Uint128::new(pool_no),
                total_yes: Uint128::new(pool_yes + 10),
                total_no: Uint128::new(pool_no + 10),
                lp_supply: Uint128::new(supply),
                lp_burned: Uint128::zero(),
                lp_paid: Uint128::zero(),
                neutral_half_dust: half_dust,
                lp_accrual: Uint128::new(accrual),
                principal_at_resolution: Some(Uint128::new((q2 + 20).div_ceil(2))),
                fees_at_resolution: Some(Uint128::new(fees)),
                terminal_liability_twice: Some(Uint128::new(q2 + 20)),
                pool_yes_at_resolution: Some(Uint128::new(pool_yes)),
                pool_no_at_resolution: Some(Uint128::new(pool_no)),
                total_yes_at_resolution: Some(Uint128::new(pool_yes + 10)),
                total_no_at_resolution: Some(Uint128::new(pool_no + 10)),
            },
        )
        .unwrap();
    deps
}

fn redeem(
    deps: &mut cosmwasm_std::OwnedDeps<
        cosmwasm_std::MemoryStorage,
        cosmwasm_std::testing::MockApi,
        cosmwasm_std::testing::MockQuerier,
    >,
    amount: u128,
) -> cosmwasm_std::Response {
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info("creator", &[]),
        ExecuteMsg::RedeemLp {
            amount: Uint128::new(amount),
        },
    )
    .unwrap()
}

fn sent(response: &Response) -> Uint128 {
    response.messages.first().map_or(Uint128::zero(), |msg| {
        let cosmwasm_std::CosmosMsg::Bank(cosmwasm_std::BankMsg::Send { amount, .. }) = &msg.msg
        else {
            panic!("expected bank send")
        };
        amount[0].amount
    })
}

#[test]
fn yes_no_and_neutral_pay_only_frozen_pool_value_and_fees() {
    for (payout, expected) in [
        (Payout::for_outcome(Outcome::Yes), 12u128),
        (Payout::for_outcome(Outcome::No), 14),
        (Payout::neutral(), 13),
    ] {
        let mut deps = resolved(payout, 7, 9, 3, 5, 0, 0);
        assert_eq!(sent(&redeem(&mut deps, 3)), Uint128::new(expected));
        let accounting = state::ACCOUNTING.load(&deps.storage).unwrap();
        assert_eq!(accounting.lp_burned, Uint128::new(3));
        assert_eq!(accounting.fees, Uint128::zero());
        assert_eq!(accounting.pool_yes, Uint128::zero());
        assert_eq!(accounting.pool_no, Uint128::zero());
        assert_eq!(accounting.total_yes, Uint128::new(10));
        assert_eq!(accounting.total_no, Uint128::new(10));
        assert_eq!(accounting.terminal_liability_twice, Some(Uint128::new(20)));
    }
}

#[test]
fn partial_burns_equal_aggregate_with_cumulative_position_and_fee_floors() {
    let payout = Payout::neutral();
    let mut partial = resolved(payout.clone(), 2, 3, 3, 5, 0, 0);
    let payments = [
        sent(&redeem(&mut partial, 1)),
        sent(&redeem(&mut partial, 1)),
        sent(&redeem(&mut partial, 1)),
    ];
    assert_eq!(payments, [Uint128::one(), Uint128::new(3), Uint128::new(3)]);

    let mut aggregate = resolved(payout, 2, 3, 3, 5, 0, 0);
    assert_eq!(sent(&redeem(&mut aggregate, 3)), Uint128::new(7));
    assert_eq!(
        state::ACCOUNTING.load(&partial.storage).unwrap(),
        state::ACCOUNTING.load(&aggregate.storage).unwrap()
    );
    let accounting = state::ACCOUNTING.load(&partial.storage).unwrap();
    assert_eq!(accounting.neutral_half_dust, 1);
    assert_eq!(accounting.terminal_liability_twice, Some(Uint128::new(20)));
}

#[test]
fn odd_pool_numerator_uses_shared_dust_and_late_accrual_survives_full_burn() {
    let mut deps = resolved(Payout::neutral(), 2, 3, 3, 0, 7, 0);
    state::POSITIONS
        .save(
            &mut deps.storage,
            &Addr::unchecked("alice"),
            &state::Position {
                yes: Uint128::one(),
                no: Uint128::zero(),
            },
        )
        .unwrap();
    assert_eq!(sent(&redeem(&mut deps, 3)), Uint128::new(2));
    let accounting = state::ACCOUNTING.load(&deps.storage).unwrap();
    assert_eq!(accounting.lp_burned, accounting.lp_supply);
    assert_eq!(accounting.neutral_half_dust, 1);
    assert_eq!(accounting.lp_accrual, Uint128::new(7));

    // The user's odd neutral remainder arrives only after every base LP unit is gone.
    let late_dust = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("alice", &[]),
        ExecuteMsg::RedeemPositions {
            yes: Uint128::one(),
            no: Uint128::zero(),
        },
    )
    .unwrap();
    assert_eq!(sent(&late_dust), Uint128::zero());
    let accounting = state::ACCOUNTING.load(&deps.storage).unwrap();
    assert_eq!(accounting.neutral_half_dust, 0);
    assert_eq!(accounting.lp_accrual, Uint128::new(8));

    let claimed = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("creator", &[]),
        ExecuteMsg::ClaimLpAccrual {},
    )
    .unwrap();
    assert_eq!(sent(&claimed), Uint128::new(8));
    assert_eq!(
        state::ACCOUNTING.load(&deps.storage).unwrap().lp_accrual,
        Uint128::zero()
    );
    assert_eq!(
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("creator", &[]),
            ExecuteMsg::ClaimLpAccrual {},
        )
        .unwrap_err(),
        ContractError::EmptyLpAccrual
    );
}

#[test]
fn immutable_lp_only_zero_overdraw_repeated_funds_and_pre_resolution_are_rejected() {
    let mut deps = resolved(Payout::for_outcome(Outcome::Yes), 7, 9, 3, 5, 4, 0);
    let before = state::ACCOUNTING.load(&deps.storage).unwrap();
    for (sender, amount) in [("attacker", 1), ("creator", 0), ("creator", 4)] {
        assert!(execute(
            deps.as_mut(),
            mock_env(),
            mock_info(sender, &[]),
            ExecuteMsg::RedeemLp {
                amount: Uint128::new(amount)
            },
        )
        .is_err());
        assert_eq!(state::ACCOUNTING.load(&deps.storage).unwrap(), before);
    }
    assert!(execute(
        deps.as_mut(),
        mock_env(),
        mock_info("attacker", &[]),
        ExecuteMsg::ClaimLpAccrual {},
    )
    .is_err());
    assert!(execute(
        deps.as_mut(),
        mock_env(),
        mock_info("creator", &[cosmwasm_std::coin(1, "ujuno")]),
        ExecuteMsg::RedeemLp {
            amount: Uint128::one()
        },
    )
    .is_err());
    redeem(&mut deps, 3);
    assert!(execute(
        deps.as_mut(),
        mock_env(),
        mock_info("creator", &[]),
        ExecuteMsg::RedeemLp {
            amount: Uint128::one()
        },
    )
    .is_err());

    let mut unresolved = resolved(Payout::neutral(), 2, 2, 3, 5, 4, 0);
    state::LIFECYCLE
        .update(
            &mut unresolved.storage,
            |mut lifecycle| -> cosmwasm_std::StdResult<_> {
                lifecycle.payout = None;
                Ok(lifecycle)
            },
        )
        .unwrap();
    assert_eq!(
        execute(
            unresolved.as_mut(),
            mock_env(),
            mock_info("creator", &[]),
            ExecuteMsg::RedeemLp {
                amount: Uint128::one()
            },
        )
        .unwrap_err(),
        ContractError::NotResolved
    );
    assert_eq!(
        execute(
            unresolved.as_mut(),
            mock_env(),
            mock_info("creator", &[]),
            ExecuteMsg::ClaimLpAccrual {},
        )
        .unwrap_err(),
        ContractError::NotResolved
    );
}

fn empty_market_instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> Result<Response, ContractError> {
    let fixture = resolved(Payout::for_outcome(Outcome::Yes), 7, 9, 3, 5, 0, 0);
    state::CONFIG.save(deps.storage, &state::CONFIG.load(&fixture.storage)?)?;
    state::LIFECYCLE.save(deps.storage, &state::LIFECYCLE.load(&fixture.storage)?)?;
    state::ACCOUNTING.save(deps.storage, &state::ACCOUNTING.load(&fixture.storage)?)?;
    Ok(Response::new())
}

#[test]
fn bank_send_failure_rolls_back_lp_burn_fee_and_position_debits() {
    let mut app: App = AppBuilder::new().build(|router, _, storage| {
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked("creator"),
                vec![cosmwasm_std::coin(1, "ujuno")],
            )
            .unwrap();
    });
    let code = app.store_code(Box::new(ContractWrapper::new(
        execute,
        empty_market_instantiate,
        query,
    )));
    let market = app
        .instantiate_contract(
            code,
            Addr::unchecked("creator"),
            &Empty {},
            &[],
            "market",
            None,
        )
        .unwrap();
    let before: AccountingResponse = app
        .wrap()
        .query_wasm_smart(&market, &QueryMsg::Accounting {})
        .unwrap();
    // The harness market has no coins, so its generated BankMsg fails after execute mutates storage.
    app.execute_contract(
        Addr::unchecked("creator"),
        market.clone(),
        &ExecuteMsg::RedeemLp {
            amount: Uint128::new(3),
        },
        &[],
    )
    .unwrap_err();
    let after: AccountingResponse = app
        .wrap()
        .query_wasm_smart(&market, &QueryMsg::Accounting {})
        .unwrap();
    assert_eq!(after, before);
}
