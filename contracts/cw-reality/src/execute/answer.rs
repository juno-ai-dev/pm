//! `ExecuteMsg::SubmitAnswer` and `ExecuteMsg::DisputeAnswer` handlers.
//!
//! Reality.eth has a single `submitAnswer` for both first and subsequent
//! answers; cw-reality splits them so external indexers can distinguish
//! "first answer of a question" from "counter-answer," but the underlying
//! algorithm is the same. The two thin variants call `apply_answer`.
//!
//! Defenses encoded here:
//! - FM-6: `Uint128::checked_mul` for the 2× doubling rule (no silent overflow).
//! - FM-7: `current_bond_seen` front-run guard.
//! - FM-8: indirect — `answer_timeout_secs` was checked at ask time.
//! - FM-12: pull-payment posture — bonds simply land in `info.funds` and
//!   stay in the contract's bank balance; no inline external call.

use cosmwasm_std::{Binary, Coin, DepsMut, Env, Event, MessageInfo, Response, Uint128};

use crate::error::ContractError;
use crate::escalation::{satisfies_doubling, MAX_DISPUTE_ROUNDS};
use crate::filter::{FilterQueryMsg, FilterResponse};
use crate::hash::next_history_hash;
use crate::state::{State, QUESTIONS};

pub fn execute_submit_answer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    question_id: Binary,
    answer: Binary,
    current_bond_seen: Option<Uint128>,
) -> Result<Response, ContractError> {
    apply_answer(
        deps,
        env,
        info,
        question_id,
        answer,
        current_bond_seen,
        "submit_answer",
    )
}

pub fn execute_dispute_answer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    question_id: Binary,
    new_answer: Binary,
    current_bond_seen: Option<Uint128>,
) -> Result<Response, ContractError> {
    apply_answer(
        deps,
        env,
        info,
        question_id,
        new_answer,
        current_bond_seen,
        "dispute_answer",
    )
}

fn apply_answer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    question_id: Binary,
    answer: Binary,
    current_bond_seen: Option<Uint128>,
    action_label: &str,
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

    let now = env.block.time.seconds();
    match question.state_at(now) {
        State::OpenUnanswered | State::OpenAnswered => {}
        s => {
            return Err(ContractError::InvalidState {
                expected: "OpenUnanswered | OpenAnswered".to_string(),
                actual: format!("{s:?}"),
            })
        }
    }

    // opening_ts gate (Reality.eth pattern — question not answerable until ts).
    if let Some(ts) = question.opening_ts {
        if now < ts {
            return Err(ContractError::InvalidState {
                expected: "opening_ts elapsed".to_string(),
                actual: format!("now={now} opening_ts={ts}"),
            });
        }
    }

    // Round cap.
    if question.round_count >= MAX_DISPUTE_ROUNDS {
        return Err(ContractError::RoundCapReached {
            cap: MAX_DISPUTE_ROUNDS,
        });
    }

    // Bond denom + amount.
    let bond_amount = single_coin_amount(&info.funds, &question.bond_denom)?;

    // Front-run guard (FM-7).
    if let Some(expected) = current_bond_seen {
        if question.current_bond > expected {
            return Err(ContractError::BondExceedsExpected {
                actual: question.current_bond,
                expected,
            });
        }
    }

    // Bond magnitude. First answer floor = question.min_bond; subsequent must
    // double.
    if question.current_bond.is_zero() {
        if bond_amount < question.min_bond {
            return Err(ContractError::BondBelowMinimum {
                provided: bond_amount,
                minimum: question.min_bond,
            });
        }
    } else if !satisfies_doubling(question.current_bond, bond_amount) {
        return Err(ContractError::BondMustDouble {
            provided: bond_amount,
            previous: question.current_bond,
        });
    }

    // cw-filter callout if the question has an answer_schema (lessons §7.4 —
    // the cw-filter contract address was captured at ask time so subsequent
    // cw-filter migrations cannot brick the question).
    if let Some(schema) = &question.answer_schema {
        // The dao-proposal-wavs precedent (`contract.rs:441-461`) queries
        // cw-filter with a CosmosMsg envelope. cw-reality wraps the answer
        // bytes in a `Bank::Send` placeholder carrying the answer in its
        // amount metadata; the cw-filter spec is responsible for inspecting
        // that envelope shape. A future filter QueryMsg variant
        // (`FilterValue { filter, value }`) would be cleaner — flagged in
        // lessons §9.3.
        let envelope = answer_envelope(&question.bond_denom, &answer);
        let resp: FilterResponse = deps.querier.query_wasm_smart(
            schema.contract.clone(),
            &FilterQueryMsg::Filter {
                filter: schema.filter.clone(),
                msg: envelope,
            },
        )?;
        match resp {
            FilterResponse::Pass {} => {}
            FilterResponse::Fail { reason } => {
                return Err(ContractError::AnswerFilterFail { index: 0, reason })
            }
            FilterResponse::Fatal { reason } => {
                return Err(ContractError::AnswerFilterFatal { index: 0, reason })
            }
        }
    }

    // History-hash chain update.
    let new_hash = next_history_hash(
        deps.api,
        &question.history_hash,
        &answer,
        &question.bond_denom,
        bond_amount,
        &info.sender,
        false, // is_commitment — v1 ships without commit-reveal (lessons §0)
    )?;

    let prev_hash = question.history_hash;
    question.history_hash = new_hash;
    question.best_answer = Some(answer.clone());
    question.current_bond = bond_amount;
    question.round_count = question.round_count.saturating_add(1);
    let new_finalize_ts = now.saturating_add(u64::from(question.answer_timeout_secs));
    question.finalize_ts = Some(new_finalize_ts);

    QUESTIONS.save(deps.storage, &qid, &question)?;

    Ok(Response::new()
        .add_attribute("action", action_label)
        .add_attribute("question_id", hex::encode(qid))
        .add_attribute("answerer", info.sender.as_str())
        .add_attribute("bond", bond_amount.to_string())
        .add_attribute("round", question.round_count.to_string())
        .add_attribute("finalize_ts", new_finalize_ts.to_string())
        .add_event(
            Event::new("cw_reality/new_answer")
                .add_attribute("question_id", Binary::from(qid).to_base64())
                .add_attribute("prev_history_hash", Binary::from(prev_hash).to_base64())
                .add_attribute("history_hash", Binary::from(new_hash).to_base64())
                .add_attribute("answer", answer.to_base64())
                .add_attribute("bond_denom", &question.bond_denom)
                .add_attribute("bond_amount", bond_amount.to_string())
                .add_attribute("answerer", info.sender.as_str())
                .add_attribute("is_commitment", "false")
                .add_attribute("round", question.round_count.to_string()),
        ))
}

fn single_coin_amount(funds: &[Coin], expected_denom: &str) -> Result<Uint128, ContractError> {
    match funds.len() {
        1 => {
            let c = &funds[0];
            if c.denom != expected_denom {
                Err(ContractError::BondDenomMismatch {
                    expected: expected_denom.to_string(),
                    actual: c.denom.clone(),
                })
            } else if c.amount.is_zero() {
                Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
                    "answer: zero-value bond",
                )))
            } else {
                Ok(c.amount)
            }
        }
        n => Err(ContractError::InvalidBondFunds { count: n }),
    }
}

/// Wrap an answer payload in a `BankMsg::Send` envelope so the cw-filter
/// `QueryMsg::Filter { msg: CosmosMsg }` query has something to inspect.
/// The cw-filter schema is expected to look at the single Coin: denom =
/// the question's bond denom, amount = answer bytes interpreted as a
/// length-prefix-encoded blob via the `to_base64` representation.
///
/// **This is the documented v1 wrapping.** A cleaner long-term solution is a
/// `FilterValue { filter, value: serde_json::Value }` query variant upstream
/// in cw-filter (lessons §9.3, dao-proposal-wavs §4 note).
fn answer_envelope(bond_denom: &str, answer: &Binary) -> cosmwasm_std::CosmosMsg {
    cosmwasm_std::CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
        to_address: format!("cw-reality-answer:{}", answer.to_base64()),
        amount: vec![Coin {
            denom: bond_denom.to_string(),
            amount: Uint128::one(),
        }],
    })
}
