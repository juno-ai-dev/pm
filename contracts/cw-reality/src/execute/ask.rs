//! `ExecuteMsg::AskQuestion` handler.
//!
//! Defenses encoded here (see `docs/reality-eth-lessons.md` §5 + §6):
//! - FM-1: `initial_bond >= config.min_initial_bond_floor`.
//! - FM-5: `question_id` includes the contract address (cross-deployment-
//!   collision defense).
//! - FM-7: `current_bond_seen` does not apply here — there is no prior bond
//!   at ask time.
//! - FM-8: `answer_timeout_secs >= config.min_answer_timeout_secs` AND
//!   `<= MAX_ANSWER_TIMEOUT_SECS`.
//! - FM-11: question parameters become part of the question_id — they are
//!   immutable post-creation.

use cosmwasm_std::{Addr, Binary, Coin, DepsMut, Env, MessageInfo, Response, Uint128};

use crate::error::ContractError;
use crate::escalation::{DEFAULT_ARBITRATION_TIMEOUT_SECS, MAX_ANSWER_TIMEOUT_SECS};
use crate::filter::AnswerSchemaFilter;
use crate::hash::NULL_HISTORY_HASH;
use crate::id;
use crate::state::{AnswerType, Question, CONFIG, QUESTIONS};

#[allow(clippy::too_many_arguments)]
pub fn execute_ask_question(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    text: String,
    answer_type: AnswerType,
    bond_denom: String,
    initial_bond: Uint128,
    answer_timeout_secs: u32,
    arbitrator: Option<String>,
    arbitration_timeout_secs: Option<u32>,
    answer_schema: Option<AnswerSchemaFilter>,
    opening_ts: Option<u64>,
    nonce: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if bond_denom.is_empty() {
        return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
            "bond_denom must be non-empty",
        )));
    }
    if initial_bond < config.min_initial_bond_floor {
        return Err(ContractError::InitialBondBelowFloor {
            provided: initial_bond,
            floor: config.min_initial_bond_floor,
        });
    }
    if answer_timeout_secs < config.min_answer_timeout_secs {
        return Err(ContractError::AnswerTimeoutBelowFloor {
            provided: answer_timeout_secs,
            floor: config.min_answer_timeout_secs,
        });
    }
    if answer_timeout_secs > MAX_ANSWER_TIMEOUT_SECS {
        return Err(ContractError::AnswerTimeoutTooHigh {});
    }

    let arbitrator_addr: Option<Addr> = arbitrator
        .as_deref()
        .map(|a| deps.api.addr_validate(a))
        .transpose()?;

    let arbitration_timeout_secs =
        arbitration_timeout_secs.unwrap_or(DEFAULT_ARBITRATION_TIMEOUT_SECS);
    if arbitration_timeout_secs > MAX_ANSWER_TIMEOUT_SECS {
        return Err(ContractError::AnswerTimeoutTooHigh {});
    }

    // The asker may include a native bounty in `info.funds` — at most one
    // coin, in `bond_denom`. cw20 bounty arrives via the Receive path.
    let bounty = native_bounty(&info.funds, &bond_denom)?;

    let content_h = id::content_hash(&text);
    let qid = id::question_id(
        deps.api,
        &env.contract.address,
        &info.sender,
        nonce,
        &content_h,
        arbitrator_addr.as_ref(),
        answer_timeout_secs,
        initial_bond,
        &bond_denom,
        opening_ts,
    )?;

    if QUESTIONS.has(deps.storage, &qid) {
        return Err(ContractError::QuestionAlreadyExists {
            id: hex::encode(qid),
        });
    }

    let question = Question {
        asker: info.sender.clone(),
        text,
        answer_type,
        bond_denom: bond_denom.clone(),
        initial_bond,
        min_bond: initial_bond,
        answer_timeout_secs,
        arbitrator: arbitrator_addr.clone(),
        arbitration_timeout_secs,
        arbitration_deadline: None,
        answer_schema,
        nonce,
        opening_ts,
        bounty,
        best_answer: None,
        current_bond: Uint128::zero(),
        history_hash: NULL_HISTORY_HASH,
        round_count: 0,
        finalize_ts: None,
        is_pending_arbitration: false,
        is_claimed: false,
    };

    QUESTIONS.save(deps.storage, &qid, &question)?;

    Ok(Response::new()
        .add_attribute("action", "ask_question")
        .add_attribute("question_id", hex::encode(qid))
        .add_attribute("asker", info.sender.as_str())
        .add_attribute("bond_denom", &bond_denom)
        .add_attribute("initial_bond", initial_bond.to_string())
        .add_attribute("answer_timeout_secs", answer_timeout_secs.to_string())
        .add_attribute(
            "arbitrator",
            arbitrator_addr
                .as_ref()
                .map(|a| a.as_str())
                .unwrap_or("<none>"),
        )
        .add_attribute("nonce", nonce.to_string())
        .add_attribute("bounty", bounty.to_string())
        .add_attribute("content_hash", hex::encode(content_h))
        .add_event(
            cosmwasm_std::Event::new("cw_reality/new_question")
                .add_attribute("question_id", Binary::from(qid).to_base64())
                .add_attribute("asker", info.sender.as_str())
                .add_attribute("content_hash", Binary::from(content_h).to_base64())
                .add_attribute("bond_denom", &bond_denom)
                .add_attribute("initial_bond", initial_bond.to_string())
                .add_attribute("answer_timeout_secs", answer_timeout_secs.to_string())
                .add_attribute("nonce", nonce.to_string()),
        ))
}

/// Inspect `info.funds`. Returns the bounty amount (in `bond_denom`), or 0 if
/// the asker sent no funds. Rejects multi-denom or wrong-denom funds.
fn native_bounty(funds: &[Coin], bond_denom: &str) -> Result<Uint128, ContractError> {
    match funds.len() {
        0 => Ok(Uint128::zero()),
        1 => {
            let c = &funds[0];
            if c.denom != bond_denom {
                Err(ContractError::BondDenomMismatch {
                    expected: bond_denom.to_string(),
                    actual: c.denom.clone(),
                })
            } else {
                Ok(c.amount)
            }
        }
        n => Err(ContractError::InvalidBondFunds { count: n }),
    }
}
