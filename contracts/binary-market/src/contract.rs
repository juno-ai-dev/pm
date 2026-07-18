#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env, Event, MessageInfo, Reply,
    ReplyOn, Response, StdError, StdResult, SubMsg, Uint128, Uint256, Uint512, WasmMsg,
};

use crate::{
    error::ContractError,
    guards,
    math::{self, BuyQuote, Reserves, SellQuote},
    msg::{
        ConfigResponse, ExactRatio, ExecuteMsg, IdentityResponse, ImpactDirection, InstantiateMsg,
        LifecycleStatus, PositionResponse, QueryMsg, QuestionResponse, QuoteResponse,
        StateResponse,
    },
    question,
    state::{self, Accounting, Config, Lifecycle, Position, ReplyInProgress},
};
use cw_reality::{
    hash::next_history_hash,
    msg::{
        ExecuteMsg as OracleExecuteMsg, FinalAnswerResponse as OracleFinalAnswerResponse,
        QueryMsg as OracleQueryMsg, QuestionResponse as OracleQuestionResponse,
    },
    state::{AnswerType, Question as OracleQuestion, State as OracleState},
};
use pm_types::{Outcome, Payout, ProtocolVersion, UJUNO_DENOM};

pub const REPLY_ACTIVATION: u64 = 1;
pub const REPLY_CHALLENGE: u64 = 2;
pub const REPLY_GOVERNANCE_VERDICT: u64 = 3;
pub const REPLY_STALLED_CANCELLATION: u64 = 4;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    validate_instantiate(&msg, env.block.time.seconds())?;
    let required = msg.initial_liquidity.checked_add(msg.oracle_bounty)?;
    guards::exact_funds(&info.funds, UJUNO_DENOM, required)?;
    let factory = deps.api.addr_validate(&msg.factory)?;
    guards::sender(&info.sender, &factory)?;
    let creator = deps.api.addr_validate(&msg.creator)?;
    let oracle = deps.api.addr_validate(&msg.oracle)?;
    let verdict_authority = deps.api.addr_validate(&msg.verdict_authority)?;
    let (question_text, question_hash) = question::canonical_question(
        &msg.question,
        &env.contract.address,
        &oracle,
        &verdict_authority,
        msg.close_ts,
        msg.opening_ts,
        msg.oracle_initial_bond,
        env.block.time.seconds(),
    )?;
    let expected_question_id = question::question_id(
        deps.api,
        &oracle,
        &env.contract.address,
        msg.nonce,
        &question::hash_array(&question_hash)?,
        msg.answer_timeout_secs,
        msg.oracle_initial_bond,
        msg.opening_ts,
    )?;
    let config = Config {
        protocol_version: ProtocolVersion::V1,
        factory,
        initial_lp: creator.clone(),
        creator,
        oracle,
        verdict_authority,
        tier: msg.tier,
        collateral_denom: UJUNO_DENOM.to_owned(),
        close_ts: msg.close_ts,
        opening_ts: msg.opening_ts,
        initial_liquidity: msg.initial_liquidity,
        oracle_bounty: msg.oracle_bounty,
        oracle_initial_bond: msg.oracle_initial_bond,
        answer_timeout_secs: msg.answer_timeout_secs,
        arbitration_timeout_secs: msg.arbitration_timeout_secs,
        fee_bps: msg.fee_bps,
        min_trade: msg.min_trade,
        max_trade_bps: msg.max_trade_bps,
        max_position_per_side: msg.max_position_per_side,
        collateral_cap: msg.collateral_cap,
        challenge_bond: msg.challenge_bond,
        yes_answer: Binary::from(hex::decode(question::YES_HEX).expect("valid constant")),
        no_answer: Binary::from(hex::decode(question::NO_HEX).expect("valid constant")),
        invalid_answer: Binary::from(hex::decode(question::INVALID_HEX).expect("valid constant")),
        unresolved_answer: Binary::from(
            hex::decode(question::UNRESOLVED_HEX).expect("valid constant"),
        ),
        question: question_text.clone(),
        question_hash: question_hash.clone(),
        nonce: msg.nonce,
    };
    state::CONFIG.save(deps.storage, &config)?;
    state::LIFECYCLE.save(
        deps.storage,
        &Lifecycle {
            activated: false,
            payout: None,
            resolution_answer: None,
            resolution_height: None,
            resolution_time: None,
            challenge_used: false,
        },
    )?;
    state::ACCOUNTING.save(
        deps.storage,
        &Accounting {
            principal: Uint128::zero(),
            fees: Uint128::zero(),
            challenge: Uint128::zero(),
            pool_yes: Uint128::zero(),
            pool_no: Uint128::zero(),
            total_yes: Uint128::zero(),
            total_no: Uint128::zero(),
            lp_supply: Uint128::zero(),
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
    )?;
    state::REPLY_IN_PROGRESS.save(
        deps.storage,
        &ReplyInProgress::Activation {
            expected_question_id: expected_question_id.clone(),
        },
    )?;
    let ask = OracleExecuteMsg::AskQuestion {
        text: question_text,
        answer_type: AnswerType::Bool,
        bond_denom: UJUNO_DENOM.to_owned(),
        initial_bond: msg.oracle_initial_bond,
        answer_timeout_secs: msg.answer_timeout_secs,
        arbitrator: Some(env.contract.address.to_string()),
        arbitration_timeout_secs: Some(msg.arbitration_timeout_secs),
        answer_schema: None,
        opening_ts: Some(msg.opening_ts),
        nonce: msg.nonce,
    };
    let submsg = SubMsg {
        id: REPLY_ACTIVATION,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.oracle.to_string(),
            msg: to_json_binary(&ask)?,
            funds: vec![cosmwasm_std::coin(msg.oracle_bounty.u128(), UJUNO_DENOM)],
        }),
        gas_limit: None,
        reply_on: ReplyOn::Success,
    };
    Ok(Response::new()
        .add_submessage(submsg)
        .add_attribute("action", "instantiate")
        .add_attribute("verdict_authority", config.verdict_authority.to_string())
        .add_attribute("status", "initializing"))
}

fn validate_instantiate(msg: &InstantiateMsg, creation_ts: u64) -> Result<(), ContractError> {
    if msg.close_ts <= creation_ts || msg.opening_ts < msg.close_ts {
        return Err(ContractError::InvalidConfig(
            "opening_ts must be at or after a future close_ts".into(),
        ));
    }
    if msg.initial_liquidity.is_zero() || msg.collateral_cap < msg.initial_liquidity {
        return Err(ContractError::InvalidConfig(
            "liquidity must be positive and within cap".into(),
        ));
    }
    if msg.oracle_bounty < Uint128::new(question::MIN_ORACLE_BOUNTY)
        || msg.oracle_initial_bond < Uint128::new(question::MIN_ORACLE_INITIAL_BOND)
        || msg.answer_timeout_secs != question::ANSWER_TIMEOUT_SECS
        || msg.arbitration_timeout_secs != question::ARBITRATION_TIMEOUT_SECS
        || msg.challenge_bond.is_zero()
    {
        return Err(ContractError::InvalidConfig(
            "oracle parameters must match accepted v1 bounds".into(),
        ));
    }
    if msg.fee_bps > 10_000
        || msg.max_trade_bps == 0
        || msg.max_trade_bps > 2_500
        || msg.min_trade.is_zero()
        || msg.max_position_per_side.is_zero()
    {
        return Err(ContractError::InvalidConfig(
            "invalid fee or trade bounds".into(),
        ));
    }
    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let config = state::CONFIG.load(deps.storage)?;
    let lifecycle = state::LIFECYCLE.load(deps.storage)?;
    let challenge = state::CHALLENGE.may_load(deps.storage)?;
    match msg {
        ExecuteMsg::Split { amount } => {
            guards::trading(&env, &config, &lifecycle)?;
            guards::exact_funds(&info.funds, UJUNO_DENOM, amount)?;
            execute_split(deps, env, info, &config, amount)
        }
        ExecuteMsg::Buy {
            outcome,
            min_out,
            deadline,
        } => {
            guards::trading(&env, &config, &lifecycle)?;
            guards::user_deadline(&env, deadline)?;
            if info.funds.len() != 1
                || info.funds[0].denom != UJUNO_DENOM
                || info.funds[0].amount.is_zero()
            {
                return Err(ContractError::InvalidFunds {
                    expected: Uint128::zero(),
                    denom: UJUNO_DENOM.into(),
                });
            }
            let gross = info.funds[0].amount;
            execute_buy(deps, env, info, &config, outcome, min_out, gross)
        }
        ExecuteMsg::Sell {
            outcome,
            return_amount,
            max_in,
            deadline,
        } => {
            guards::no_funds(&info.funds)?;
            guards::trading(&env, &config, &lifecycle)?;
            guards::user_deadline(&env, deadline)?;
            execute_sell(deps, env, info, &config, outcome, return_amount, max_in)
        }
        ExecuteMsg::Merge { amount } => {
            guards::no_funds(&info.funds)?;
            if !lifecycle.activated {
                return Err(ContractError::NotActivated);
            }
            guards::unresolved(&lifecycle)?;
            execute_merge(deps, env, info, &config, amount)
        }
        ExecuteMsg::Challenge {} => {
            if guards::derived_lifecycle(
                env.block.time.seconds(),
                &config,
                &lifecycle,
                challenge.as_ref(),
            ) != LifecycleStatus::AwaitingResolution
                || lifecycle.challenge_used
            {
                return Err(ContractError::NoPendingChallenge);
            }
            execute_challenge(deps, env, info, &config)
        }
        ExecuteMsg::GovernanceVerdict {
            question_id,
            answer,
            payee,
        } => {
            guards::no_funds(&info.funds)?;
            guards::governance_verdict(&env, &info.sender, &config, challenge.as_ref())?;
            execute_governance_verdict(
                deps,
                env,
                &config,
                challenge.as_ref().expect("guarded"),
                question_id,
                answer,
                payee,
            )
        }
        ExecuteMsg::FinalizeStalledChallenge {} => {
            guards::no_funds(&info.funds)?;
            let challenge = challenge
                .as_ref()
                .ok_or(ContractError::NoPendingChallenge)?;
            execute_finalize_stalled(deps, env, &config, challenge)
        }
        ExecuteMsg::Resolve {} => {
            guards::no_funds(&info.funds)?;
            if guards::derived_lifecycle(
                env.block.time.seconds(),
                &config,
                &lifecycle,
                challenge.as_ref(),
            ) != LifecycleStatus::AwaitingResolution
            {
                return Err(if lifecycle.payout.is_some() {
                    ContractError::AlreadyResolved
                } else {
                    ContractError::ResolutionMismatch("market is not awaiting resolution".into())
                });
            }
            execute_resolve(deps, env, &config)
        }
        ExecuteMsg::RedeemPositions { yes, no } => {
            guards::no_funds(&info.funds)?;
            let payout = lifecycle.payout.ok_or(ContractError::NotResolved)?;
            execute_redeem_positions(deps, env, info, &config, &payout, yes, no)
        }
        ExecuteMsg::RedeemLp { amount } => {
            guards::no_funds(&info.funds)?;
            let payout = lifecycle.payout.ok_or(ContractError::NotResolved)?;
            execute_redeem_lp(deps, env, info, &config, &payout, amount)
        }
        ExecuteMsg::ClaimLpAccrual {} => {
            guards::no_funds(&info.funds)?;
            lifecycle.payout.ok_or(ContractError::NotResolved)?;
            execute_claim_lp_accrual(deps, env, info, &config)
        }
    }
}

fn query_oracle_question(
    deps: Deps,
    config: &Config,
    question_id: &Binary,
) -> Result<OracleQuestionResponse, ContractError> {
    Ok(deps.querier.query_wasm_smart(
        config.oracle.clone(),
        &OracleQueryMsg::Question {
            question_id: question_id.clone(),
        },
    )?)
}

fn verify_challengeable_question(
    actual: &OracleQuestionResponse,
    expected_id: &Binary,
    config: &Config,
    market: &cosmwasm_std::Addr,
    now: u64,
) -> Result<(Binary, Uint128), ContractError> {
    let q = &actual.question;
    let answer = q.best_answer.clone().ok_or_else(|| {
        ContractError::ArbitrationMismatch("question has no current answer".into())
    })?;
    let valid = actual.question_id == *expected_id
        && actual.state == OracleState::OpenAnswered
        && q.asker == *market
        && q.text == config.question
        && q.answer_type == AnswerType::Bool
        && q.bond_denom == config.collateral_denom
        && q.initial_bond == config.oracle_initial_bond
        && q.answer_timeout_secs == config.answer_timeout_secs
        && q.arbitrator.as_ref() == Some(market)
        && q.arbitration_timeout_secs == config.arbitration_timeout_secs
        && q.arbitration_deadline.is_none()
        && q.nonce == config.nonce
        && q.opening_ts == Some(config.opening_ts)
        && !q.is_pending_arbitration
        && q.finalize_ts.is_some_and(|deadline| now < deadline)
        && !q.current_bond.is_zero();
    if !valid {
        return Err(ContractError::ArbitrationMismatch(
            "question is not the exact bound, live OpenAnswered question".into(),
        ));
    }
    Ok((answer, q.current_bond))
}

fn execute_challenge(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    config: &Config,
) -> Result<Response, ContractError> {
    let now = env.block.time.seconds();
    execute_challenge_at(deps, env, info, config, now)
}

fn execute_challenge_at(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    config: &Config,
    now: u64,
) -> Result<Response, ContractError> {
    let question_id = state::QUESTION_ID.load(deps.storage)?;
    let actual = query_oracle_question(deps.as_ref(), config, &question_id)?;
    let (answer, oracle_bond) =
        verify_challengeable_question(&actual, &question_id, config, &env.contract.address, now)?;
    let required = config.challenge_bond.max(oracle_bond);
    guards::exact_funds(&info.funds, UJUNO_DENOM, required)?;
    let deadline = now
        .checked_add(u64::from(config.arbitration_timeout_secs))
        .ok_or(ContractError::ArbitrationDeadlineOverflow)?;
    state::CHALLENGE.save(
        deps.storage,
        &state::Challenge {
            challenger: info.sender.clone(),
            answer: answer.clone(),
            oracle_bond,
            started_at: now,
            deadline,
            oracle_snapshot: actual.question,
        },
    )?;
    state::LIFECYCLE.update(deps.storage, |mut lifecycle| -> StdResult<_> {
        lifecycle.challenge_used = true;
        Ok(lifecycle)
    })?;
    state::ACCOUNTING.update(deps.storage, |mut accounting| -> StdResult<_> {
        accounting.challenge = required;
        Ok(accounting)
    })?;
    state::REPLY_IN_PROGRESS.save(
        deps.storage,
        &ReplyInProgress::Challenge {
            challenger: info.sender.clone(),
        },
    )?;
    let request = OracleExecuteMsg::RequestArbitration {
        question_id: question_id.clone(),
        current_bond_seen: Some(oracle_bond),
    };
    Ok(Response::new()
        .add_submessage(SubMsg::reply_on_success(
            WasmMsg::Execute {
                contract_addr: config.oracle.to_string(),
                msg: to_json_binary(&request)?,
                funds: vec![],
            },
            REPLY_CHALLENGE,
        ))
        .add_event(
            Event::new("juno_pm_v1")
                .add_attribute("action", "challenge_requested")
                .add_attribute("protocol_version", "1")
                .add_attribute("factory", config.factory.to_string())
                .add_attribute("market", env.contract.address)
                .add_attribute("height", env.block.height.to_string())
                .add_attribute("block_time", env.block.time.seconds().to_string())
                .add_attribute("authority", config.verdict_authority.to_string())
                .add_attribute("challenger", info.sender)
                .add_attribute("question_id", question_id.to_base64())
                .add_attribute("answer_hex", hex::encode(answer.as_slice()))
                .add_attribute("answer_base64", answer.to_base64())
                .add_attribute("oracle_bond", oracle_bond)
                .add_attribute("challenge_bond", required)
                .add_attribute("arbitration_deadline", deadline.to_string()),
        ))
}

fn verify_pending_challenge(
    actual: &OracleQuestionResponse,
    question_id: &Binary,
    challenge: &state::Challenge,
    deadline: u64,
) -> Result<(), ContractError> {
    let mut expected = challenge.oracle_snapshot.clone();
    expected.is_pending_arbitration = true;
    expected.arbitration_deadline = Some(deadline);
    if actual.question_id != *question_id
        || actual.state != OracleState::PendingArbitration
        || actual.question != expected
    {
        return Err(ContractError::ArbitrationMismatch(
            "oracle pending state does not match the challenge snapshot".into(),
        ));
    }
    Ok(())
}

fn execute_governance_verdict(
    deps: DepsMut,
    env: Env,
    config: &Config,
    challenge: &state::Challenge,
    question_id: Binary,
    answer: Binary,
    payee: String,
) -> Result<Response, ContractError> {
    let expected_id = state::QUESTION_ID.load(deps.storage)?;
    if question_id != expected_id {
        return Err(ContractError::ArbitrationMismatch(
            "wrong question id".into(),
        ));
    }
    if answer.is_empty() {
        return Err(ContractError::InvalidVerdictAnswer);
    }
    let payee = deps.api.addr_validate(&payee)?;
    let actual = query_oracle_question(deps.as_ref(), config, &question_id)?;
    verify_pending_challenge(&actual, &question_id, challenge, challenge.deadline)?;
    state::REPLY_IN_PROGRESS.save(
        deps.storage,
        &ReplyInProgress::GovernanceVerdict {
            answer: answer.clone(),
            payee: payee.clone(),
        },
    )?;
    let submit = OracleExecuteMsg::SubmitArbitration {
        question_id: question_id.clone(),
        winning_answer: answer.clone(),
        payee: payee.to_string(),
    };
    Ok(Response::new()
        .add_submessage(SubMsg::reply_on_success(
            WasmMsg::Execute {
                contract_addr: config.oracle.to_string(),
                msg: to_json_binary(&submit)?,
                funds: vec![],
            },
            REPLY_GOVERNANCE_VERDICT,
        ))
        .add_event(
            Event::new("juno_pm_v1")
                .add_attribute("action", "governance_verdict_forwarded")
                .add_attribute("protocol_version", "1")
                .add_attribute("factory", config.factory.to_string())
                .add_attribute("market", env.contract.address)
                .add_attribute("height", env.block.height.to_string())
                .add_attribute("block_time", env.block.time.seconds().to_string())
                .add_attribute("authority", config.verdict_authority.to_string())
                .add_attribute("question_id", question_id.to_base64())
                .add_attribute("answer_hex", hex::encode(answer.as_slice()))
                .add_attribute("answer_base64", answer.to_base64())
                .add_attribute("payee", payee.to_string()),
        ))
}

fn verify_finalized_verdict(
    api: &dyn cosmwasm_std::Api,
    actual: &OracleQuestionResponse,
    question_id: &Binary,
    challenge: &state::Challenge,
    answer: &Binary,
    payee: &cosmwasm_std::Addr,
    now: u64,
) -> Result<(), ContractError> {
    let snapshot = &challenge.oracle_snapshot;
    let history_hash = next_history_hash(
        api,
        &snapshot.history_hash,
        answer,
        &snapshot.bond_denom,
        Uint128::zero(),
        payee,
        false,
    )?;
    let round_count = snapshot
        .round_count
        .checked_add(1)
        .ok_or(ContractError::ArbitrationRoundOverflow)?;
    let mut expected: OracleQuestion = snapshot.clone();
    expected.is_pending_arbitration = false;
    expected.arbitration_deadline = None;
    expected.best_answer = Some(answer.clone());
    expected.history_hash = history_hash;
    expected.round_count = round_count;
    expected.finalize_ts = Some(now);
    if actual.question_id != *question_id
        || actual.state != OracleState::Finalized
        || actual.question != expected
    {
        return Err(ContractError::ArbitrationMismatch(
            "oracle did not apply the exact forwarded verdict history transition".into(),
        ));
    }
    Ok(())
}

fn settle_challenge(
    deps: DepsMut,
    env: &Env,
    config: &Config,
    refund: bool,
    reason: &str,
) -> Result<Response, ContractError> {
    let challenge = state::CHALLENGE.load(deps.storage)?;
    let amount = state::ACCOUNTING.load(deps.storage)?.challenge;
    if amount.is_zero() || amount != config.challenge_bond.max(challenge.oracle_bond) {
        return Err(ContractError::InvariantViolation(
            "challenge liability is absent or differs from escrow".into(),
        ));
    }
    state::ACCOUNTING.update(deps.storage, |mut accounting| {
        if accounting.challenge != amount {
            return Err(ContractError::InvariantViolation(
                "challenge liability changed during settlement".into(),
            ));
        }
        accounting.challenge = Uint128::zero();
        if !refund {
            accounting.lp_accrual = accounting.lp_accrual.checked_add(amount)?;
        }
        Ok(accounting)
    })?;
    state::CHALLENGE.remove(deps.storage);
    state::REPLY_IN_PROGRESS.remove(deps.storage);
    let event = Event::new("juno_pm_v1")
        .add_attribute(
            "action",
            if refund {
                "challenge_refunded"
            } else {
                "challenge_slashed"
            },
        )
        .add_attribute("protocol_version", "1")
        .add_attribute("factory", config.factory.to_string())
        .add_attribute("market", env.contract.address.to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("block_time", env.block.time.seconds().to_string())
        .add_attribute("authority", config.verdict_authority.to_string())
        .add_attribute("challenger", challenge.challenger.to_string())
        .add_attribute("amount", amount)
        .add_attribute(
            "recipient",
            if refund {
                challenge.challenger.to_string()
            } else {
                config.initial_lp.to_string()
            },
        )
        .add_attribute(
            "disposition",
            if refund { "refunded" } else { "slashed_to_lp" },
        )
        .add_attribute("reason", reason);
    let response = Response::new().add_event(event);
    if refund {
        Ok(response.add_message(BankMsg::Send {
            to_address: challenge.challenger.to_string(),
            amount: vec![cosmwasm_std::coin(amount.u128(), UJUNO_DENOM)],
        }))
    } else {
        Ok(response)
    }
}

fn verify_cancelled(
    actual: &OracleQuestionResponse,
    question_id: &Binary,
    challenge: &state::Challenge,
    synchronization_now: u64,
) -> Result<(), ContractError> {
    let timeout = u64::from(challenge.oracle_snapshot.answer_timeout_secs);
    let earliest = challenge
        .deadline
        .checked_add(timeout)
        .ok_or(ContractError::ArbitrationDeadlineOverflow)?;
    let latest = synchronization_now
        .checked_add(timeout)
        .ok_or(ContractError::ArbitrationDeadlineOverflow)?;
    let finalize_ts = actual.question.finalize_ts;
    let mut expected = challenge.oracle_snapshot.clone();
    expected.is_pending_arbitration = false;
    expected.arbitration_deadline = None;
    expected.finalize_ts = finalize_ts;
    if actual.question_id != *question_id
        || actual.state != OracleState::OpenAnswered
        || actual.question != expected
        || !finalize_ts.is_some_and(|ts| earliest <= ts && ts <= latest)
    {
        return Err(ContractError::ArbitrationMismatch(
            "oracle cancellation does not match the challenge snapshot or re-extension window"
                .into(),
        ));
    }
    Ok(())
}

fn execute_finalize_stalled(
    deps: DepsMut,
    env: Env,
    config: &Config,
    challenge: &state::Challenge,
) -> Result<Response, ContractError> {
    if env.block.time.seconds() < challenge.deadline {
        return Err(ContractError::ArbitrationDeadlineNotReached);
    }
    let question_id = state::QUESTION_ID.load(deps.storage)?;
    let actual = query_oracle_question(deps.as_ref(), config, &question_id)?;
    if actual.state == OracleState::PendingArbitration {
        verify_pending_challenge(&actual, &question_id, challenge, challenge.deadline)?;
        state::REPLY_IN_PROGRESS.save(deps.storage, &ReplyInProgress::StalledCancellation)?;
        let challenge_bond = state::ACCOUNTING.load(deps.storage)?.challenge;
        let event = Event::new("juno_pm_v1")
            .add_attribute("action", "arbitration_stalled")
            .add_attribute("protocol_version", "1")
            .add_attribute("factory", config.factory.to_string())
            .add_attribute("market", env.contract.address.to_string())
            .add_attribute("height", env.block.height.to_string())
            .add_attribute("block_time", env.block.time.seconds().to_string())
            .add_attribute("question_id", question_id.to_base64())
            .add_attribute("arbitration_deadline", challenge.deadline.to_string())
            .add_attribute("challenge_bond", challenge_bond);
        return Ok(Response::new()
            .add_submessage(SubMsg::reply_on_success(
                WasmMsg::Execute {
                    contract_addr: config.oracle.to_string(),
                    msg: to_json_binary(&OracleExecuteMsg::CancelArbitration { question_id })?,
                    funds: vec![],
                },
                REPLY_STALLED_CANCELLATION,
            ))
            .add_event(event));
    }
    verify_cancelled(&actual, &question_id, challenge, env.block.time.seconds())?;
    settle_challenge(deps, &env, config, false, "oracle_already_cancelled")
}

fn execute_redeem_positions(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    config: &Config,
    payout: &Payout,
    yes: Uint128,
    no: Uint128,
) -> Result<Response, ContractError> {
    if yes.is_zero() && no.is_zero() {
        return Err(ContractError::EmptyRedemption);
    }
    let mut position = state::load_position(deps.storage, &info.sender)?;
    if position.yes < yes || position.no < no {
        return Err(ContractError::InsufficientPosition);
    }
    position.yes = position.yes.checked_sub(yes)?;
    position.no = position.no.checked_sub(no)?;

    let mut accounting = state::ACCOUNTING.load(deps.storage)?;
    let mut terminal = accounting
        .terminal_liability_twice
        .ok_or(ContractError::NotResolved)?;
    let payment;
    if *payout == Payout::for_outcome(Outcome::Yes) || *payout == Payout::for_outcome(Outcome::No) {
        payment = yes
            .checked_mul(payout.yes_numerator)?
            .checked_add(no.checked_mul(payout.no_numerator)?)?;
        terminal = terminal.checked_sub(payment.checked_mul(Uint128::new(2))?)?;
    } else if *payout == Payout::neutral() {
        let burned = yes.checked_add(no)?;
        let mut redemption = state::NEUTRAL_REDEMPTIONS
            .may_load(deps.storage, &info.sender)?
            .unwrap_or(state::NeutralRedemption {
                cumulative_numerator: Uint128::zero(),
                whole_paid: Uint128::zero(),
                finalized_half: false,
            });
        if redemption.finalized_half {
            return Err(ContractError::InvariantViolation(
                "neutral remainder was already finalized".into(),
            ));
        }
        redemption.cumulative_numerator = redemption.cumulative_numerator.checked_add(burned)?;
        let whole_credit = Uint128::new(redemption.cumulative_numerator.u128() / 2);
        payment = whole_credit.checked_sub(redemption.whole_paid)?;
        redemption.whole_paid = whole_credit;
        terminal = terminal.checked_sub(payment.checked_mul(Uint128::new(2))?)?;

        if position.yes.is_zero()
            && position.no.is_zero()
            && redemption.cumulative_numerator.u128() % 2 == 1
        {
            redemption.finalized_half = true;
            terminal = terminal.checked_sub(Uint128::one())?;
            assign_half_dust(&mut accounting)?;
        }
        state::NEUTRAL_REDEMPTIONS.save(deps.storage, &info.sender, &redemption)?;
    } else {
        return Err(ContractError::InvariantViolation(
            "unsupported terminal payout vector".into(),
        ));
    }

    accounting.total_yes = accounting.total_yes.checked_sub(yes)?;
    accounting.total_no = accounting.total_no.checked_sub(no)?;
    accounting.terminal_liability_twice = Some(terminal);
    state::POSITIONS.save(deps.storage, &info.sender, &position)?;
    state::ACCOUNTING.save(deps.storage, &accounting)?;

    let event = Event::new("juno_pm_v1")
        .add_attribute("protocol_version", "1")
        .add_attribute("factory", config.factory.to_string())
        .add_attribute("market", env.contract.address.to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("block_time", env.block.time.seconds().to_string())
        .add_attribute("action", "positions_redeemed")
        .add_attribute("account", info.sender.to_string())
        .add_attribute("yes_burned", yes.to_string())
        .add_attribute("no_burned", no.to_string())
        .add_attribute("paid", payment.to_string())
        .add_attribute("terminal_liability_numerator_after", terminal.to_string());
    let response = Response::new().add_event(event);
    if payment.is_zero() {
        Ok(response)
    } else {
        Ok(response.add_message(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![cosmwasm_std::coin(payment.u128(), UJUNO_DENOM)],
        }))
    }
}

fn proportional_floor(
    total: Uint128,
    burned: Uint128,
    supply: Uint128,
) -> Result<Uint128, ContractError> {
    if supply.is_zero() {
        return Err(ContractError::InvariantViolation(
            "zero fixed LP supply".into(),
        ));
    }
    let value = Uint256::from(total)
        .checked_mul(Uint256::from(burned))
        .map_err(|error| ContractError::Math(error.to_string()))?
        .checked_div(Uint256::from(supply))
        .map_err(|error| ContractError::Math(error.to_string()))?;
    Uint128::try_from(value).map_err(|error| ContractError::Math(error.to_string()))
}

fn pool_terminal_numerator(
    yes: Uint128,
    no: Uint128,
    payout: &Payout,
) -> Result<Uint128, ContractError> {
    let scale = match payout.denominator.u128() {
        1 => Uint128::new(2),
        2 => Uint128::one(),
        _ => {
            return Err(ContractError::InvariantViolation(
                "unsupported payout denominator".into(),
            ))
        }
    };
    Ok(yes
        .checked_mul(payout.yes_numerator)?
        .checked_add(no.checked_mul(payout.no_numerator)?)?
        .checked_mul(scale)?)
}

fn assign_half_dust(accounting: &mut Accounting) -> Result<(), ContractError> {
    accounting.neutral_half_dust = accounting
        .neutral_half_dust
        .checked_add(1)
        .ok_or_else(|| ContractError::InvariantViolation("half-dust overflow".into()))?;
    if accounting.neutral_half_dust == 2 {
        accounting.neutral_half_dust = 0;
        accounting.lp_accrual = accounting.lp_accrual.checked_add(Uint128::one())?;
    }
    Ok(())
}

fn execute_redeem_lp(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    config: &Config,
    payout: &Payout,
    amount: Uint128,
) -> Result<Response, ContractError> {
    guards::sender(&info.sender, &config.initial_lp)?;
    if amount.is_zero() {
        return Err(ContractError::EmptyLpRedemption);
    }
    let mut accounting = state::ACCOUNTING.load(deps.storage)?;
    let supply = accounting.lp_supply;
    let burned_after = accounting.lp_burned.checked_add(amount)?;
    if burned_after > supply {
        return Err(ContractError::InsufficientLpUnits);
    }
    let pool_yes = accounting
        .pool_yes_at_resolution
        .ok_or(ContractError::NotResolved)?;
    let pool_no = accounting
        .pool_no_at_resolution
        .ok_or(ContractError::NotResolved)?;
    let fees = accounting
        .fees_at_resolution
        .ok_or(ContractError::NotResolved)?;
    let q2 = pool_terminal_numerator(pool_yes, pool_no, payout)?;
    let allocated_after = proportional_floor(q2, burned_after, supply)?;
    let position_whole_after = allocated_after / Uint128::new(2);
    let fee_whole_after = proportional_floor(fees, burned_after, supply)?;
    let entitled_after = position_whole_after.checked_add(fee_whole_after)?;
    let payment = entitled_after.checked_sub(accounting.lp_paid)?;

    let yes_removed_after = proportional_floor(pool_yes, burned_after, supply)?;
    let no_removed_after = proportional_floor(pool_no, burned_after, supply)?;
    let yes_removed_before = pool_yes.checked_sub(accounting.pool_yes)?;
    let no_removed_before = pool_no.checked_sub(accounting.pool_no)?;
    let yes_delta = yes_removed_after.checked_sub(yes_removed_before)?;
    let no_delta = no_removed_after.checked_sub(no_removed_before)?;
    let fee_paid_before = proportional_floor(fees, accounting.lp_burned, supply)?;
    let fee_delta = fee_whole_after.checked_sub(fee_paid_before)?;
    let position_paid_before =
        proportional_floor(q2, accounting.lp_burned, supply)? / Uint128::new(2);
    let position_delta = position_whole_after.checked_sub(position_paid_before)?;

    accounting.lp_burned = burned_after;
    accounting.lp_paid = entitled_after;
    accounting.fees = accounting.fees.checked_sub(fee_delta)?;
    accounting.pool_yes = accounting.pool_yes.checked_sub(yes_delta)?;
    accounting.pool_no = accounting.pool_no.checked_sub(no_delta)?;
    accounting.total_yes = accounting.total_yes.checked_sub(yes_delta)?;
    accounting.total_no = accounting.total_no.checked_sub(no_delta)?;
    let mut terminal = accounting
        .terminal_liability_twice
        .ok_or(ContractError::NotResolved)?;
    terminal = terminal.checked_sub(position_delta.checked_mul(Uint128::new(2))?)?;
    if burned_after == supply && q2.u128() % 2 == 1 {
        terminal = terminal.checked_sub(Uint128::one())?;
        assign_half_dust(&mut accounting)?;
    }
    accounting.terminal_liability_twice = Some(terminal);
    state::ACCOUNTING.save(deps.storage, &accounting)?;

    let event = Event::new("juno_pm_v1")
        .add_attribute("action", "lp_redeemed")
        .add_attribute("protocol_version", "1")
        .add_attribute("factory", config.factory.to_string())
        .add_attribute("market", env.contract.address)
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("block_time", env.block.time.seconds().to_string())
        .add_attribute("lp", info.sender.to_string())
        .add_attribute("lp_burned", amount)
        .add_attribute("position_paid", position_delta)
        .add_attribute("fee_paid", fee_delta)
        .add_attribute("lp_supply_remaining", supply.checked_sub(burned_after)?);
    let response = Response::new().add_event(event);
    if payment.is_zero() {
        Ok(response)
    } else {
        Ok(response.add_message(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![cosmwasm_std::coin(payment.u128(), UJUNO_DENOM)],
        }))
    }
}

fn execute_claim_lp_accrual(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    config: &Config,
) -> Result<Response, ContractError> {
    guards::sender(&info.sender, &config.initial_lp)?;
    let mut accounting = state::ACCOUNTING.load(deps.storage)?;
    let payment = accounting.lp_accrual;
    if payment.is_zero() {
        return Err(ContractError::EmptyLpAccrual);
    }
    accounting.lp_accrual = Uint128::zero();
    state::ACCOUNTING.save(deps.storage, &accounting)?;
    Ok(Response::new()
        .add_message(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![cosmwasm_std::coin(payment.u128(), UJUNO_DENOM)],
        })
        .add_event(
            Event::new("juno_pm_v1")
                .add_attribute("action", "lp_accrual_claimed")
                .add_attribute("protocol_version", "1")
                .add_attribute("factory", config.factory.to_string())
                .add_attribute("market", env.contract.address)
                .add_attribute("height", env.block.height.to_string())
                .add_attribute("block_time", env.block.time.seconds().to_string())
                .add_attribute("lp", info.sender)
                .add_attribute("amount", payment)
                .add_attribute("lp_accrual_after", "0"),
        ))
}

fn execute_resolve(deps: DepsMut, env: Env, config: &Config) -> Result<Response, ContractError> {
    let question_id = state::QUESTION_ID.load(deps.storage)?;
    let final_answer: OracleFinalAnswerResponse = deps.querier.query_wasm_smart(
        config.oracle.clone(),
        &OracleQueryMsg::FinalAnswerIfMatches {
            question_id: question_id.clone(),
            min_bond: Some(config.challenge_bond),
            min_timeout_secs: Some(config.answer_timeout_secs),
            required_arbitrator: Some(env.contract.address.to_string()),
            required_denom: Some(config.collateral_denom.clone()),
        },
    )?;
    let question: OracleQuestionResponse = deps.querier.query_wasm_smart(
        config.oracle.clone(),
        &OracleQueryMsg::Question {
            question_id: question_id.clone(),
        },
    )?;
    verify_resolution(
        &final_answer,
        &question,
        &question_id,
        config,
        &env.contract.address,
    )?;

    let payout = if final_answer.final_answer == config.yes_answer {
        Payout::for_outcome(Outcome::Yes)
    } else if final_answer.final_answer == config.no_answer {
        Payout::for_outcome(Outcome::No)
    } else {
        Payout::neutral()
    };
    let mut accounting = state::ACCOUNTING.load(deps.storage)?;
    assert_pre_resolution(&accounting)?;
    let principal = accounting.principal;
    let terminal_liability_twice = principal.checked_mul(Uint128::new(2))?;
    accounting.principal_at_resolution = Some(principal);
    accounting.fees_at_resolution = Some(accounting.fees);
    accounting.terminal_liability_twice = Some(terminal_liability_twice);
    accounting.pool_yes_at_resolution = Some(accounting.pool_yes);
    accounting.pool_no_at_resolution = Some(accounting.pool_no);
    accounting.total_yes_at_resolution = Some(accounting.total_yes);
    accounting.total_no_at_resolution = Some(accounting.total_no);

    let mut lifecycle = state::LIFECYCLE.load(deps.storage)?;
    if lifecycle.payout.is_some() {
        return Err(ContractError::AlreadyResolved);
    }
    lifecycle.payout = Some(payout.clone());
    lifecycle.resolution_answer = Some(final_answer.final_answer.clone());
    lifecycle.resolution_height = Some(env.block.height);
    lifecycle.resolution_time = Some(env.block.time.seconds());
    state::ACCOUNTING.save(deps.storage, &accounting)?;
    state::LIFECYCLE.save(deps.storage, &lifecycle)?;

    Ok(Response::new().add_event(
        Event::new("juno_pm_v1")
            .add_attribute("action", "market_resolved")
            .add_attribute("protocol_version", "1")
            .add_attribute("factory", config.factory.to_string())
            .add_attribute("market", env.contract.address.to_string())
            .add_attribute("question_id", question_id.to_base64())
            .add_attribute(
                "answer_hex",
                hex::encode(final_answer.final_answer.as_slice()),
            )
            .add_attribute("answer_base64", final_answer.final_answer.to_base64())
            .add_attribute("final_bond", final_answer.final_bond.to_string())
            .add_attribute("payout_yes_num", payout.yes_numerator.to_string())
            .add_attribute("payout_no_num", payout.no_numerator.to_string())
            .add_attribute("payout_den", payout.denominator.to_string())
            .add_attribute("principal_at_resolution", principal.to_string())
            .add_attribute(
                "terminal_liability_numerator",
                terminal_liability_twice.to_string(),
            )
            .add_attribute("height", env.block.height.to_string())
            .add_attribute("block_time", env.block.time.seconds().to_string()),
    ))
}

fn verify_resolution(
    final_answer: &OracleFinalAnswerResponse,
    actual: &OracleQuestionResponse,
    expected_id: &Binary,
    config: &Config,
    market: &cosmwasm_std::Addr,
) -> Result<(), ContractError> {
    let q = &actual.question;
    let terminal_state = matches!(actual.state, OracleState::Finalized | OracleState::Claimed);
    let matches = terminal_state
        && final_answer.question_id == *expected_id
        && actual.question_id == *expected_id
        && q.asker == *market
        && q.text.as_bytes() == config.question.as_bytes()
        && q.answer_type == AnswerType::Bool
        && q.bond_denom == config.collateral_denom
        && q.initial_bond == config.oracle_initial_bond
        && q.min_bond == config.oracle_initial_bond
        && q.answer_timeout_secs == config.answer_timeout_secs
        && q.arbitrator.as_ref() == Some(market)
        && q.arbitration_timeout_secs == config.arbitration_timeout_secs
        && q.arbitration_deadline.is_none()
        && q.answer_schema.is_none()
        && q.nonce == config.nonce
        && q.opening_ts == Some(config.opening_ts)
        && q.bounty >= config.oracle_bounty
        && q.best_answer.as_ref() == Some(&final_answer.final_answer)
        && q.current_bond == final_answer.final_bond
        && q.current_bond >= config.challenge_bond
        && q.finalize_ts.is_some()
        && !q.is_pending_arbitration
        && q.is_claimed == matches!(actual.state, OracleState::Claimed);
    if !matches {
        return Err(ContractError::ResolutionMismatch(
            "final-answer and full-question responses do not exactly match the bound market".into(),
        ));
    }
    Ok(())
}

fn validate_amount(amount: Uint128, minimum: Uint128) -> Result<(), ContractError> {
    if amount < minimum {
        return Err(ContractError::AmountBelowMinimum { minimum });
    }
    Ok(())
}

/// Cheap consensus assertion over aggregate ledgers. Map reconciliation is
/// covered by model tests because maps cannot be summed during execution.
fn assert_pre_resolution(accounting: &Accounting) -> Result<(), ContractError> {
    if accounting.total_yes != accounting.principal
        || accounting.total_no != accounting.principal
        || accounting.pool_yes > accounting.total_yes
        || accounting.pool_no > accounting.total_no
        || accounting.pool_yes.is_zero()
        || accounting.pool_no.is_zero()
    {
        return Err(ContractError::InvariantViolation(
            "Y = N = P and positive bounded pool reserves are required".into(),
        ));
    }
    Ok(())
}

fn complete_set_event(
    action: &str,
    env: &Env,
    config: &Config,
    caller: &cosmwasm_std::Addr,
    amount: Uint128,
    accounting: &Accounting,
) -> Event {
    Event::new("juno_pm_v1")
        .add_attribute("protocol_version", "1")
        .add_attribute("factory", config.factory.to_string())
        .add_attribute("market", env.contract.address.to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("block_time", env.block.time.seconds().to_string())
        .add_attribute("action", action)
        .add_attribute("account", caller.to_string())
        .add_attribute("amount", amount.to_string())
        .add_attribute("principal_after", accounting.principal.to_string())
        .add_attribute("total_yes", accounting.total_yes.to_string())
        .add_attribute("total_no", accounting.total_no.to_string())
        .add_attribute("pool_yes", accounting.pool_yes.to_string())
        .add_attribute("pool_no", accounting.pool_no.to_string())
}

fn execute_split(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    config: &Config,
    amount: Uint128,
) -> Result<Response, ContractError> {
    validate_amount(amount, config.min_trade)?;
    let before = state::ACCOUNTING.load(deps.storage)?;
    assert_pre_resolution(&before)?;
    enforce_configured_ratio(amount, reserves(&before), config.max_trade_bps)?;
    let mut position = state::load_position(deps.storage, &info.sender)?;
    position.yes = position.yes.checked_add(amount)?;
    position.no = position.no.checked_add(amount)?;
    enforce_position_cap(&position, config.max_position_per_side)?;
    let accounting = state::ACCOUNTING.update(deps.storage, |mut accounting| {
        assert_pre_resolution(&accounting)?;
        accounting.principal = accounting.principal.checked_add(amount)?;
        if accounting.principal > config.collateral_cap {
            return Err(ContractError::CollateralCapExceeded);
        }
        accounting.total_yes = accounting.total_yes.checked_add(amount)?;
        accounting.total_no = accounting.total_no.checked_add(amount)?;
        assert_pre_resolution(&accounting)?;
        Ok(accounting)
    })?;
    state::POSITIONS.save(deps.storage, &info.sender, &position)?;
    Ok(Response::new().add_event(complete_set_event(
        "split",
        &env,
        config,
        &info.sender,
        amount,
        &accounting,
    )))
}

fn execute_merge(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    config: &Config,
    amount: Uint128,
) -> Result<Response, ContractError> {
    validate_amount(amount, config.min_trade)?;
    let before = state::ACCOUNTING.load(deps.storage)?;
    assert_pre_resolution(&before)?;
    enforce_configured_ratio(amount, reserves(&before), config.max_trade_bps)?;
    let mut position = state::load_position(deps.storage, &info.sender)?;
    if position.yes < amount || position.no < amount {
        return Err(ContractError::InsufficientPosition);
    }
    let accounting =
        state::ACCOUNTING.update(deps.storage, |mut accounting| -> Result<_, ContractError> {
            assert_pre_resolution(&accounting)?;
            accounting.principal = accounting.principal.checked_sub(amount)?;
            accounting.total_yes = accounting.total_yes.checked_sub(amount)?;
            accounting.total_no = accounting.total_no.checked_sub(amount)?;
            assert_pre_resolution(&accounting)?;
            Ok(accounting)
        })?;
    position.yes = position.yes.checked_sub(amount)?;
    position.no = position.no.checked_sub(amount)?;
    state::POSITIONS.save(deps.storage, &info.sender, &position)?;
    Ok(Response::new()
        .add_message(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![cosmwasm_std::coin(amount.u128(), UJUNO_DENOM)],
        })
        .add_event(complete_set_event(
            "merge",
            &env,
            config,
            &info.sender,
            amount,
            &accounting,
        )))
}

fn reserves(accounting: &Accounting) -> Reserves {
    Reserves {
        yes: accounting.pool_yes,
        no: accounting.pool_no,
    }
}

fn enforce_position_cap(
    position: &state::Position,
    max_position_per_side: Uint128,
) -> Result<(), ContractError> {
    if position.yes > max_position_per_side || position.no > max_position_per_side {
        return Err(ContractError::PositionCapExceeded);
    }
    Ok(())
}

fn enforce_configured_ratio(
    amount: Uint128,
    before: Reserves,
    max_trade_bps: u16,
) -> Result<(), ContractError> {
    let smaller = before.yes.min(before.no);
    let maximum256 = Uint256::from(smaller)
        .checked_mul(Uint256::from(max_trade_bps))
        .map_err(|error| ContractError::Math(error.to_string()))?
        .checked_div(Uint256::from(math::FEE_SCALE))
        .map_err(|error| ContractError::Math(error.to_string()))?;
    let maximum =
        Uint128::try_from(maximum256).map_err(|error| ContractError::Math(error.to_string()))?;
    if amount > maximum {
        return Err(ContractError::ReserveRatioLimitExceeded);
    }
    Ok(())
}

fn buy_quote(
    accounting: &Accounting,
    config: &Config,
    outcome: Outcome,
    gross: Uint128,
) -> Result<BuyQuote, ContractError> {
    validate_amount(gross, config.min_trade)?;
    let quote = math::buy_exact_collateral(reserves(accounting), outcome, gross, config.fee_bps)
        .map_err(|error| ContractError::Math(error.to_string()))?;
    enforce_configured_ratio(
        quote.net_collateral,
        quote.reserves_before,
        config.max_trade_bps,
    )?;
    Ok(quote)
}

fn sell_quote(
    accounting: &Accounting,
    config: &Config,
    outcome: Outcome,
    return_amount: Uint128,
) -> Result<SellQuote, ContractError> {
    validate_amount(return_amount, config.min_trade)?;
    let quote = math::sell_for_exact_collateral(
        reserves(accounting),
        outcome,
        return_amount,
        config.fee_bps,
    )
    .map_err(|error| ContractError::Math(error.to_string()))?;
    enforce_configured_ratio(
        quote.gross_collateral,
        quote.reserves_before,
        config.max_trade_bps,
    )?;
    Ok(quote)
}

#[allow(clippy::too_many_arguments)]
fn trade_event(
    side: &str,
    env: &Env,
    config: &Config,
    caller: &cosmwasm_std::Addr,
    outcome: &Outcome,
    gross: Uint128,
    net: Uint128,
    fee: Uint128,
    input: Uint128,
    output: Uint128,
    before: Reserves,
    accounting: &Accounting,
) -> Event {
    Event::new("juno_pm_v1")
        .add_attribute("protocol_version", "1")
        .add_attribute("factory", config.factory.to_string())
        .add_attribute("market", env.contract.address.to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("block_time", env.block.time.seconds().to_string())
        .add_attribute("action", "trade")
        .add_attribute("side", side)
        .add_attribute(
            "outcome",
            match outcome {
                Outcome::Yes => "yes",
                Outcome::No => "no",
            },
        )
        .add_attribute("account", caller.to_string())
        .add_attribute("gross", gross.to_string())
        .add_attribute("net", net.to_string())
        .add_attribute("fee", fee.to_string())
        .add_attribute("input", input.to_string())
        .add_attribute("output", output.to_string())
        .add_attribute("reserve_yes_before", before.yes.to_string())
        .add_attribute("reserve_no_before", before.no.to_string())
        .add_attribute("reserve_yes_after", accounting.pool_yes.to_string())
        .add_attribute("reserve_no_after", accounting.pool_no.to_string())
        .add_attribute("principal_after", accounting.principal.to_string())
        .add_attribute("fee_liability_after", accounting.fees.to_string())
}

fn execute_buy(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    config: &Config,
    outcome: Outcome,
    min_out: Uint128,
    gross: Uint128,
) -> Result<Response, ContractError> {
    let before = state::ACCOUNTING.load(deps.storage)?;
    assert_pre_resolution(&before)?;
    let quote = buy_quote(&before, config, outcome.clone(), gross)?;
    if quote.outcome_out < min_out {
        return Err(ContractError::SlippageExceeded);
    }
    let mut after = before;
    after.principal = after.principal.checked_add(quote.net_collateral)?;
    if after.principal > config.collateral_cap {
        return Err(ContractError::CollateralCapExceeded);
    }
    after.fees = after.fees.checked_add(quote.fee)?;
    after.total_yes = after.total_yes.checked_add(quote.net_collateral)?;
    after.total_no = after.total_no.checked_add(quote.net_collateral)?;
    after.pool_yes = quote.reserves_after.yes;
    after.pool_no = quote.reserves_after.no;
    assert_pre_resolution(&after)?;
    let mut position = state::load_position(deps.storage, &info.sender)?;
    match outcome {
        Outcome::Yes => position.yes = position.yes.checked_add(quote.outcome_out)?,
        Outcome::No => position.no = position.no.checked_add(quote.outcome_out)?,
    }
    enforce_position_cap(&position, config.max_position_per_side)?;
    state::ACCOUNTING.save(deps.storage, &after)?;
    state::POSITIONS.save(deps.storage, &info.sender, &position)?;
    Ok(Response::new().add_event(trade_event(
        "buy",
        &env,
        config,
        &info.sender,
        &outcome,
        quote.gross_collateral,
        quote.net_collateral,
        quote.fee,
        quote.gross_collateral,
        quote.outcome_out,
        quote.reserves_before,
        &after,
    )))
}

fn execute_sell(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    config: &Config,
    outcome: Outcome,
    return_amount: Uint128,
    max_in: Uint128,
) -> Result<Response, ContractError> {
    let before = state::ACCOUNTING.load(deps.storage)?;
    assert_pre_resolution(&before)?;
    let quote = sell_quote(&before, config, outcome.clone(), return_amount)?;
    if quote.outcome_in > max_in {
        return Err(ContractError::SlippageExceeded);
    }
    let mut position = state::load_position(deps.storage, &info.sender)?;
    let selected = match outcome {
        Outcome::Yes => &mut position.yes,
        Outcome::No => &mut position.no,
    };
    if *selected < quote.outcome_in {
        return Err(ContractError::InsufficientPosition);
    }
    *selected = selected.checked_sub(quote.outcome_in)?;
    let mut after = before;
    after.principal = after.principal.checked_sub(quote.gross_collateral)?;
    after.fees = after.fees.checked_add(quote.fee)?;
    after.total_yes = after.total_yes.checked_sub(quote.gross_collateral)?;
    after.total_no = after.total_no.checked_sub(quote.gross_collateral)?;
    after.pool_yes = quote.reserves_after.yes;
    after.pool_no = quote.reserves_after.no;
    assert_pre_resolution(&after)?;
    state::ACCOUNTING.save(deps.storage, &after)?;
    state::POSITIONS.save(deps.storage, &info.sender, &position)?;
    Ok(Response::new()
        .add_message(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![cosmwasm_std::coin(return_amount.u128(), UJUNO_DENOM)],
        })
        .add_event(trade_event(
            "sell",
            &env,
            config,
            &info.sender,
            &outcome,
            quote.gross_collateral,
            quote.net_collateral,
            quote.fee,
            quote.outcome_in,
            quote.net_collateral,
            quote.reserves_before,
            &after,
        )))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> Result<Response, ContractError> {
    let pending = state::REPLY_IN_PROGRESS.load(deps.storage)?;
    let matches = matches!(
        (reply.id, &pending),
        (REPLY_ACTIVATION, ReplyInProgress::Activation { .. })
            | (REPLY_CHALLENGE, ReplyInProgress::Challenge { .. })
            | (
                REPLY_GOVERNANCE_VERDICT,
                ReplyInProgress::GovernanceVerdict { .. }
            )
            | (
                REPLY_STALLED_CANCELLATION,
                ReplyInProgress::StalledCancellation
            )
    );
    if !matches {
        if ![
            REPLY_ACTIVATION,
            REPLY_CHALLENGE,
            REPLY_GOVERNANCE_VERDICT,
            REPLY_STALLED_CANCELLATION,
        ]
        .contains(&reply.id)
        {
            return Err(ContractError::UnknownReplyId(reply.id));
        }
        return Err(ContractError::ReplyStateMismatch);
    }
    if reply.id != REPLY_ACTIVATION {
        reply
            .result
            .into_result()
            .map_err(ContractError::ArbitrationSubmessage)?;
        let config = state::CONFIG.load(deps.storage)?;
        let question_id = state::QUESTION_ID.load(deps.storage)?;
        let challenge = state::CHALLENGE.load(deps.storage)?;
        let actual = query_oracle_question(deps.as_ref(), &config, &question_id)?;
        return match pending {
            ReplyInProgress::Challenge { challenger } => {
                if challenger != challenge.challenger {
                    return Err(ContractError::ReplyStateMismatch);
                }
                verify_pending_challenge(&actual, &question_id, &challenge, challenge.deadline)?;
                state::REPLY_IN_PROGRESS.remove(deps.storage);
                Ok(Response::new())
            }
            ReplyInProgress::GovernanceVerdict { answer, payee } => {
                verify_finalized_verdict(
                    deps.api,
                    &actual,
                    &question_id,
                    &challenge,
                    &answer,
                    &payee,
                    env.block.time.seconds(),
                )?;
                let refund = answer != challenge.answer;
                settle_challenge(
                    deps,
                    &env,
                    &config,
                    refund,
                    if refund {
                        "different_verdict"
                    } else {
                        "identical_verdict"
                    },
                )
                .map(|response| response.add_attribute("verdict_payee", payee.to_string()))
            }
            ReplyInProgress::StalledCancellation => {
                verify_cancelled(&actual, &question_id, &challenge, env.block.time.seconds())?;
                settle_challenge(deps, &env, &config, false, "arbitration_timeout")
            }
            ReplyInProgress::Activation { .. } => Err(ContractError::ReplyStateMismatch),
        };
    }
    reply
        .result
        .into_result()
        .map_err(ContractError::ActivationMismatch)?;
    let ReplyInProgress::Activation {
        expected_question_id,
    } = pending
    else {
        return Err(ContractError::ReplyStateMismatch);
    };
    let config = state::CONFIG.load(deps.storage)?;
    let actual: OracleQuestionResponse = deps.querier.query_wasm_smart(
        config.oracle.clone(),
        &OracleQueryMsg::Question {
            question_id: expected_question_id.clone(),
        },
    )?;
    verify_oracle_question(
        &actual,
        &expected_question_id,
        &config,
        &env.contract.address,
    )?;
    state::QUESTION_ID.save(deps.storage, &expected_question_id)?;
    state::LIFECYCLE.update(deps.storage, |mut lifecycle| -> StdResult<_> {
        lifecycle.activated = true;
        Ok(lifecycle)
    })?;
    state::ACCOUNTING.update(deps.storage, |mut accounting| -> StdResult<_> {
        accounting.principal = config.initial_liquidity;
        accounting.pool_yes = config.initial_liquidity;
        accounting.pool_no = config.initial_liquidity;
        accounting.total_yes = config.initial_liquidity;
        accounting.total_no = config.initial_liquidity;
        accounting.lp_supply = config.initial_liquidity;
        Ok(accounting)
    })?;
    assert_pre_resolution(&state::ACCOUNTING.load(deps.storage)?)?;
    state::POSITIONS.save(deps.storage, &config.initial_lp, &Position::default())?;
    state::REPLY_IN_PROGRESS.remove(deps.storage);
    Ok(Response::new().add_event(
        cosmwasm_std::Event::new("juno_pm_v1")
            .add_attribute("action", "market_activated")
            .add_attribute("protocol_version", "1")
            .add_attribute("factory", config.factory.to_string())
            .add_attribute("market", env.contract.address.to_string())
            .add_attribute("height", env.block.height.to_string())
            .add_attribute("block_time", env.block.time.seconds().to_string())
            .add_attribute("creator", config.creator.to_string())
            .add_attribute("verdict_authority", config.verdict_authority.to_string())
            .add_attribute("lp", config.initial_lp.to_string())
            .add_attribute("nonce", config.nonce.to_string())
            .add_attribute("question_id", expected_question_id.to_base64())
            .add_attribute("question_hash", config.question_hash.to_base64())
            .add_attribute("close_ts", config.close_ts.to_string())
            .add_attribute("opening_ts", config.opening_ts.to_string()),
    ))
}

fn verify_oracle_question(
    actual: &OracleQuestionResponse,
    expected_id: &Binary,
    config: &Config,
    market: &cosmwasm_std::Addr,
) -> Result<(), ContractError> {
    let q = &actual.question;
    let matches = actual.question_id == *expected_id
        && actual.state == OracleState::OpenUnanswered
        && q.asker == *market
        && q.text.as_bytes() == config.question.as_bytes()
        && q.answer_type == AnswerType::Bool
        && q.bond_denom == UJUNO_DENOM
        && q.initial_bond == config.oracle_initial_bond
        && q.min_bond == config.oracle_initial_bond
        && q.answer_timeout_secs == config.answer_timeout_secs
        && q.arbitrator.as_ref() == Some(market)
        && q.arbitration_timeout_secs == config.arbitration_timeout_secs
        && q.arbitration_deadline.is_none()
        && q.answer_schema.is_none()
        && q.nonce == config.nonce
        && q.opening_ts == Some(config.opening_ts)
        && q.bounty == config.oracle_bounty
        && q.best_answer.is_none()
        && q.current_bond.is_zero()
        && q.history_hash == [0; 32]
        && q.round_count == 0
        && q.finalize_ts.is_none()
        && !q.is_pending_arbitration
        && !q.is_claimed;
    if !matches {
        return Err(ContractError::ActivationMismatch(
            "queried question does not exactly match immutable market parameters".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod activation_verification_tests {
    use super::{
        execute_challenge_at, verify_finalized_verdict, verify_oracle_question, verify_resolution,
    };
    use crate::{
        error::ContractError,
        question,
        state::{self, Accounting, Challenge, Config, Lifecycle},
    };
    use cosmwasm_std::{
        coin, from_json,
        testing::{mock_dependencies, mock_env, mock_info},
        to_json_binary, Addr, Binary, ContractResult, SystemResult, Uint128, WasmQuery,
    };
    use cw_reality::{
        filter::AnswerSchemaFilter,
        msg::{FinalAnswerResponse, QueryMsg as OracleQueryMsg, QuestionResponse},
        state::{AnswerType, Question, State},
    };
    use pm_types::{ProtocolVersion, TierId};

    fn fixture() -> (Config, Addr, Binary, QuestionResponse) {
        let market = Addr::unchecked("market");
        let expected_id = Binary::from(vec![7; 32]);
        let config = Config {
            protocol_version: ProtocolVersion::V1,
            factory: Addr::unchecked("factory"),
            creator: Addr::unchecked("creator"),
            initial_lp: Addr::unchecked("creator"),
            oracle: Addr::unchecked("oracle"),
            verdict_authority: Addr::unchecked("governance"),
            tier: TierId(1),
            collateral_denom: "ujuno".into(),
            close_ts: 1_800_000_000,
            opening_ts: 1_800_086_400,
            initial_liquidity: Uint128::new(100),
            oracle_bounty: Uint128::new(1_000_000),
            oracle_initial_bond: Uint128::new(10_000_000),
            answer_timeout_secs: question::ANSWER_TIMEOUT_SECS,
            arbitration_timeout_secs: question::ARBITRATION_TIMEOUT_SECS,
            fee_bps: 200,
            min_trade: Uint128::one(),
            max_trade_bps: 2_500,
            max_position_per_side: Uint128::MAX,
            collateral_cap: Uint128::new(10_000),
            challenge_bond: Uint128::new(10_000_000),
            yes_answer: Binary::from(vec![1; 32]),
            no_answer: Binary::from(vec![0; 32]),
            invalid_answer: Binary::from(vec![255; 32]),
            unresolved_answer: Binary::from(vec![254; 32]),
            question: "canonical question".into(),
            question_hash: Binary::from(vec![9; 32]),
            nonce: 11,
        };
        let response = QuestionResponse {
            question_id: expected_id.clone(),
            state: State::OpenUnanswered,
            question: Question {
                asker: market.clone(),
                text: config.question.clone(),
                answer_type: AnswerType::Bool,
                bond_denom: "ujuno".into(),
                initial_bond: config.oracle_initial_bond,
                min_bond: config.oracle_initial_bond,
                answer_timeout_secs: config.answer_timeout_secs,
                arbitrator: Some(market.clone()),
                arbitration_timeout_secs: config.arbitration_timeout_secs,
                arbitration_deadline: None,
                answer_schema: None,
                nonce: config.nonce,
                opening_ts: Some(config.opening_ts),
                bounty: config.oracle_bounty,
                best_answer: None,
                current_bond: Uint128::zero(),
                history_hash: [0; 32],
                round_count: 0,
                finalize_ts: None,
                is_pending_arbitration: false,
                is_claimed: false,
            },
        };
        (config, market, expected_id, response)
    }

    #[test]
    fn every_id_omitted_or_initial_state_field_mismatch_rejects_activation() {
        let (config, market, expected_id, exact) = fixture();
        assert!(verify_oracle_question(&exact, &expected_id, &config, &market).is_ok());

        let mut mismatches = Vec::new();
        let mut value = exact.clone();
        value.question.answer_type = AnswerType::Uint;
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.arbitration_timeout_secs += 1;
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.answer_schema = Some(AnswerSchemaFilter {
            contract: "filter".into(),
            filter: serde_json::json!({}),
        });
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.bounty += Uint128::one();
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.arbitration_deadline = Some(config.opening_ts);
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.best_answer = Some(Binary::from(vec![1; 32]));
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.current_bond = Uint128::one();
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.history_hash = [1; 32];
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.round_count = 1;
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.finalize_ts = Some(config.opening_ts);
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.is_pending_arbitration = true;
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.is_claimed = true;
        mismatches.push(value);

        for mismatch in mismatches {
            assert!(verify_oracle_question(&mismatch, &expected_id, &config, &market).is_err());
        }
    }

    fn resolution_fixture() -> (Config, Addr, Binary, FinalAnswerResponse, QuestionResponse) {
        let (config, market, expected_id, mut question) = fixture();
        question.state = State::Finalized;
        question.question.best_answer = Some(config.yes_answer.clone());
        question.question.current_bond = config.challenge_bond;
        question.question.history_hash = [7; 32];
        question.question.round_count = 1;
        question.question.finalize_ts = Some(config.opening_ts + config.answer_timeout_secs as u64);
        let final_answer = FinalAnswerResponse {
            question_id: expected_id.clone(),
            final_answer: config.yes_answer.clone(),
            final_bond: config.challenge_bond,
        };
        (config, market, expected_id, final_answer, question)
    }

    #[test]
    fn final_and_claimed_exact_responses_pass_but_conflicts_and_weak_bonds_reject() {
        let (config, market, expected_id, final_answer, question) = resolution_fixture();
        assert!(
            verify_resolution(&final_answer, &question, &expected_id, &config, &market).is_ok()
        );

        let mut claimed = question.clone();
        claimed.state = State::Claimed;
        claimed.question.is_claimed = true;
        assert!(verify_resolution(&final_answer, &claimed, &expected_id, &config, &market).is_ok());

        for state in [
            State::NotCreated,
            State::OpenUnanswered,
            State::OpenAnswered,
            State::PendingArbitration,
        ] {
            let mut value = question.clone();
            value.state = state;
            assert!(
                verify_resolution(&final_answer, &value, &expected_id, &config, &market).is_err()
            );
        }
        let mut conflict = question.clone();
        conflict.question.best_answer = Some(config.no_answer.clone());
        assert!(
            verify_resolution(&final_answer, &conflict, &expected_id, &config, &market).is_err()
        );
        let mut conflict = question.clone();
        conflict.question.current_bond += Uint128::one();
        assert!(
            verify_resolution(&final_answer, &conflict, &expected_id, &config, &market).is_err()
        );
        let mut weak_final = final_answer.clone();
        weak_final.final_bond = config.challenge_bond - Uint128::one();
        let mut weak_question = question.clone();
        weak_question.question.current_bond = weak_final.final_bond;
        assert!(
            verify_resolution(&weak_final, &weak_question, &expected_id, &config, &market).is_err()
        );
    }

    #[test]
    fn every_bound_question_field_mismatch_rejects_resolution() {
        let (config, market, expected_id, final_answer, exact) = resolution_fixture();
        let mut mismatches = Vec::new();
        let mut value = exact.clone();
        value.question_id = Binary::from(vec![8; 32]);
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.asker = Addr::unchecked("other-market");
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.text.push('!');
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.answer_type = AnswerType::Uint;
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.bond_denom = "other".into();
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.initial_bond += Uint128::one();
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.min_bond += Uint128::one();
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.answer_timeout_secs += 1;
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.arbitrator = Some(Addr::unchecked("other-market"));
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.arbitration_timeout_secs += 1;
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.arbitration_deadline = Some(config.opening_ts);
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.answer_schema = Some(AnswerSchemaFilter {
            contract: "filter".into(),
            filter: serde_json::json!({}),
        });
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.nonce += 1;
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.opening_ts = Some(config.opening_ts + 1);
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.bounty = config.oracle_bounty - Uint128::one();
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.finalize_ts = None;
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.is_pending_arbitration = true;
        mismatches.push(value);
        let mut value = exact.clone();
        value.question.is_claimed = true;
        mismatches.push(value);

        for mismatch in mismatches {
            assert!(
                verify_resolution(&final_answer, &mismatch, &expected_id, &config, &market)
                    .is_err()
            );
        }
        let mut wrong_id = final_answer.clone();
        wrong_id.question_id = Binary::from(vec![9; 32]);
        assert!(verify_resolution(&wrong_id, &exact, &expected_id, &config, &market).is_err());
    }

    fn verdict_fixture() -> (Binary, Binary, Addr, Challenge, QuestionResponse) {
        let (_, _, question_id, mut pre) = fixture();
        let answer = Binary::from(vec![2; 32]);
        let payee = Addr::unchecked("payee");
        pre.question.best_answer = Some(Binary::from(vec![1; 32]));
        pre.question.current_bond = Uint128::new(10_000_000);
        pre.question.history_hash = [3; 32];
        pre.question.round_count = 4;
        pre.question.finalize_ts = Some(20_000);
        pre.state = State::OpenAnswered;
        let challenge = Challenge {
            challenger: Addr::unchecked("challenger"),
            answer: pre.question.best_answer.clone().unwrap(),
            oracle_bond: pre.question.current_bond,
            started_at: 10_000,
            deadline: 11_000,
            oracle_snapshot: pre.question.clone(),
        };
        let mut actual = pre;
        actual.state = State::Finalized;
        actual.question.best_answer = Some(answer.clone());
        actual.question.is_pending_arbitration = false;
        actual.question.arbitration_deadline = None;
        actual.question.round_count += 1;
        actual.question.finalize_ts = Some(12_000);
        (question_id, answer, payee, challenge, actual)
    }

    #[test]
    fn verdict_verifier_rejects_wrong_payee_hash_hash_round_and_immutable_drift() {
        let deps = mock_dependencies();
        let (question_id, answer, payee, challenge, mut exact) = verdict_fixture();
        exact.question.history_hash = cw_reality::hash::next_history_hash(
            deps.as_ref().api,
            &challenge.oracle_snapshot.history_hash,
            &answer,
            &challenge.oracle_snapshot.bond_denom,
            Uint128::zero(),
            &payee,
            false,
        )
        .unwrap();
        assert!(verify_finalized_verdict(
            deps.as_ref().api,
            &exact,
            &question_id,
            &challenge,
            &answer,
            &payee,
            12_000,
        )
        .is_ok());

        let mut wrong_payee_history = exact.clone();
        wrong_payee_history.question.history_hash = cw_reality::hash::next_history_hash(
            deps.as_ref().api,
            &challenge.oracle_snapshot.history_hash,
            &answer,
            &challenge.oracle_snapshot.bond_denom,
            Uint128::zero(),
            &Addr::unchecked("wrong-payee"),
            false,
        )
        .unwrap();
        assert!(verify_finalized_verdict(
            deps.as_ref().api,
            &wrong_payee_history,
            &question_id,
            &challenge,
            &answer,
            &payee,
            12_000,
        )
        .is_err());

        let mut wrong_hash = exact.clone();
        wrong_hash.question.history_hash[0] ^= 1;
        assert!(verify_finalized_verdict(
            deps.as_ref().api,
            &wrong_hash,
            &question_id,
            &challenge,
            &answer,
            &payee,
            12_000,
        )
        .is_err());

        let mut wrong_round = exact.clone();
        wrong_round.question.round_count += 1;
        assert!(verify_finalized_verdict(
            deps.as_ref().api,
            &wrong_round,
            &question_id,
            &challenge,
            &answer,
            &payee,
            12_000,
        )
        .is_err());

        let mut immutable_drift = exact;
        immutable_drift.question.asker = Addr::unchecked("other-market");
        assert!(verify_finalized_verdict(
            deps.as_ref().api,
            &immutable_drift,
            &question_id,
            &challenge,
            &answer,
            &payee,
            12_000,
        )
        .is_err());
    }

    #[test]
    fn challenge_deadline_overflow_rejects_before_any_mutation() {
        let (config, market, question_id, mut oracle_response) = fixture();
        let now = u64::MAX - u64::from(config.arbitration_timeout_secs) + 1;
        oracle_response.state = State::OpenAnswered;
        oracle_response.question.best_answer = Some(Binary::from(vec![1; 32]));
        oracle_response.question.current_bond = config.challenge_bond;
        oracle_response.question.history_hash = [5; 32];
        oracle_response.question.round_count = 1;
        oracle_response.question.finalize_ts = Some(u64::MAX);

        let mut deps = mock_dependencies();
        let query_response = oracle_response.clone();
        deps.querier.update_wasm(move |query| match query {
            WasmQuery::Smart { msg, .. } => {
                let _: OracleQueryMsg = from_json(msg).unwrap();
                SystemResult::Ok(ContractResult::Ok(to_json_binary(&query_response).unwrap()))
            }
            _ => panic!("unexpected query"),
        });
        state::QUESTION_ID
            .save(deps.as_mut().storage, &question_id)
            .unwrap();
        state::LIFECYCLE
            .save(
                deps.as_mut().storage,
                &Lifecycle {
                    activated: true,
                    payout: None,
                    resolution_answer: None,
                    resolution_height: None,
                    resolution_time: None,
                    challenge_used: false,
                },
            )
            .unwrap();
        state::ACCOUNTING
            .save(
                deps.as_mut().storage,
                &Accounting {
                    principal: Uint128::zero(),
                    fees: Uint128::zero(),
                    challenge: Uint128::zero(),
                    pool_yes: Uint128::zero(),
                    pool_no: Uint128::zero(),
                    total_yes: Uint128::zero(),
                    total_no: Uint128::zero(),
                    lp_supply: Uint128::zero(),
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
        let mut env = mock_env();
        env.contract.address = market;
        let error = execute_challenge_at(
            deps.as_mut(),
            env,
            mock_info("challenger", &[coin(config.challenge_bond.u128(), "ujuno")]),
            &config,
            now,
        )
        .unwrap_err();
        assert_eq!(error, ContractError::ArbitrationDeadlineOverflow);
        assert!(state::CHALLENGE
            .may_load(deps.as_ref().storage)
            .unwrap()
            .is_none());
        assert_eq!(
            state::ACCOUNTING
                .load(deps.as_ref().storage)
                .unwrap()
                .challenge,
            Uint128::zero()
        );
        assert!(
            !state::LIFECYCLE
                .load(deps.as_ref().storage)
                .unwrap()
                .challenge_used
        );
        assert_eq!(oracle_response.question.history_hash, [5; 32]);
    }
}

fn exact_ratio(ratio: math::QuoteRatio) -> ExactRatio {
    ExactRatio {
        numerator: Uint512::from(ratio.numerator),
        denominator: Uint512::from(ratio.denominator),
    }
}

fn impact(
    before: math::QuoteRatio,
    after: math::QuoteRatio,
) -> StdResult<(ExactRatio, ImpactDirection)> {
    let after_cross = Uint512::from(after.numerator)
        .checked_mul(Uint512::from(before.denominator))
        .map_err(|error| StdError::generic_err(error.to_string()))?;
    let before_cross = Uint512::from(before.numerator)
        .checked_mul(Uint512::from(after.denominator))
        .map_err(|error| StdError::generic_err(error.to_string()))?;
    let denominator = Uint512::from(after.denominator)
        .checked_mul(Uint512::from(before.denominator))
        .map_err(|error| StdError::generic_err(error.to_string()))?;
    let (numerator, direction) = match after_cross.cmp(&before_cross) {
        std::cmp::Ordering::Greater => (
            after_cross
                .checked_sub(before_cross)
                .map_err(|error| StdError::generic_err(error.to_string()))?,
            ImpactDirection::Up,
        ),
        std::cmp::Ordering::Less => (
            before_cross
                .checked_sub(after_cross)
                .map_err(|error| StdError::generic_err(error.to_string()))?,
            ImpactDirection::Down,
        ),
        std::cmp::Ordering::Equal => (Uint512::zero(), ImpactDirection::Flat),
    };
    Ok((
        ExactRatio {
            numerator,
            denominator,
        },
        direction,
    ))
}

fn quote_response_buy(env: &Env, outcome: Outcome, quote: BuyQuote) -> StdResult<QuoteResponse> {
    let (price_impact, impact_direction) = impact(quote.marginal_before, quote.marginal_after)?;
    Ok(QuoteResponse {
        height: env.block.height,
        block_time: env.block.time.seconds(),
        outcome,
        gross: quote.gross_collateral,
        net: quote.net_collateral,
        fee: quote.fee,
        input: quote.gross_collateral,
        output: quote.outcome_out,
        reserve_yes_before: quote.reserves_before.yes,
        reserve_no_before: quote.reserves_before.no,
        reserve_yes_after: quote.reserves_after.yes,
        reserve_no_after: quote.reserves_after.no,
        average_price: exact_ratio(quote.average_execution),
        marginal_before: exact_ratio(quote.marginal_before),
        marginal_after: exact_ratio(quote.marginal_after),
        fee_rate: ExactRatio {
            numerator: Uint512::from(quote.fee),
            denominator: Uint512::from(quote.gross_collateral),
        },
        price_impact,
        impact_direction,
        min_out: Some(quote.outcome_out),
        max_in: None,
    })
}

fn quote_response_sell(env: &Env, outcome: Outcome, quote: SellQuote) -> StdResult<QuoteResponse> {
    let (price_impact, impact_direction) = impact(quote.marginal_before, quote.marginal_after)?;
    Ok(QuoteResponse {
        height: env.block.height,
        block_time: env.block.time.seconds(),
        outcome,
        gross: quote.gross_collateral,
        net: quote.net_collateral,
        fee: quote.fee,
        input: quote.outcome_in,
        output: quote.net_collateral,
        reserve_yes_before: quote.reserves_before.yes,
        reserve_no_before: quote.reserves_before.no,
        reserve_yes_after: quote.reserves_after.yes,
        reserve_no_after: quote.reserves_after.no,
        average_price: exact_ratio(quote.average_execution),
        marginal_before: exact_ratio(quote.marginal_before),
        marginal_after: exact_ratio(quote.marginal_after),
        fee_rate: ExactRatio {
            numerator: Uint512::from(quote.fee),
            denominator: Uint512::from(quote.gross_collateral),
        },
        price_impact,
        impact_direction,
        min_out: None,
        max_in: Some(quote.outcome_in),
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    let config = state::CONFIG.load(deps.storage)?;
    match msg {
        QueryMsg::Config {} => to_json_binary(&ConfigResponse {
            protocol_version: config.protocol_version,
            factory: config.factory.to_string(),
            creator: config.creator.to_string(),
            initial_lp: config.initial_lp.to_string(),
            oracle: config.oracle.to_string(),
            verdict_authority: config.verdict_authority.to_string(),
            tier: config.tier,
            collateral_denom: config.collateral_denom,
            close_ts: config.close_ts,
            opening_ts: config.opening_ts,
            initial_liquidity: config.initial_liquidity,
            oracle_bounty: config.oracle_bounty,
            oracle_initial_bond: config.oracle_initial_bond,
            answer_timeout_secs: config.answer_timeout_secs,
            arbitration_timeout_secs: config.arbitration_timeout_secs,
            fee_bps: config.fee_bps,
            min_trade: config.min_trade,
            max_trade_bps: config.max_trade_bps,
            max_position_per_side: config.max_position_per_side,
            collateral_cap: config.collateral_cap,
            challenge_bond: config.challenge_bond,
        }),
        QueryMsg::Identity {} => to_json_binary(&IdentityResponse {
            protocol_version: ProtocolVersion::V1,
            factory: config.factory.to_string(),
            market: env.contract.address.to_string(),
            nonce: config.nonce,
            question_id: state::QUESTION_ID.may_load(deps.storage)?,
        }),
        QueryMsg::State {} => {
            let lifecycle = state::LIFECYCLE.load(deps.storage)?;
            let challenge = state::CHALLENGE.may_load(deps.storage)?;
            to_json_binary(&StateResponse {
                status: guards::derived_lifecycle(
                    env.block.time.seconds(),
                    &config,
                    &lifecycle,
                    challenge.as_ref(),
                ),
                activated: lifecycle.activated,
                challenge_used: lifecycle.challenge_used,
            })
        }
        QueryMsg::QuoteBuy { outcome, gross } => {
            let lifecycle = state::LIFECYCLE.load(deps.storage)?;
            guards::trading(&env, &config, &lifecycle)
                .map_err(|error| StdError::generic_err(error.to_string()))?;
            let accounting = state::ACCOUNTING.load(deps.storage)?;
            let quote = buy_quote(&accounting, &config, outcome.clone(), gross)
                .map_err(|error| StdError::generic_err(error.to_string()))?;
            to_json_binary(&quote_response_buy(&env, outcome, quote)?)
        }
        QueryMsg::QuoteSell {
            outcome,
            return_amount,
        } => {
            let lifecycle = state::LIFECYCLE.load(deps.storage)?;
            guards::trading(&env, &config, &lifecycle)
                .map_err(|error| StdError::generic_err(error.to_string()))?;
            let accounting = state::ACCOUNTING.load(deps.storage)?;
            let quote = sell_quote(&accounting, &config, outcome.clone(), return_amount)
                .map_err(|error| StdError::generic_err(error.to_string()))?;
            to_json_binary(&quote_response_sell(&env, outcome, quote)?)
        }
        QueryMsg::Position { address } => {
            let address = deps.api.addr_validate(&address)?;
            let position = state::load_position(deps.storage, &address)?;
            to_json_binary(&PositionResponse {
                address: address.to_string(),
                yes: position.yes,
                no: position.no,
            })
        }
        QueryMsg::Resolution {} => {
            let lifecycle = state::LIFECYCLE.load(deps.storage)?;
            let accounting = state::ACCOUNTING.load(deps.storage)?;
            let answer = lifecycle.resolution_answer;
            to_json_binary(&crate::msg::ResolutionResponse {
                answer_hex: answer.as_ref().map(|value| hex::encode(value.as_slice())),
                answer_base64: answer.as_ref().map(Binary::to_base64),
                answer,
                payout: lifecycle.payout,
                height: lifecycle.resolution_height,
                time: lifecycle.resolution_time,
                principal_at_resolution: accounting.principal_at_resolution,
                terminal_liability_twice: accounting.terminal_liability_twice,
                pool_yes_at_resolution: accounting.pool_yes_at_resolution,
                pool_no_at_resolution: accounting.pool_no_at_resolution,
                total_yes_at_resolution: accounting.total_yes_at_resolution,
                total_no_at_resolution: accounting.total_no_at_resolution,
            })
        }
        QueryMsg::Question {} => to_json_binary(&QuestionResponse {
            text: config.question,
            hash_hex: hex::encode(config.question_hash.as_slice()),
            hash_base64: config.question_hash.to_base64(),
            hash: config.question_hash,
            nonce: config.nonce,
            question_id: state::QUESTION_ID.may_load(deps.storage)?,
            oracle: config.oracle.to_string(),
            opening_ts: config.opening_ts,
            close_ts: config.close_ts,
            yes_answer_hex: hex::encode(config.yes_answer.as_slice()),
            yes_answer_base64: config.yes_answer.to_base64(),
            no_answer_hex: hex::encode(config.no_answer.as_slice()),
            no_answer_base64: config.no_answer.to_base64(),
            invalid_answer_hex: hex::encode(config.invalid_answer.as_slice()),
            invalid_answer_base64: config.invalid_answer.to_base64(),
            unresolved_answer_hex: hex::encode(config.unresolved_answer.as_slice()),
            unresolved_answer_base64: config.unresolved_answer.to_base64(),
        }),
        QueryMsg::Accounting {} => {
            let accounting = state::ACCOUNTING.load(deps.storage)?;
            to_json_binary(&crate::msg::AccountingResponse {
                principal: accounting.principal,
                fees: accounting.fees,
                challenge: accounting.challenge,
                terminal_liability_twice: accounting.terminal_liability_twice,
                total_yes: accounting.total_yes,
                total_no: accounting.total_no,
                lp_supply: accounting.lp_supply,
                lp_burned: accounting.lp_burned,
                lp_paid: accounting.lp_paid,
                neutral_half_dust: accounting.neutral_half_dust,
                lp_accrual: accounting.lp_accrual,
                principal_at_resolution: accounting.principal_at_resolution,
                fees_at_resolution: accounting.fees_at_resolution,
                pool_yes_at_resolution: accounting.pool_yes_at_resolution,
                pool_no_at_resolution: accounting.pool_no_at_resolution,
                total_yes_at_resolution: accounting.total_yes_at_resolution,
                total_no_at_resolution: accounting.total_no_at_resolution,
            })
        }
        QueryMsg::Pool {} => {
            let accounting = state::ACCOUNTING.load(deps.storage)?;
            to_json_binary(&crate::msg::PoolResponse {
                yes: accounting.pool_yes,
                no: accounting.pool_no,
            })
        }
        QueryMsg::LpPosition {} => {
            let accounting = state::ACCOUNTING.load(deps.storage)?;
            to_json_binary(&crate::msg::LpPositionResponse {
                owner: config.initial_lp.to_string(),
                supply: accounting.lp_supply,
                burned: accounting.lp_burned,
                paid: accounting.lp_paid,
                later_accrual: accounting.lp_accrual,
            })
        }
        QueryMsg::Challenge {} => {
            let challenge = state::CHALLENGE.may_load(deps.storage)?;
            let challenge_bond = state::ACCOUNTING.load(deps.storage)?.challenge;
            to_json_binary(&crate::msg::ChallengeResponse {
                challenger: challenge.as_ref().map(|value| value.challenger.to_string()),
                answer: challenge.as_ref().map(|value| value.answer.clone()),
                answer_hex: challenge
                    .as_ref()
                    .map(|value| hex::encode(value.answer.as_slice())),
                answer_base64: challenge.as_ref().map(|value| value.answer.to_base64()),
                oracle_bond: challenge.as_ref().map(|value| value.oracle_bond),
                challenge_bond,
                started_at: challenge.as_ref().map(|value| value.started_at),
                deadline: challenge.as_ref().map(|value| value.deadline),
                oracle_snapshot: challenge.map(|value| value.oracle_snapshot),
            })
        }
        QueryMsg::Solvency {} => {
            let accounting = state::ACCOUNTING.load(deps.storage)?;
            let bank_balance = deps
                .querier
                .query_balance(env.contract.address.clone(), &config.collateral_denom)?
                .amount;
            // Before resolution principal is the backing liability. Afterwards,
            // redemptions reduce the terminal numerator while `principal` remains
            // the immutable resolution snapshot. Round the remaining half-ujuno
            // numerator up so every outstanding half remains covered.
            let principal_or_terminal_liability = match accounting.terminal_liability_twice {
                Some(value) => value
                    .checked_add(Uint128::one())?
                    .checked_div(Uint128::new(2))?,
                None => accounting.principal,
            };
            let fee_liability = accounting.fees;
            let challenge_liability = accounting.challenge;
            let lp_whole_coin_accrual = accounting.lp_accrual;
            let accounted_liability = principal_or_terminal_liability
                .checked_add(fee_liability)?
                .checked_add(challenge_liability)?
                .checked_add(lp_whole_coin_accrual)?;
            let (forced_excess, shortfall) = if bank_balance >= accounted_liability {
                (
                    bank_balance.checked_sub(accounted_liability)?,
                    Uint128::zero(),
                )
            } else {
                (
                    Uint128::zero(),
                    accounted_liability.checked_sub(bank_balance)?,
                )
            };
            to_json_binary(&crate::msg::SolvencyResponse {
                height: env.block.height,
                block_time: env.block.time.seconds(),
                bank_balance,
                principal_or_terminal_liability,
                fee_liability,
                challenge_liability,
                lp_whole_coin_accrual,
                accounted_liability,
                forced_excess,
                shortfall,
                solvent: shortfall.is_zero(),
            })
        }
    }
}
