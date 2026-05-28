//! `ExecuteMsg::Withdraw` — drain the caller's pull-payment balance for a
//! single denom.
//!
//! Reality.eth's `BalanceHolder.withdraw` is the only outbound transfer in
//! the contract; cw-reality mirrors that. Bonds are credited via the `Claim`
//! walk, then drained here. No inline BankMsg during state mutation —
//! reentrancy posture per FM-12 / lessons §2.1.

use cosmwasm_std::{BankMsg, Coin, DepsMut, Env, Event, MessageInfo, Response};

use crate::error::ContractError;
use crate::state::BALANCES;

pub fn execute_withdraw(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    denom: String,
) -> Result<Response, ContractError> {
    if denom.is_empty() {
        return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
            "withdraw: denom must be non-empty",
        )));
    }
    let amount = BALANCES
        .may_load(deps.storage, (&info.sender, &denom))?
        .unwrap_or_default();
    if amount.is_zero() {
        return Err(ContractError::NothingToWithdraw {});
    }
    BALANCES.remove(deps.storage, (&info.sender, &denom));

    Ok(Response::new()
        .add_message(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![Coin {
                denom: denom.clone(),
                amount,
            }],
        })
        .add_attribute("action", "withdraw")
        .add_attribute("recipient", info.sender.as_str())
        .add_attribute("denom", &denom)
        .add_attribute("amount", amount.to_string())
        .add_event(
            Event::new("cw_reality/withdraw")
                .add_attribute("recipient", info.sender.as_str())
                .add_attribute("denom", denom)
                .add_attribute("amount", amount.to_string()),
        ))
}
