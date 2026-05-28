//! `ExecuteMsg::Receive` — cw20 entry point.
//!
//! When a cw20 token is the question's bond denom, bonds arrive through the
//! `Cw20ReceiveMsg` hook. The receiving contract (`info.sender`) is the cw20
//! contract address — we treat that address string as the bond denom for
//! storage and accounting purposes (mirroring how `cw20` deposit modules
//! handle multi-denom in dao-pre-propose-base).
//!
//! The bonder identity is `cw20_msg.sender`, not `info.sender` — same
//! pattern as `dao-pre-propose-base/src/execute.rs:131-190`.

use cosmwasm_std::{from_json, Coin, DepsMut, Env, MessageInfo, Response, Uint128};
use cw20::Cw20ReceiveMsg;

use crate::error::ContractError;
use crate::msg::ReceiveAction;

pub fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    // info.sender is the cw20 token contract. Treat it as the bond denom.
    let cw20_denom = info.sender.to_string();
    let bonder = deps.api.addr_validate(&wrapper.sender)?;
    let amount: Uint128 = wrapper.amount;
    if amount.is_zero() {
        return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
            "Receive: zero-amount cw20 transfer",
        )));
    }

    let action: ReceiveAction = from_json(&wrapper.msg)?;

    // Synthesize a MessageInfo as if the bonder had called the variant
    // directly with the cw20 contract address as the bond denom.
    let synthesised_info = MessageInfo {
        sender: bonder,
        funds: vec![Coin {
            denom: cw20_denom.clone(),
            amount,
        }],
    };

    match action {
        ReceiveAction::AskQuestion {
            text,
            answer_type,
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
            synthesised_info,
            text,
            answer_type,
            cw20_denom,
            initial_bond,
            answer_timeout_secs,
            arbitrator,
            arbitration_timeout_secs,
            answer_schema,
            opening_ts,
            nonce,
        ),
        ReceiveAction::FundBounty { question_id } => {
            crate::execute::fund_bounty::execute_fund_bounty(
                deps,
                env,
                synthesised_info,
                question_id,
            )
        }
        ReceiveAction::SubmitAnswer {
            question_id,
            answer,
            current_bond_seen,
        } => crate::execute::answer::execute_submit_answer(
            deps,
            env,
            synthesised_info,
            question_id,
            answer,
            current_bond_seen,
        ),
        ReceiveAction::DisputeAnswer {
            question_id,
            new_answer,
            current_bond_seen,
        } => crate::execute::answer::execute_dispute_answer(
            deps,
            env,
            synthesised_info,
            question_id,
            new_answer,
            current_bond_seen,
        ),
    }
}
