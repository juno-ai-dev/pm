//! `ExecuteMsg::FundBounty` handler.
//!
//! Tops up the bounty paid to the eventual winner. Permitted while the
//! question is not yet finalized — matches Reality.eth `fundAnswerBounty`.

use cosmwasm_std::{Binary, Coin, DepsMut, Env, MessageInfo, Response, Uint128};

use crate::error::ContractError;
use crate::state::{State, QUESTIONS};

pub fn execute_fund_bounty(
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

    let amount = single_coin_amount(&info.funds, &question.bond_denom)?;
    if amount.is_zero() {
        return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
            "FundBounty: zero-value transfer",
        )));
    }

    question.bounty = question.bounty.checked_add(amount)?;
    QUESTIONS.save(deps.storage, &qid, &question)?;

    Ok(Response::new()
        .add_attribute("action", "fund_bounty")
        .add_attribute("question_id", hex::encode(qid))
        .add_attribute("denom", &question.bond_denom)
        .add_attribute("amount", amount.to_string())
        .add_attribute("new_bounty", question.bounty.to_string()))
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
            } else {
                Ok(c.amount)
            }
        }
        n => Err(ContractError::InvalidBondFunds { count: n }),
    }
}
