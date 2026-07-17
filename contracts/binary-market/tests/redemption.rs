use binary_market::{
    contract::{execute, instantiate},
    msg::{ExecuteMsg, InstantiateMsg},
    question::{ObservationInput, QuestionInput, SourceInput},
    state::{self, Accounting, Position},
};
use cosmwasm_std::{
    coin,
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, BankMsg, CosmosMsg, Uint128,
};
use pm_types::{Payout, TierId};

fn instantiate_msg() -> InstantiateMsg {
    InstantiateMsg {
        factory: "factory".into(),
        creator: "creator".into(),
        oracle: "oracle".into(),
        verdict_authority: "governance".into(),
        tier: TierId(1),
        question: QuestionInput {
            title: "Redemption fixture?".into(),
            proposition: "Will the redemption fixture pass?".into(),
            definitions: vec![],
            invalid_conditions: vec!["The fixture is withdrawn.".into()],
            primary_sources: vec![SourceInput {
                publisher: "Juno PM".into(),
                identifier: "tests/redemption".into(),
                url: "https://example.com/redemption".into(),
                retrieval: "HTTPS JSON".into(),
                publication_revision_policy: "Latest before opening controls.".into(),
                fallback_condition: "Unavailable for 72 hours.".into(),
            }],
            secondary_sources: vec![],
            source_disagreement_policy: "The primary source controls.".into(),
            observation: ObservationInput {
                start_ts: 1_572_000_000,
                end_ts: 1_572_086_400,
                cutoff_ts: 1_572_086_400,
                inclusivity: "inclusive".into(),
                revision_policy: "Corrections before opening control.".into(),
            },
        },
        nonce: 90,
        close_ts: 1_572_000_000,
        opening_ts: 1_572_086_400,
        initial_liquidity: Uint128::new(100),
        oracle_bounty: Uint128::new(1_000_000),
        oracle_initial_bond: Uint128::new(10_000_000),
        answer_timeout_secs: 86_400,
        arbitration_timeout_secs: 1_814_400,
        fee_bps: 200,
        min_trade: Uint128::one(),
        max_trade_bps: 2_500,
        max_position_per_side: Uint128::MAX,
        collateral_cap: Uint128::new(10_000),
        challenge_bond: Uint128::new(10_000_000),
    }
}

type TestDeps = cosmwasm_std::OwnedDeps<
    cosmwasm_std::MemoryStorage,
    cosmwasm_std::testing::MockApi,
    cosmwasm_std::testing::MockQuerier,
>;

fn resolved(payout: Payout, positions: &[(&str, u128, u128)]) -> TestDeps {
    let mut deps = mock_dependencies();
    instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info("factory", &[coin(1_000_100, "ujuno")]),
        instantiate_msg(),
    )
    .unwrap();
    state::LIFECYCLE
        .update(
            &mut deps.storage,
            |mut lifecycle| -> cosmwasm_std::StdResult<_> {
                lifecycle.activated = true;
                lifecycle.payout = Some(payout);
                Ok(lifecycle)
            },
        )
        .unwrap();
    let total_yes: u128 = positions.iter().map(|(_, yes, _)| yes).sum();
    let total_no: u128 = positions.iter().map(|(_, _, no)| no).sum();
    let principal = total_yes.max(total_no);
    state::ACCOUNTING
        .save(
            &mut deps.storage,
            &Accounting {
                principal: Uint128::new(principal),
                fees: Uint128::zero(),
                challenge: Uint128::zero(),
                pool_yes: Uint128::zero(),
                pool_no: Uint128::zero(),
                total_yes: Uint128::new(total_yes),
                total_no: Uint128::new(total_no),
                lp_supply: Uint128::zero(),
                lp_burned: Uint128::zero(),
                lp_paid: Uint128::zero(),
                neutral_half_dust: 0,
                lp_accrual: Uint128::zero(),
                principal_at_resolution: Some(Uint128::new(principal)),
                fees_at_resolution: Some(Uint128::zero()),
                terminal_liability_twice: Some(Uint128::new(total_yes + total_no)),
                pool_yes_at_resolution: Some(Uint128::zero()),
                pool_no_at_resolution: Some(Uint128::zero()),
                total_yes_at_resolution: Some(Uint128::new(total_yes)),
                total_no_at_resolution: Some(Uint128::new(total_no)),
            },
        )
        .unwrap();
    for (owner, yes, no) in positions {
        state::POSITIONS
            .save(
                &mut deps.storage,
                &Addr::unchecked(*owner),
                &Position {
                    yes: Uint128::new(*yes),
                    no: Uint128::new(*no),
                },
            )
            .unwrap();
    }
    deps
}

fn redeem(deps: &mut TestDeps, owner: &str, yes: u128, no: u128) -> cosmwasm_std::Response {
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(owner, &[]),
        ExecuteMsg::RedeemPositions {
            yes: Uint128::new(yes),
            no: Uint128::new(no),
        },
    )
    .unwrap()
}

fn sent(response: &cosmwasm_std::Response) -> Uint128 {
    response
        .messages
        .first()
        .map_or(Uint128::zero(), |message| match &message.msg {
            CosmosMsg::Bank(BankMsg::Send { amount, .. }) => amount[0].amount,
            _ => panic!("unexpected redemption message"),
        })
}

#[test]
fn valid_redemption_pays_only_winners_and_debits_t2_twice() {
    let mut deps = resolved(
        Payout::for_outcome(pm_types::Outcome::Yes),
        &[("alice", 3, 4)],
    );
    let winning = redeem(&mut deps, "alice", 2, 1);
    assert_eq!(sent(&winning), Uint128::new(2));
    assert_eq!(
        state::ACCOUNTING
            .load(&deps.storage)
            .unwrap()
            .terminal_liability_twice,
        Some(Uint128::new(3))
    );
    let event = winning.events.first().unwrap();
    for key in [
        "account",
        "yes_burned",
        "no_burned",
        "paid",
        "terminal_liability_numerator_after",
    ] {
        assert!(event
            .attributes
            .iter()
            .any(|attribute| attribute.key == key));
    }

    let losing = redeem(&mut deps, "alice", 0, 2);
    assert_eq!(sent(&losing), Uint128::zero());
    assert_eq!(
        state::ACCOUNTING
            .load(&deps.storage)
            .unwrap()
            .terminal_liability_twice,
        Some(Uint128::new(3))
    );
    assert_eq!(
        state::load_position(&deps.storage, &Addr::unchecked("alice")).unwrap(),
        Position {
            yes: Uint128::one(),
            no: Uint128::one(),
        }
    );

    let mut no_wins = resolved(
        Payout::for_outcome(pm_types::Outcome::No),
        &[("alice", 3, 4)],
    );
    assert_eq!(sent(&redeem(&mut no_wins, "alice", 1, 3)), Uint128::new(3));
    assert_eq!(
        state::ACCOUNTING
            .load(&no_wins.storage)
            .unwrap()
            .terminal_liability_twice,
        Some(Uint128::one())
    );
}

#[test]
fn neutral_partial_partitions_equal_aggregate_and_finalize_odd_half() {
    let mut partial = resolved(Payout::neutral(), &[("alice", 2, 3)]);
    assert_eq!(sent(&redeem(&mut partial, "alice", 1, 0)), Uint128::zero());
    assert_eq!(sent(&redeem(&mut partial, "alice", 0, 2)), Uint128::one());
    assert_eq!(sent(&redeem(&mut partial, "alice", 1, 1)), Uint128::one());

    let mut aggregate = resolved(Payout::neutral(), &[("alice", 2, 3)]);
    assert_eq!(
        sent(&redeem(&mut aggregate, "alice", 2, 3)),
        Uint128::new(2)
    );
    assert_eq!(
        state::ACCOUNTING.load(&partial.storage).unwrap(),
        state::ACCOUNTING.load(&aggregate.storage).unwrap()
    );
    assert_eq!(
        state::NEUTRAL_REDEMPTIONS
            .load(&partial.storage, &Addr::unchecked("alice"))
            .unwrap(),
        state::NEUTRAL_REDEMPTIONS
            .load(&aggregate.storage, &Addr::unchecked("alice"))
            .unwrap()
    );
    let accounting = state::ACCOUNTING.load(&partial.storage).unwrap();
    assert_eq!(accounting.terminal_liability_twice, Some(Uint128::zero()));
    assert_eq!(accounting.neutral_half_dust, 1);
    assert_eq!(accounting.lp_accrual, Uint128::zero());
}

#[test]
fn neutral_half_dust_pairs_to_lp_and_address_splitting_cannot_help_callers() {
    let mut split = resolved(Payout::neutral(), &[("alice", 1, 0), ("bob", 0, 1)]);
    assert_eq!(sent(&redeem(&mut split, "alice", 1, 0)), Uint128::zero());
    assert_eq!(sent(&redeem(&mut split, "bob", 0, 1)), Uint128::zero());
    let split_accounting = state::ACCOUNTING.load(&split.storage).unwrap();
    assert_eq!(split_accounting.neutral_half_dust, 0);
    assert_eq!(split_accounting.lp_accrual, Uint128::one());
    assert_eq!(
        split_accounting.terminal_liability_twice,
        Some(Uint128::zero())
    );

    let mut combined = resolved(Payout::neutral(), &[("alice", 1, 1)]);
    assert_eq!(sent(&redeem(&mut combined, "alice", 1, 1)), Uint128::one());
    assert_eq!(
        state::ACCOUNTING
            .load(&combined.storage)
            .unwrap()
            .lp_accrual,
        Uint128::zero()
    );
}

#[test]
fn neutral_seeded_partitions_never_outpay_aggregate_and_abandoned_units_remain() {
    for seed in [1u64, 0x5eed, 0xdead_beef] {
        let yes = (seed as u128 % 19) + 1;
        let no = ((seed >> 4) as u128 % 19) + 1;
        let mut deps = resolved(Payout::neutral(), &[("alice", yes, no)]);
        let mut random = seed;
        let mut yes_left = yes;
        let mut no_left = no;
        let mut paid = Uint128::zero();
        while yes_left + no_left > 1 {
            random = random
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            if yes_left > 0 && random & 1 == 0 {
                yes_left -= 1;
                paid += sent(&redeem(&mut deps, "alice", 1, 0));
            } else if no_left > 0 {
                no_left -= 1;
                paid += sent(&redeem(&mut deps, "alice", 0, 1));
            } else {
                yes_left -= 1;
                paid += sent(&redeem(&mut deps, "alice", 1, 0));
            }
        }
        assert_eq!(paid, Uint128::new((yes + no - 1) / 2));
        let before = state::ACCOUNTING.load(&deps.storage).unwrap();
        assert_eq!(
            before.neutral_half_dust, 0,
            "abandoned unit must not finalize"
        );
        assert_eq!(before.terminal_liability_twice, Some(Uint128::one()));
        paid += sent(&redeem(&mut deps, "alice", yes_left, no_left));
        assert_eq!(paid, Uint128::new((yes + no) / 2));
    }
}

#[test]
fn redemption_rejects_empty_overdraw_attached_funds_and_repeated_claims() {
    let mut deps = resolved(Payout::neutral(), &[("alice", 1, 1)]);
    let before = state::ACCOUNTING.load(&deps.storage).unwrap();
    for (yes, no) in [(0, 0), (2, 0), (0, 2)] {
        assert!(execute(
            deps.as_mut(),
            mock_env(),
            mock_info("alice", &[]),
            ExecuteMsg::RedeemPositions {
                yes: Uint128::new(yes),
                no: Uint128::new(no),
            },
        )
        .is_err());
    }
    assert!(execute(
        deps.as_mut(),
        mock_env(),
        mock_info("alice", &[coin(1, "ujuno")]),
        ExecuteMsg::RedeemPositions {
            yes: Uint128::one(),
            no: Uint128::one(),
        },
    )
    .is_err());
    assert_eq!(state::ACCOUNTING.load(&deps.storage).unwrap(), before);
    redeem(&mut deps, "alice", 1, 1);
    assert!(execute(
        deps.as_mut(),
        mock_env(),
        mock_info("alice", &[]),
        ExecuteMsg::RedeemPositions {
            yes: Uint128::one(),
            no: Uint128::zero(),
        },
    )
    .is_err());
}
