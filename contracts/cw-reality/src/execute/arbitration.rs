//! `RequestArbitration` / `CancelArbitration` / `SubmitArbitration` handlers.
//!
//! Reality.eth precedent in `RealityETH-3.0.sol`:
//! - `notifyOfArbitrationRequest`: `onlyArbitrator + stateOpen + finalize_ts > 0`
//!   (audit issue #2 fix — port the answer-required gate literally).
//! - `cancelArbitration`: `onlyArbitrator + statePendingArbitration`; re-extends
//!   `finalize_ts = now + timeout` (NOT restore).
//! - `submitAnswerByArbitrator`: `onlyArbitrator + statePendingArbitration`;
//!   sets `finalize_ts = now`; appends history entry with `bond = 0`.
//!
//! **Deliberate deviation from Reality.eth:** after the
//! `arbitration_deadline` elapses, anyone (not just the arbitrator) can call
//! `CancelArbitration` to unfreeze the question. This protects against a
//! stalled arbitrator address with no on-chain incentive to act — a real risk
//! under the `Option<Addr>` design (lessons §7.1). Reality.eth assumes the
//! arbitrator contract is economically bonded by some external system
//! (Kleros); we cannot assume the same.

use cosmwasm_std::{Addr, Binary, DepsMut, Env, Event, MessageInfo, Response, Uint128};

use crate::error::ContractError;
use crate::hash::next_history_hash;
use crate::state::{State, QUESTIONS, UNRESOLVED_ANSWER_BYTES};

pub fn execute_request_arbitration(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    question_id: Binary,
    current_bond_seen: Option<Uint128>,
) -> Result<Response, ContractError> {
    let qid: [u8; 32] =
        question_id
            .as_slice()
            .try_into()
            .map_err(|_| ContractError::QuestionNotFound {
                id: question_id.to_base64(),
            })?;
    let mut question =
        QUESTIONS
            .may_load(deps.storage, &qid)?
            .ok_or_else(|| ContractError::QuestionNotFound {
                id: hex::encode(qid),
            })?;

    let arbitrator = question
        .arbitrator
        .clone()
        .ok_or(ContractError::NoArbitrator {})?;
    if info.sender != arbitrator {
        return Err(ContractError::NotArbitrator {});
    }

    let now = env.block.time.seconds();
    if question.state_at(now) != State::OpenAnswered {
        return Err(ContractError::InvalidState {
            expected: "OpenAnswered".to_string(),
            actual: format!("{:?}", question.state_at(now)),
        });
    }

    // Audit issue #2 fix — requires at least one prior answer. Belt-and-braces
    // with the state check (OpenAnswered already implies finalize_ts is set).
    if question.finalize_ts.is_none() {
        return Err(ContractError::ArbitrationNoAnswer {});
    }

    // Front-run guard.
    if let Some(expected) = current_bond_seen {
        if question.current_bond > expected {
            return Err(ContractError::BondExceedsExpected {
                actual: question.current_bond,
                expected,
            });
        }
    }

    question.is_pending_arbitration = true;
    let deadline = now.saturating_add(u64::from(question.arbitration_timeout_secs));
    question.arbitration_deadline = Some(deadline);

    QUESTIONS.save(deps.storage, &qid, &question)?;

    Ok(Response::new()
        .add_attribute("action", "request_arbitration")
        .add_attribute("question_id", hex::encode(qid))
        .add_attribute("arbitrator", arbitrator.as_str())
        .add_attribute("deadline", deadline.to_string())
        .add_event(
            Event::new("cw_reality/arbitration_requested")
                .add_attribute("question_id", Binary::from(qid).to_base64())
                .add_attribute("arbitrator", arbitrator.as_str())
                .add_attribute("deadline", deadline.to_string()),
        ))
}

pub fn execute_cancel_arbitration(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    question_id: Binary,
) -> Result<Response, ContractError> {
    let qid: [u8; 32] =
        question_id
            .as_slice()
            .try_into()
            .map_err(|_| ContractError::QuestionNotFound {
                id: question_id.to_base64(),
            })?;
    let mut question =
        QUESTIONS
            .may_load(deps.storage, &qid)?
            .ok_or_else(|| ContractError::QuestionNotFound {
                id: hex::encode(qid),
            })?;

    if !question.is_pending_arbitration {
        return Err(ContractError::InvalidState {
            expected: "PendingArbitration".to_string(),
            actual: format!("{:?}", question.state_at(env.block.time.seconds())),
        });
    }

    let now = env.block.time.seconds();
    let is_arbitrator = question
        .arbitrator
        .as_ref()
        .map(|a| *a == info.sender)
        .unwrap_or(false);
    let deadline_passed = question
        .arbitration_deadline
        .map(|d| d <= now)
        .unwrap_or(false);

    if !is_arbitrator && !deadline_passed {
        return Err(ContractError::Unauthorized {});
    }

    question.is_pending_arbitration = false;
    question.arbitration_deadline = None;
    // Re-extend finalize_ts from now — not restore. Reality.eth precedent
    // (`RealityETH-3.0.sol:530-531`).
    let new_finalize_ts = now.saturating_add(u64::from(question.answer_timeout_secs));
    question.finalize_ts = Some(new_finalize_ts);

    QUESTIONS.save(deps.storage, &qid, &question)?;

    Ok(Response::new()
        .add_attribute("action", "cancel_arbitration")
        .add_attribute("question_id", hex::encode(qid))
        .add_attribute("canceller", info.sender.as_str())
        .add_attribute("new_finalize_ts", new_finalize_ts.to_string())
        .add_event(
            Event::new("cw_reality/arbitration_cancelled")
                .add_attribute("question_id", Binary::from(qid).to_base64())
                .add_attribute("canceller", info.sender.as_str())
                .add_attribute("new_finalize_ts", new_finalize_ts.to_string()),
        ))
}

pub fn execute_submit_arbitration(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    question_id: Binary,
    winning_answer: Binary,
    payee: String,
) -> Result<Response, ContractError> {
    let qid: [u8; 32] =
        question_id
            .as_slice()
            .try_into()
            .map_err(|_| ContractError::QuestionNotFound {
                id: question_id.to_base64(),
            })?;
    let mut question =
        QUESTIONS
            .may_load(deps.storage, &qid)?
            .ok_or_else(|| ContractError::QuestionNotFound {
                id: hex::encode(qid),
            })?;

    let arbitrator = question
        .arbitrator
        .clone()
        .ok_or(ContractError::NoArbitrator {})?;
    if info.sender != arbitrator {
        return Err(ContractError::NotArbitrator {});
    }
    if !question.is_pending_arbitration {
        return Err(ContractError::InvalidState {
            expected: "PendingArbitration".to_string(),
            actual: format!("{:?}", question.state_at(env.block.time.seconds())),
        });
    }

    let payee: Addr = deps.api.addr_validate(&payee)?;

    // Reality.eth lets the arbitrator pick ANY answer (including new ones not
    // in history). We match that — the ARBITRATION.md "pick from history"
    // wording was tightened in the source-walk: the trust boundary is the
    // arbitrator address, not the answer surface. The arbitrator authoring
    // a new answer is functionally equivalent to choosing a juror's answer
    // off-chain. The `UNRESOLVED_ANSWER_BYTES` sentinel is the explicit-
    // decline path.
    let is_unresolved = winning_answer.as_slice() == UNRESOLVED_ANSWER_BYTES.as_slice();

    let now = env.block.time.seconds();

    // Append an arbitrator history entry: bond = 0, answerer = payee,
    // is_commitment = false. The `questions[qid].bond` (current_bond) field
    // stays as the last user bond — that's what the claim walk uses to skip
    // the chain-tip 2.5% shave (lessons §3 invariant 4).
    let prev_hash = question.history_hash;
    let new_hash = next_history_hash(
        deps.api,
        &prev_hash,
        &winning_answer,
        &question.bond_denom,
        Uint128::zero(),
        &payee,
        false,
    )?;

    question.is_pending_arbitration = false;
    question.arbitration_deadline = None;
    question.best_answer = Some(winning_answer.clone());
    question.history_hash = new_hash;
    question.round_count = question.round_count.saturating_add(1);
    question.finalize_ts = Some(now);

    QUESTIONS.save(deps.storage, &qid, &question)?;

    Ok(Response::new()
        .add_attribute("action", "submit_arbitration")
        .add_attribute("question_id", hex::encode(qid))
        .add_attribute("arbitrator", arbitrator.as_str())
        .add_attribute("payee", payee.as_str())
        .add_attribute("unresolved", is_unresolved.to_string())
        .add_event(
            Event::new("cw_reality/arbitration_submitted")
                .add_attribute("question_id", Binary::from(qid).to_base64())
                .add_attribute("arbitrator", arbitrator.as_str())
                .add_attribute("winning_answer", winning_answer.to_base64())
                .add_attribute("payee", payee.as_str())
                .add_attribute("prev_history_hash", Binary::from(prev_hash).to_base64())
                .add_attribute("history_hash", Binary::from(new_hash).to_base64())
                .add_attribute("unresolved", is_unresolved.to_string()),
        ))
}
