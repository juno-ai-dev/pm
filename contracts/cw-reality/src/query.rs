//! Read-side handlers.
//!
//! `FinalAnswerIfMatches` is Reality.eth's reader-side trust knob
//! (`getFinalAnswerIfMatches`): a downstream contract consuming the answer
//! can require minimum bond, minimum timeout, a specific arbitrator, and a
//! specific denom. Reverts unless every constraint passes. This is the
//! single most important defense the consumer has against accepting answers
//! that don't meet its safety threshold.

use cosmwasm_std::{Binary, Deps, Order, StdError, StdResult};
use cw_storage_plus::Bound;

use crate::msg::{BalanceResponse, FinalAnswerResponse, QuestionResponse, QuestionsListResponse};
use crate::state::{State, BALANCES, QUESTIONS};

pub fn query_question(
    deps: Deps,
    env: cosmwasm_std::Env,
    question_id: Binary,
) -> StdResult<QuestionResponse> {
    let qid: [u8; 32] = question_id
        .as_slice()
        .try_into()
        .map_err(|_| StdError::generic_err("question_id must be 32 bytes"))?;
    let question = QUESTIONS
        .may_load(deps.storage, &qid)?
        .ok_or_else(|| StdError::not_found("Question"))?;
    let state = question.state_at(env.block.time.seconds());
    Ok(QuestionResponse {
        question_id,
        question,
        state,
    })
}

pub fn query_final_answer(
    deps: Deps,
    env: cosmwasm_std::Env,
    question_id: Binary,
) -> StdResult<FinalAnswerResponse> {
    let qid: [u8; 32] = question_id
        .as_slice()
        .try_into()
        .map_err(|_| StdError::generic_err("question_id must be 32 bytes"))?;
    let question = QUESTIONS
        .may_load(deps.storage, &qid)?
        .ok_or_else(|| StdError::not_found("Question"))?;
    let now = env.block.time.seconds();
    match question.state_at(now) {
        State::Finalized | State::Claimed => {}
        _ => return Err(StdError::generic_err("question is not finalized")),
    }
    let final_answer = question
        .best_answer
        .clone()
        .ok_or_else(|| StdError::generic_err("question has no best_answer (unreachable)"))?;
    Ok(FinalAnswerResponse {
        question_id,
        final_answer,
        final_bond: question.current_bond,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn query_final_answer_if_matches(
    deps: Deps,
    env: cosmwasm_std::Env,
    question_id: Binary,
    min_bond: Option<cosmwasm_std::Uint128>,
    min_timeout_secs: Option<u32>,
    required_arbitrator: Option<String>,
    required_denom: Option<String>,
) -> StdResult<FinalAnswerResponse> {
    let qid: [u8; 32] = question_id
        .as_slice()
        .try_into()
        .map_err(|_| StdError::generic_err("question_id must be 32 bytes"))?;
    let question = QUESTIONS
        .may_load(deps.storage, &qid)?
        .ok_or_else(|| StdError::not_found("Question"))?;
    let now = env.block.time.seconds();
    match question.state_at(now) {
        State::Finalized | State::Claimed => {}
        _ => return Err(StdError::generic_err("question is not finalized")),
    }

    if let Some(min) = min_bond {
        if question.current_bond < min {
            return Err(StdError::generic_err(
                "final_bond below caller's min_bond — guarantees not met",
            ));
        }
    }
    if let Some(min) = min_timeout_secs {
        if question.answer_timeout_secs < min {
            return Err(StdError::generic_err(
                "answer_timeout_secs below caller's min — guarantees not met",
            ));
        }
    }
    if let Some(required) = required_arbitrator {
        let required_addr = deps.api.addr_validate(&required)?;
        match &question.arbitrator {
            Some(actual) if *actual == required_addr => {}
            _ => {
                return Err(StdError::generic_err(
                    "arbitrator mismatch — guarantees not met",
                ))
            }
        }
    }
    if let Some(required) = required_denom {
        if question.bond_denom != required {
            return Err(StdError::generic_err(
                "bond_denom mismatch — guarantees not met",
            ));
        }
    }

    let final_answer = question
        .best_answer
        .clone()
        .ok_or_else(|| StdError::generic_err("question has no best_answer (unreachable)"))?;
    Ok(FinalAnswerResponse {
        question_id,
        final_answer,
        final_bond: question.current_bond,
    })
}

const DEFAULT_LIMIT: u32 = 30;
const MAX_LIMIT: u32 = 100;

pub fn query_list(
    deps: Deps,
    env: cosmwasm_std::Env,
    start_after: Option<Binary>,
    limit: Option<u32>,
    status: Option<State>,
) -> StdResult<QuestionsListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let now = env.block.time.seconds();
    let start: Option<Vec<u8>> = start_after.map(|b| b.to_vec());
    let bound = start.as_deref().map(Bound::exclusive);
    let mut out: Vec<QuestionResponse> = Vec::new();
    let iter = QUESTIONS.range(deps.storage, bound, None, Order::Ascending);
    for kv in iter {
        let (qid_bytes, question) = kv?;
        let state = question.state_at(now);
        if let Some(want) = &status {
            if &state != want {
                continue;
            }
        }
        out.push(QuestionResponse {
            question_id: Binary::from(qid_bytes),
            question,
            state,
        });
        if out.len() >= limit {
            break;
        }
    }
    Ok(QuestionsListResponse { questions: out })
}

pub fn query_balance(deps: Deps, address: String, denom: String) -> StdResult<BalanceResponse> {
    let addr = deps.api.addr_validate(&address)?;
    let amount = BALANCES
        .may_load(deps.storage, (&addr, &denom))?
        .unwrap_or_default();
    Ok(BalanceResponse {
        address,
        denom,
        amount,
    })
}
