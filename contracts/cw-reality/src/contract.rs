//! Contract entry points.
//!
//! State-mutating handlers live in this file as stubs for stage 2's first
//! slice; subsequent slices fill in `execute_ask_question`,
//! `execute_submit_answer`, etc., per the state machine in `state.rs`.

use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::escalation::{MAX_ANSWER_TIMEOUT_SECS, MIN_ANSWER_TIMEOUT_SECS_FLOOR};
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{Config, CONFIG};

const CONTRACT_NAME: &str = "crates.io:cw-reality";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if msg.min_initial_bond_floor.is_zero() {
        return Err(ContractError::ZeroMinInitialBondFloor {});
    }
    if msg.min_answer_timeout_secs < MIN_ANSWER_TIMEOUT_SECS_FLOOR {
        return Err(ContractError::MinAnswerTimeoutTooLow {});
    }
    if msg.min_answer_timeout_secs > MAX_ANSWER_TIMEOUT_SECS {
        return Err(ContractError::AnswerTimeoutTooHigh {});
    }

    let admin = msg.admin.map(|a| deps.api.addr_validate(&a)).transpose()?;

    CONFIG.save(
        deps.storage,
        &Config {
            admin,
            min_initial_bond_floor: msg.min_initial_bond_floor,
            min_answer_timeout_secs: msg.min_answer_timeout_secs,
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute(
            "min_initial_bond_floor",
            msg.min_initial_bond_floor.to_string(),
        )
        .add_attribute(
            "min_answer_timeout_secs",
            msg.min_answer_timeout_secs.to_string(),
        ))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AskQuestion {
            text,
            answer_type,
            bond_denom,
            initial_bond,
            answer_timeout_secs,
            arbitrator,
            arbitration_timeout_secs,
            answer_schema,
            opening_ts,
            nonce,
        } => crate::execute::ask::execute_ask_question(
            deps,
            env,
            info,
            text,
            answer_type,
            bond_denom,
            initial_bond,
            answer_timeout_secs,
            arbitrator,
            arbitration_timeout_secs,
            answer_schema,
            opening_ts,
            nonce,
        ),
        ExecuteMsg::FundBounty { question_id } => {
            crate::execute::fund_bounty::execute_fund_bounty(deps, env, info, question_id)
        }
        ExecuteMsg::SubmitAnswer {
            question_id,
            answer,
            current_bond_seen,
        } => crate::execute::answer::execute_submit_answer(
            deps,
            env,
            info,
            question_id,
            answer,
            current_bond_seen,
        ),
        ExecuteMsg::DisputeAnswer {
            question_id,
            new_answer,
            current_bond_seen,
        } => crate::execute::answer::execute_dispute_answer(
            deps,
            env,
            info,
            question_id,
            new_answer,
            current_bond_seen,
        ),
        ExecuteMsg::RequestArbitration {
            question_id,
            current_bond_seen,
        } => crate::execute::arbitration::execute_request_arbitration(
            deps,
            env,
            info,
            question_id,
            current_bond_seen,
        ),
        ExecuteMsg::CancelArbitration { question_id } => {
            crate::execute::arbitration::execute_cancel_arbitration(deps, env, info, question_id)
        }
        ExecuteMsg::SubmitArbitration {
            question_id,
            winning_answer,
            payee,
        } => crate::execute::arbitration::execute_submit_arbitration(
            deps,
            env,
            info,
            question_id,
            winning_answer,
            payee,
        ),
        ExecuteMsg::Claim {
            question_id,
            history_entries,
        } => crate::execute::claim::execute_claim(deps, env, info, question_id, history_entries),
        ExecuteMsg::Withdraw { denom } => {
            crate::execute::withdraw::execute_withdraw(deps, env, info, denom)
        }
        ExecuteMsg::Receive(wrapper) => {
            crate::execute::receive::execute_receive(deps, env, info, wrapper)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::Question { question_id } => {
            to_json_binary(&crate::query::query_question(deps, env, question_id)?)
        }
        QueryMsg::FinalAnswer { question_id } => {
            to_json_binary(&crate::query::query_final_answer(deps, env, question_id)?)
        }
        QueryMsg::FinalAnswerIfMatches {
            question_id,
            min_bond,
            min_timeout_secs,
            required_arbitrator,
            required_denom,
        } => to_json_binary(&crate::query::query_final_answer_if_matches(
            deps,
            env,
            question_id,
            min_bond,
            min_timeout_secs,
            required_arbitrator,
            required_denom,
        )?),
        QueryMsg::List {
            start_after,
            limit,
            status,
        } => to_json_binary(&crate::query::query_list(
            deps,
            env,
            start_after,
            limit,
            status,
        )?),
        QueryMsg::Balance { address, denom } => {
            to_json_binary(&crate::query::query_balance(deps, address, denom)?)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new().add_attribute("action", "migrate"))
}
