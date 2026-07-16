use cosmwasm_std::{
    entry_point, to_json_binary, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env, Event,
    MessageInfo, Reply, ReplyOn, Response, StdError, StdResult, SubMsg, Uint128, Uint256, WasmMsg,
};

use crate::{
    error::ContractError,
    guards,
    math::{self, BuyQuote, Reserves, SellQuote},
    msg::{
        ConfigResponse, ExecuteMsg, IdentityResponse, InstantiateMsg, LifecycleStatus,
        PositionResponse, QueryMsg, QuestionResponse, QuoteResponse, StateResponse,
    },
    question,
    state::{self, Accounting, Config, Lifecycle, Position, ReplyInProgress},
};
use cw_reality::{
    msg::{
        ExecuteMsg as OracleExecuteMsg, FinalAnswerResponse as OracleFinalAnswerResponse,
        QueryMsg as OracleQueryMsg, QuestionResponse as OracleQuestionResponse,
    },
    state::{AnswerType, State as OracleState},
};
use pm_types::{Outcome, Payout, ProtocolVersion, UJUNO_DENOM};

pub const REPLY_ACTIVATION: u64 = 1;
pub const REPLY_CHALLENGE: u64 = 2;
pub const REPLY_GOVERNANCE_VERDICT: u64 = 3;
pub const REPLY_STALLED_CANCELLATION: u64 = 4;

#[entry_point]
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
    let governance = deps.api.addr_validate(&msg.governance)?;
    let (question_text, question_hash) = question::canonical_question(
        &msg.question,
        &env.contract.address,
        &oracle,
        &governance,
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
        governance,
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
    {
        return Err(ContractError::InvalidConfig(
            "invalid fee or trade bounds".into(),
        ));
    }
    Ok(())
}

#[entry_point]
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
            return execute_split(deps, env, info, &config, amount);
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
            return execute_buy(deps, env, info, &config, outcome, min_out, gross);
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
            return execute_sell(deps, env, info, &config, outcome, return_amount, max_in);
        }
        ExecuteMsg::Merge { amount } => {
            guards::no_funds(&info.funds)?;
            if !lifecycle.activated {
                return Err(ContractError::NotActivated);
            }
            guards::unresolved(&lifecycle)?;
            return execute_merge(deps, env, info, &config, amount);
        }
        ExecuteMsg::Challenge {} => {
            guards::exact_funds(&info.funds, UJUNO_DENOM, config.challenge_bond)?;
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
        }
        ExecuteMsg::GovernanceVerdict { .. } => {
            guards::no_funds(&info.funds)?;
            guards::governance_verdict(&env, &info.sender, &config, challenge.as_ref())?;
        }
        ExecuteMsg::FinalizeStalledChallenge {} => {
            guards::no_funds(&info.funds)?;
            challenge
                .as_ref()
                .ok_or(ContractError::NoPendingChallenge)?;
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
            return execute_resolve(deps, env, &config);
        }
        ExecuteMsg::RedeemPositions { .. }
        | ExecuteMsg::RedeemLp { .. }
        | ExecuteMsg::ClaimLpAccrual {} => {
            guards::no_funds(&info.funds)?;
            if lifecycle.payout.is_none() {
                return Err(ContractError::NotResolved);
            }
        }
    }
    Err(ContractError::NotImplemented)
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
        .add_attribute("action", action)
        .add_attribute("caller", caller.to_string())
        .add_attribute("amount", amount.to_string())
        .add_attribute("principal", accounting.principal.to_string())
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
    state::POSITIONS.update(
        deps.storage,
        &info.sender,
        |position| -> Result<_, ContractError> {
            let mut position = position.unwrap_or_default();
            position.yes = position.yes.checked_add(amount)?;
            position.no = position.no.checked_add(amount)?;
            Ok(position)
        },
    )?;
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
        return Err(ContractError::Math(
            "configured reserve-ratio limit exceeded".into(),
        ));
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

#[entry_point]
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
        return Err(ContractError::NotImplemented);
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
            .add_attribute("lp", config.initial_lp.to_string())
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
    use super::{verify_oracle_question, verify_resolution};
    use crate::{question, state::Config};
    use cosmwasm_std::{Addr, Binary, Uint128};
    use cw_reality::{
        filter::AnswerSchemaFilter,
        msg::{FinalAnswerResponse, QuestionResponse},
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
            governance: Addr::unchecked("governance"),
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
}

fn ratio_bps(ratio: math::QuoteRatio) -> StdResult<Uint128> {
    let value = ratio
        .numerator
        .checked_mul(Uint256::from(math::FEE_SCALE))
        .map_err(|error| StdError::generic_err(error.to_string()))?
        .checked_div(ratio.denominator)
        .map_err(|error| StdError::generic_err(error.to_string()))?;
    Uint128::try_from(value).map_err(|error| StdError::generic_err(error.to_string()))
}

fn quote_response_buy(env: &Env, outcome: Outcome, quote: BuyQuote) -> StdResult<QuoteResponse> {
    let before = ratio_bps(quote.marginal_before)?;
    let after = ratio_bps(quote.marginal_after)?;
    Ok(QuoteResponse {
        height: env.block.height,
        time: env.block.time.seconds(),
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
        average_price_bps: ratio_bps(quote.average_execution)?,
        marginal_before_bps: before,
        marginal_after_bps: after,
        price_impact_bps: after.abs_diff(before),
    })
}

fn quote_response_sell(env: &Env, outcome: Outcome, quote: SellQuote) -> StdResult<QuoteResponse> {
    let before = ratio_bps(quote.marginal_before)?;
    let after = ratio_bps(quote.marginal_after)?;
    Ok(QuoteResponse {
        height: env.block.height,
        time: env.block.time.seconds(),
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
        average_price_bps: ratio_bps(quote.average_execution)?,
        marginal_before_bps: before,
        marginal_after_bps: after,
        price_impact_bps: after.abs_diff(before),
    })
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    let config = state::CONFIG.load(deps.storage)?;
    match msg {
        QueryMsg::Config {} => to_json_binary(&ConfigResponse {
            protocol_version: config.protocol_version,
            factory: config.factory.into(),
            creator: config.creator.to_string(),
            initial_lp: config.initial_lp.into(),
            oracle: config.oracle.into(),
            governance: config.governance.into(),
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
            collateral_cap: config.collateral_cap,
            challenge_bond: config.challenge_bond,
        }),
        QueryMsg::Identity {} => to_json_binary(&IdentityResponse {
            protocol_version: ProtocolVersion::V1,
            factory: config.factory.into(),
            market: env.contract.address.into(),
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
            let accounting = state::ACCOUNTING.load(deps.storage)?;
            let quote = buy_quote(&accounting, &config, outcome.clone(), gross)
                .map_err(|error| StdError::generic_err(error.to_string()))?;
            to_json_binary(&quote_response_buy(&env, outcome, quote)?)
        }
        QueryMsg::QuoteSell {
            outcome,
            return_amount,
        } => {
            let accounting = state::ACCOUNTING.load(deps.storage)?;
            let quote = sell_quote(&accounting, &config, outcome.clone(), return_amount)
                .map_err(|error| StdError::generic_err(error.to_string()))?;
            to_json_binary(&quote_response_sell(&env, outcome, quote)?)
        }
        QueryMsg::Position { address } => {
            let address = deps.api.addr_validate(&address)?;
            let position = state::load_position(deps.storage, &address)?;
            to_json_binary(&PositionResponse {
                address: address.into(),
                yes: position.yes,
                no: position.no,
            })
        }
        QueryMsg::Resolution {} => {
            let lifecycle = state::LIFECYCLE.load(deps.storage)?;
            let accounting = state::ACCOUNTING.load(deps.storage)?;
            to_json_binary(&crate::msg::ResolutionResponse {
                answer: lifecycle.resolution_answer,
                payout: lifecycle.payout,
                height: lifecycle.resolution_height,
                time: lifecycle.resolution_time,
                principal_at_resolution: accounting.principal_at_resolution,
            })
        }
        QueryMsg::Question {} => to_json_binary(&QuestionResponse {
            text: config.question,
            hash: config.question_hash,
            nonce: config.nonce,
            question_id: state::QUESTION_ID.may_load(deps.storage)?,
            oracle: config.oracle.into(),
            opening_ts: config.opening_ts,
            close_ts: config.close_ts,
        }),
        _ => Err(StdError::not_found(
            "query state not initialized by issue #8",
        )),
    }
}
