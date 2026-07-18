//! `ExecuteMsg::Claim` — drive the resumable Reality.eth-style payout walk.
//!
//! Algorithm overview (`docs/reality-eth-lessons.md` §3, source-notes §3):
//!
//! The caller supplies the answer history newest-first. For each entry the
//! contract:
//!   1. Re-hashes `(prev_hash, answer, denom, bond, answerer, is_commitment)`
//!      and compares against the persisted cursor; mismatch reverts.
//!   2. Adds the previous round's bond to `queued_funds` — we now know who
//!      to credit it to.
//!   3. Calls `_process_history_item`: applies Reality.eth's right-answer
//!      rule. Wrong-answer entries are skipped (their bond passes to the
//!      next-earlier right-answerer). Right-answer entries either become the
//!      payee (first match) or trigger an answer-takeover-fee transfer.
//!   4. Shaves 2.5% of every interior bond. The chain-tip bond
//!      (`question.current_bond`) is exempt — that is Reality.eth's "winning
//!      bond" carve-out.
//!
//! The walk resumes from a persisted `Claim` if a previous call ran out of
//! gas mid-chain. On exhaustion (`cursor_hash == NULL`) the final payee gets
//! `queued_funds + last_bond`, the `Claim` record is removed, and the
//! question is marked `is_claimed`. Match Reality.eth.

use cosmwasm_std::{Addr, Binary, DepsMut, Env, Event, MessageInfo, Response, Uint128};

use crate::error::ContractError;
use crate::escalation::shave_interior_bond;
use crate::hash::{next_history_hash, HistoryHash, NULL_HISTORY_HASH};
use crate::msg::HistoryEntry;
use crate::state::{Claim, Question, State, CLAIMS, QUESTIONS, UNRESOLVED_ANSWER_BYTES};

pub fn execute_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    question_id: Binary,
    history_entries: Vec<HistoryEntry>,
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
    if question.state_at(now) != State::Finalized {
        return Err(ContractError::InvalidState {
            expected: "Finalized".to_string(),
            actual: format!("{:?}", question.state_at(now)),
        });
    }

    if history_entries.is_empty() {
        return Err(ContractError::NothingToClaim {});
    }

    // Load resumable state (or initialize on first call).
    let mut claim = CLAIMS
        .may_load(deps.storage, &qid)?
        .unwrap_or_else(|| Claim {
            payee: None,
            last_bond: Uint128::zero(),
            queued_funds: Uint128::zero(),
            cursor_hash: question.history_hash,
        });

    let best_answer = question
        .best_answer
        .clone()
        .ok_or(ContractError::NotFinalized {})?;
    let is_unresolved = best_answer.as_slice() == UNRESOLVED_ANSWER_BYTES.as_slice();

    let mut credits: Vec<(Addr, Uint128)> = Vec::new();

    for (i, entry) in history_entries.iter().enumerate() {
        if claim.cursor_hash == NULL_HISTORY_HASH {
            // Chain already exhausted — caller supplied trailing junk.
            break;
        }

        // Verify the supplied entry hashes against the cursor.
        let prev_hash: HistoryHash = entry
            .prev_hash
            .as_slice()
            .try_into()
            .map_err(|_| ContractError::HistoryHashMismatch { step: i })?;
        let answerer = deps.api.addr_validate(&entry.answerer)?;
        let expected = next_history_hash(
            deps.api,
            &prev_hash,
            &entry.answer,
            &question.bond_denom,
            entry.bond_amount,
            &answerer,
            entry.is_commitment,
        )?;
        if expected != claim.cursor_hash {
            return Err(ContractError::HistoryHashMismatch { step: i });
        }

        // Pull the previous round's bond into queued_funds — we know who to
        // credit it to now.
        claim.queued_funds = claim
            .queued_funds
            .checked_add(claim.last_bond)
            .map_err(ContractError::Overflow)?;

        // Process the right-answer rule for this entry.
        process_history_item(
            &mut question,
            &mut claim,
            &answerer,
            entry.bond_amount,
            &entry.answer,
            entry.is_commitment,
            &best_answer,
            is_unresolved,
            &mut credits,
        );

        claim.last_bond = entry.bond_amount;

        // 2.5% interior shave — chain-tip bond is exempt.
        if claim.last_bond != question.current_bond {
            claim.last_bond = shave_interior_bond(claim.last_bond);
        }

        claim.cursor_hash = prev_hash;
    }

    let chain_exhausted = claim.cursor_hash == NULL_HISTORY_HASH;
    if chain_exhausted {
        // Final credit: the very first answerer (who is the current `payee`
        // unless no right-answer was ever submitted) gets the residual.
        if let Some(p) = &claim.payee {
            let final_amount = claim
                .queued_funds
                .checked_add(claim.last_bond)
                .map_err(ContractError::Overflow)?;
            credits.push((p.clone(), final_amount));
        } else {
            // No right answer ever found. All bonds drain to the contract
            // (which functions as a burn). queued_funds + last_bond stays in
            // the contract's bank balance.
        }
        CLAIMS.remove(deps.storage, &qid);
        question.is_claimed = true;
    } else {
        // Persist resumable state. Pay out the queued portion to the current
        // payee so each call has a deterministic on-chain effect (Reality.eth
        // does this — lessons §3).
        if let Some(p) = &claim.payee {
            if !claim.queued_funds.is_zero() {
                credits.push((p.clone(), claim.queued_funds));
                claim.queued_funds = Uint128::zero();
            }
        }
        CLAIMS.save(deps.storage, &qid, &claim)?;
    }

    QUESTIONS.save(deps.storage, &qid, &question)?;

    // Apply pull-payment credits in a single sweep (no inline BankMsg — FM-12
    // reentrancy posture).
    let denom = question.bond_denom.clone();
    apply_credits(deps, &denom, credits.clone())?;

    let mut response = Response::new()
        .add_attribute("action", "claim")
        .add_attribute("question_id", hex::encode(qid))
        .add_attribute("caller", info.sender.as_str())
        .add_attribute("steps_processed", history_entries.len().to_string())
        .add_attribute("chain_exhausted", chain_exhausted.to_string());

    for (recipient, amount) in &credits {
        response = response.add_event(
            Event::new("cw_reality/claim_credit")
                .add_attribute("question_id", Binary::from(qid).to_base64())
                .add_attribute("recipient", recipient.as_str())
                .add_attribute("denom", &denom)
                .add_attribute("amount", amount.to_string()),
        );
    }
    if chain_exhausted {
        response = response.add_event(
            Event::new("cw_reality/claim_finalized")
                .add_attribute("question_id", Binary::from(qid).to_base64()),
        );
    }

    Ok(response)
}

/// Reality.eth's `_processHistoryItem` ported.
///
/// On a right-answer match: either become the chain-tip payee (first hit),
/// or pay the current payee a takeover fee and hand over.
#[allow(clippy::too_many_arguments)]
fn process_history_item(
    question: &mut Question,
    claim: &mut Claim,
    addr: &Addr,
    bond: Uint128,
    answer: &Binary,
    is_commitment: bool,
    best_answer: &Binary,
    is_unresolved: bool,
    credits: &mut Vec<(Addr, Uint128)>,
) {
    // v1 ships without commit-reveal (lessons §0). Treat any commitment
    // entry as a no-op match — its bond was already added to queued_funds
    // before this function ran, so it passes to the next earlier right-
    // answerer.
    if is_commitment {
        return;
    }

    if answer.as_slice() != best_answer.as_slice() {
        // Wrong answer — its bond is already in queued_funds; nothing more
        // to do on this entry.
        return;
    }

    match &claim.payee {
        None => {
            // First-encountered winning answer (walking newest→oldest, so
            // this is the LATEST winning answerer).
            claim.payee = Some(addr.clone());
            if !is_unresolved && !question.bounty.is_zero() {
                credits.push((addr.clone(), question.bounty));
                question.bounty = Uint128::zero();
            }
        }
        Some(current_payee) if current_payee != addr => {
            // An earlier right-answerer different from the current payee.
            // Pay the current payee an "answer takeover fee" capped at the
            // earlier answerer's bond — Reality.eth invariant 3.
            let takeover_fee = if claim.queued_funds >= bond {
                bond
            } else {
                claim.queued_funds
            };
            let pay_current = claim.queued_funds - takeover_fee;
            if !pay_current.is_zero() {
                credits.push((current_payee.clone(), pay_current));
            }
            claim.payee = Some(addr.clone());
            claim.queued_funds = takeover_fee;
        }
        Some(_) => {
            // Same payee gave the right answer again earlier — keep
            // accumulating into their queued_funds.
        }
    }
}

fn apply_credits(
    deps: DepsMut,
    denom: &str,
    credits: Vec<(Addr, Uint128)>,
) -> Result<(), ContractError> {
    use crate::state::BALANCES;
    for (recipient, amount) in credits {
        if amount.is_zero() {
            continue;
        }
        BALANCES.update(
            deps.storage,
            (&recipient, denom),
            |existing| -> Result<Uint128, ContractError> {
                existing
                    .unwrap_or_default()
                    .checked_add(amount)
                    .map_err(ContractError::Overflow)
            },
        )?;
    }
    Ok(())
}
