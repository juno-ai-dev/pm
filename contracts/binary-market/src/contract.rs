use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdError, StdResult, Uint128,
};

use crate::{
    error::ContractError,
    guards,
    msg::{
        ConfigResponse, ExecuteMsg, IdentityResponse, InstantiateMsg, LifecycleStatus,
        PositionResponse, QueryMsg, QuestionResponse, StateResponse,
    },
    state::{self, Accounting, Config, Lifecycle, ReplyInProgress},
};
use pm_types::{ProtocolVersion, UJUNO_DENOM};

pub const REPLY_ACTIVATION: u64 = 1;
pub const REPLY_CHALLENGE: u64 = 2;
pub const REPLY_GOVERNANCE_VERDICT: u64 = 3;
pub const REPLY_STALLED_CANCELLATION: u64 = 4;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    validate_instantiate(&msg)?;
    let required = msg.initial_liquidity.checked_add(msg.oracle_bounty)?;
    guards::exact_funds(&info.funds, UJUNO_DENOM, required)?;
    let factory = deps.api.addr_validate(&msg.factory)?;
    // Market instances may only be created by their immutable factory.
    guards::sender(&info.sender, &factory)?;
    let creator = deps.api.addr_validate(&msg.creator)?;
    let config = Config {
        protocol_version: ProtocolVersion::V1,
        factory,
        initial_lp: creator.clone(),
        creator,
        oracle: deps.api.addr_validate(&msg.oracle)?,
        governance: deps.api.addr_validate(&msg.governance)?,
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
        yes_answer: msg.yes_answer,
        no_answer: msg.no_answer,
        invalid_answer: msg.invalid_answer,
        unresolved_answer: msg.unresolved_answer,
        question: msg.question,
        question_hash: msg.question_hash,
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
    // No financial position or supply exists before the oracle activation reply.
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
        },
    )?;
    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("status", "initializing"))
}

fn validate_instantiate(msg: &InstantiateMsg) -> Result<(), ContractError> {
    if msg.question.is_empty() || msg.question_hash.len() != 32 {
        return Err(ContractError::InvalidConfig(
            "question and 32-byte hash are required".into(),
        ));
    }
    if msg.close_ts == 0 || msg.opening_ts < msg.close_ts {
        return Err(ContractError::InvalidConfig(
            "opening_ts must be at or after close_ts".into(),
        ));
    }
    if msg.initial_liquidity.is_zero() || msg.collateral_cap < msg.initial_liquidity {
        return Err(ContractError::InvalidConfig(
            "liquidity must be positive and within cap".into(),
        ));
    }
    if msg.oracle_bounty.is_zero()
        || msg.oracle_initial_bond.is_zero()
        || msg.answer_timeout_secs == 0
        || msg.arbitration_timeout_secs == 0
        || msg.challenge_bond.is_zero()
    {
        return Err(ContractError::InvalidConfig(
            "oracle and challenge parameters must be nonzero".into(),
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
    let answers = [
        &msg.yes_answer,
        &msg.no_answer,
        &msg.invalid_answer,
        &msg.unresolved_answer,
    ];
    if answers.iter().any(|a| a.len() != 32) {
        return Err(ContractError::InvalidConfig(
            "all result values must be 32 bytes".into(),
        ));
    }
    for (index, answer) in answers.iter().enumerate() {
        if answers.iter().skip(index + 1).any(|other| other == answer) {
            return Err(ContractError::InvalidConfig(
                "result values must be distinct".into(),
            ));
        }
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
        }
        ExecuteMsg::Buy { deadline, .. } => {
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
        }
        ExecuteMsg::Sell { deadline, .. } => {
            guards::no_funds(&info.funds)?;
            guards::trading(&env, &config, &lifecycle)?;
            guards::user_deadline(&env, deadline)?;
        }
        ExecuteMsg::Merge { .. } => {
            guards::no_funds(&info.funds)?;
            guards::unresolved(&lifecycle)?;
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
            guards::unresolved(&lifecycle)?;
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

#[entry_point]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> Result<Response, ContractError> {
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
    Err(ContractError::NotImplemented)
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
        QueryMsg::Position { address } => {
            let address = deps.api.addr_validate(&address)?;
            let position = state::load_position(deps.storage, &address)?;
            to_json_binary(&PositionResponse {
                address: address.into(),
                yes: position.yes,
                no: position.no,
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
