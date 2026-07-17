use binary_market::{
    contract::{execute, instantiate, reply, REPLY_ACTIVATION, REPLY_GOVERNANCE_VERDICT},
    error::ContractError,
    guards,
    msg::{ExecuteMsg, InstantiateMsg, LifecycleStatus},
    question::{ObservationInput, QuestionInput, SourceInput},
    state::{self, Accounting, Challenge, Position, ReplyInProgress},
};
use cosmwasm_std::{
    coin,
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, Binary, Reply, SubMsgResult, Timestamp, Uint128,
};
use cw_reality::state::{AnswerType, Question as OracleQuestion};
use pm_types::TierId;

fn answer(byte: u8) -> Binary {
    Binary::from(vec![byte; 32])
}
fn msg() -> InstantiateMsg {
    InstantiateMsg {
        factory: "factory".into(),
        creator: "creator".into(),
        oracle: "oracle".into(),
        verdict_authority: "governance".into(),
        tier: TierId(1),
        question: QuestionInput {
            title: "Example outcome?".into(),
            proposition: "Will the published example outcome be yes?".into(),
            definitions: vec![],
            invalid_conditions: vec!["The source is permanently unavailable.".into()],
            primary_sources: vec![SourceInput {
                publisher: "Example Authority".into(),
                identifier: "example/final".into(),
                url: "https://example.com/final".into(),
                retrieval: "HTTPS JSON".into(),
                publication_revision_policy: "Corrections before opening control.".into(),
                fallback_condition: "Unavailable for 72 hours.".into(),
            }],
            secondary_sources: vec![],
            source_disagreement_policy: "The primary source controls.".into(),
            observation: ObservationInput {
                start_ts: 1_571_900_000,
                end_ts: 1_572_000_000,
                cutoff_ts: 1_572_000_000,
                inclusivity: "inclusive".into(),
                revision_policy: "Corrections before opening control.".into(),
            },
        },
        nonce: 7,
        close_ts: 1_572_000_000,
        opening_ts: 1_572_000_000,
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
        challenge_bond: Uint128::new(20),
    }
}
fn init() -> cosmwasm_std::OwnedDeps<
    cosmwasm_std::MemoryStorage,
    cosmwasm_std::testing::MockApi,
    cosmwasm_std::testing::MockQuerier,
> {
    let mut deps = mock_dependencies();
    instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info("factory", &[coin(1_000_100, "ujuno")]),
        msg(),
    )
    .unwrap();
    deps
}

#[test]
fn instantiate_is_initializing_and_nonfinancial() {
    let deps = init();
    let lifecycle = state::LIFECYCLE.load(&deps.storage).unwrap();
    let accounting = state::ACCOUNTING.load(&deps.storage).unwrap();
    assert_eq!(
        guards::derived_lifecycle(
            1_571_999_999,
            &state::CONFIG.load(&deps.storage).unwrap(),
            &lifecycle,
            None
        ),
        LifecycleStatus::Initializing
    );
    assert!(!lifecycle.activated);
    assert_eq!(accounting.principal, Uint128::zero());
    assert!(state::QUESTION_ID
        .may_load(&deps.storage)
        .unwrap()
        .is_none());
}

#[test]
fn instantiate_rejects_wrong_no_multiple_funds_and_sender() {
    for funds in [
        vec![],
        vec![coin(1_000_099, "ujuno")],
        vec![coin(1_000_100, "uatom")],
        vec![coin(100, "ujuno"), coin(10, "ujuno")],
    ] {
        let mut deps = mock_dependencies();
        assert!(matches!(
            instantiate(
                deps.as_mut(),
                mock_env(),
                mock_info("factory", &funds),
                msg()
            ),
            Err(ContractError::InvalidFunds { .. })
        ));
    }
    let mut deps = mock_dependencies();
    assert_eq!(
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("intruder", &[coin(1_000_100, "ujuno")]),
            msg()
        )
        .unwrap_err(),
        ContractError::Unauthorized
    );
}

#[test]
fn instantiate_validation_rejects_bad_boundaries() {
    let mut invalid = msg();
    invalid.opening_ts = invalid.close_ts - 1;
    let mut deps = mock_dependencies();
    assert!(matches!(
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("factory", &[coin(1_000_100, "ujuno")]),
            invalid
        ),
        Err(ContractError::InvalidConfig(_))
    ));
    let mut invalid = msg();
    invalid.question.title.clear();
    assert!(matches!(
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("factory", &[coin(1_000_100, "ujuno")]),
            invalid
        ),
        Err(ContractError::InvalidConfig(_))
    ));
}

#[test]
fn trading_rejects_exact_close_boundary_and_deadline_is_inclusive() {
    let deps = init();
    let config = state::CONFIG.load(&deps.storage).unwrap();
    let mut lifecycle = state::LIFECYCLE.load(&deps.storage).unwrap();
    lifecycle.activated = true;
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(1_571_999_999);
    assert!(guards::trading(&env, &config, &lifecycle).is_ok());
    assert!(guards::user_deadline(&env, 1_571_999_999).is_ok());
    env.block.time = Timestamp::from_seconds(1_572_000_000);
    assert_eq!(
        guards::trading(&env, &config, &lifecycle).unwrap_err(),
        ContractError::MarketClosed
    );
    env.block.time = Timestamp::from_seconds(1_572_000_001);
    assert_eq!(
        guards::user_deadline(&env, 1_572_000_000).unwrap_err(),
        ContractError::DeadlineExpired
    );
}

#[test]
fn governance_is_exact_sender_and_strictly_before_deadline() {
    let deps = init();
    let config = state::CONFIG.load(&deps.storage).unwrap();
    let challenge = Challenge {
        challenger: Addr::unchecked("challenger"),
        answer: answer(1),
        oracle_bond: Uint128::one(),
        started_at: 100,
        deadline: 200,
        oracle_snapshot: OracleQuestion {
            asker: Addr::unchecked("market"),
            text: config.question.clone(),
            answer_type: AnswerType::Bool,
            bond_denom: config.collateral_denom.clone(),
            initial_bond: config.oracle_initial_bond,
            min_bond: config.oracle_initial_bond,
            answer_timeout_secs: config.answer_timeout_secs,
            arbitrator: Some(Addr::unchecked("market")),
            arbitration_timeout_secs: config.arbitration_timeout_secs,
            arbitration_deadline: None,
            answer_schema: None,
            nonce: config.nonce,
            opening_ts: Some(config.opening_ts),
            bounty: config.oracle_bounty,
            best_answer: Some(answer(1)),
            current_bond: Uint128::one(),
            history_hash: [0; 32],
            round_count: 1,
            finalize_ts: Some(300),
            is_pending_arbitration: false,
            is_claimed: false,
        },
    };
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(199);
    assert!(guards::governance_verdict(
        &env,
        &Addr::unchecked("governance"),
        &config,
        Some(&challenge)
    )
    .is_ok());
    assert_eq!(
        guards::governance_verdict(&env, &Addr::unchecked("other"), &config, Some(&challenge))
            .unwrap_err(),
        ContractError::Unauthorized
    );
    env.block.time = Timestamp::from_seconds(200);
    assert_eq!(
        guards::governance_verdict(
            &env,
            &Addr::unchecked("governance"),
            &config,
            Some(&challenge)
        )
        .unwrap_err(),
        ContractError::ArbitrationDeadlineReached
    );
}

#[test]
fn no_funds_actions_reject_attached_coins_before_stub() {
    let mut deps = init();
    assert_eq!(
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("anyone", &[coin(1, "ujuno")]),
            ExecuteMsg::Resolve {}
        )
        .unwrap_err(),
        ContractError::UnexpectedFunds
    );
}

#[test]
fn reply_ids_are_bound_to_their_rollback_state() {
    let mut deps = init();
    state::REPLY_IN_PROGRESS
        .save(
            &mut deps.storage,
            &ReplyInProgress::Activation {
                expected_question_id: answer(8),
            },
        )
        .unwrap();
    let result = SubMsgResult::Ok(cosmwasm_std::SubMsgResponse {
        events: vec![],
        data: None,
    });
    assert_eq!(
        reply(
            deps.as_mut(),
            mock_env(),
            Reply {
                id: 99,
                result: result.clone()
            }
        )
        .unwrap_err(),
        ContractError::UnknownReplyId(99)
    );
    assert_eq!(
        reply(
            deps.as_mut(),
            mock_env(),
            Reply {
                id: REPLY_GOVERNANCE_VERDICT,
                result: result.clone()
            }
        )
        .unwrap_err(),
        ContractError::ReplyStateMismatch
    );
    let lifecycle_before = state::LIFECYCLE.load(&deps.storage).unwrap();
    let accounting_before = state::ACCOUNTING.load(&deps.storage).unwrap();
    assert!(matches!(
        reply(
            deps.as_mut(),
            mock_env(),
            Reply {
                id: REPLY_ACTIVATION,
                result
            }
        ),
        Err(ContractError::Std(_))
    ));
    assert_eq!(
        state::LIFECYCLE.load(&deps.storage).unwrap(),
        lifecycle_before
    );
    assert_eq!(
        state::ACCOUNTING.load(&deps.storage).unwrap(),
        accounting_before
    );
}

#[test]
fn missing_positions_are_zero_but_other_records_fail() {
    let deps = init();
    assert_eq!(
        state::load_position(&deps.storage, &Addr::unchecked("nobody"))
            .unwrap()
            .yes,
        Uint128::zero()
    );
    assert!(state::QUESTION_ID.load(&deps.storage).is_err());
    assert!(state::CHALLENGE.load(&deps.storage).is_err());
}

#[test]
fn merge_rejects_below_minimum_and_resolved_state_before_mutation() {
    let mut deps = init();
    state::LIFECYCLE
        .update(
            &mut deps.storage,
            |mut lifecycle| -> cosmwasm_std::StdResult<_> {
                lifecycle.activated = true;
                Ok(lifecycle)
            },
        )
        .unwrap();
    state::ACCOUNTING
        .save(
            &mut deps.storage,
            &Accounting {
                principal: Uint128::new(100),
                fees: Uint128::zero(),
                challenge: Uint128::zero(),
                pool_yes: Uint128::new(100),
                pool_no: Uint128::new(100),
                total_yes: Uint128::new(100),
                total_no: Uint128::new(100),
                lp_supply: Uint128::new(100),
                lp_burned: Uint128::zero(),
                lp_paid: Uint128::zero(),
                neutral_half_dust: 0,
                lp_accrual: Uint128::zero(),
                principal_at_resolution: None,
                fees_at_resolution: None,
                terminal_liability_twice: None,
                pool_yes_at_resolution: None,
                pool_no_at_resolution: None,
                total_yes_at_resolution: None,
                total_no_at_resolution: None,
            },
        )
        .unwrap();
    state::POSITIONS
        .save(
            &mut deps.storage,
            &Addr::unchecked("alice"),
            &Position {
                yes: Uint128::one(),
                no: Uint128::one(),
            },
        )
        .unwrap();
    assert_eq!(
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("alice", &[]),
            ExecuteMsg::Merge {
                amount: Uint128::zero(),
            },
        )
        .unwrap_err(),
        ContractError::AmountBelowMinimum {
            minimum: Uint128::one(),
        }
    );
    state::LIFECYCLE
        .update(
            &mut deps.storage,
            |mut lifecycle| -> cosmwasm_std::StdResult<_> {
                lifecycle.payout = Some(pm_types::Payout::neutral());
                Ok(lifecycle)
            },
        )
        .unwrap();
    let before = state::ACCOUNTING.load(&deps.storage).unwrap();
    assert_eq!(
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("alice", &[]),
            ExecuteMsg::Merge {
                amount: Uint128::one(),
            },
        )
        .unwrap_err(),
        ContractError::AlreadyResolved
    );
    assert_eq!(state::ACCOUNTING.load(&deps.storage).unwrap(), before);
}
