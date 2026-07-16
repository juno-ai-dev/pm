use binary_market::{
    contract::{execute, instantiate, reply, REPLY_ACTIVATION, REPLY_GOVERNANCE_VERDICT},
    error::ContractError,
    guards,
    msg::{ExecuteMsg, InstantiateMsg, LifecycleStatus},
    state::{self, Challenge, ReplyInProgress},
};
use cosmwasm_std::{
    coin,
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, Binary, Reply, SubMsgResult, Timestamp, Uint128,
};
use pm_types::TierId;

fn answer(byte: u8) -> Binary {
    Binary::from(vec![byte; 32])
}
fn msg() -> InstantiateMsg {
    InstantiateMsg {
        factory: "factory".into(),
        creator: "creator".into(),
        oracle: "oracle".into(),
        governance: "governance".into(),
        tier: TierId(1),
        question: "Will it happen?".into(),
        question_hash: answer(9),
        nonce: 7,
        close_ts: 1_000,
        opening_ts: 1_001,
        initial_liquidity: Uint128::new(100),
        oracle_bounty: Uint128::new(10),
        oracle_initial_bond: Uint128::new(5),
        answer_timeout_secs: 86_400,
        arbitration_timeout_secs: 1_814_400,
        fee_bps: 200,
        min_trade: Uint128::one(),
        max_trade_bps: 2_500,
        collateral_cap: Uint128::new(10_000),
        challenge_bond: Uint128::new(20),
        yes_answer: answer(1),
        no_answer: answer(2),
        invalid_answer: answer(3),
        unresolved_answer: answer(4),
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
        mock_info("factory", &[coin(110, "ujuno")]),
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
            999,
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
        vec![coin(109, "ujuno")],
        vec![coin(110, "uatom")],
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
            mock_info("intruder", &[coin(110, "ujuno")]),
            msg()
        )
        .unwrap_err(),
        ContractError::Unauthorized
    );
}

#[test]
fn instantiate_validation_rejects_bad_boundaries() {
    let mut invalid = msg();
    invalid.opening_ts = 999;
    let mut deps = mock_dependencies();
    assert!(matches!(
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("factory", &[coin(110, "ujuno")]),
            invalid
        ),
        Err(ContractError::InvalidConfig(_))
    ));
    let mut invalid = msg();
    invalid.yes_answer = answer(2);
    assert!(matches!(
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("factory", &[coin(110, "ujuno")]),
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
    env.block.time = Timestamp::from_seconds(999);
    assert!(guards::trading(&env, &config, &lifecycle).is_ok());
    assert!(guards::user_deadline(&env, 999).is_ok());
    env.block.time = Timestamp::from_seconds(1_000);
    assert_eq!(
        guards::trading(&env, &config, &lifecycle).unwrap_err(),
        ContractError::MarketClosed
    );
    env.block.time = Timestamp::from_seconds(1_001);
    assert_eq!(
        guards::user_deadline(&env, 1_000).unwrap_err(),
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
        refundable: true,
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
    assert_eq!(
        reply(
            deps.as_mut(),
            mock_env(),
            Reply {
                id: REPLY_ACTIVATION,
                result
            }
        )
        .unwrap_err(),
        ContractError::NotImplemented
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
