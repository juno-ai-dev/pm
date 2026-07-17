//! Reusable fail-closed execution guards.

use crate::{
    error::ContractError,
    msg::LifecycleStatus,
    state::{Challenge, Config, Lifecycle},
};
use cosmwasm_std::{Addr, Coin, Env, Uint128};

pub fn exact_funds(funds: &[Coin], denom: &str, expected: Uint128) -> Result<(), ContractError> {
    if funds.len() == 1 && funds[0].denom == denom && funds[0].amount == expected {
        return Ok(());
    }
    Err(ContractError::InvalidFunds {
        expected,
        denom: denom.to_owned(),
    })
}

pub fn no_funds(funds: &[Coin]) -> Result<(), ContractError> {
    if funds.is_empty() {
        Ok(())
    } else {
        Err(ContractError::UnexpectedFunds)
    }
}

pub fn sender(sender: &Addr, required: &Addr) -> Result<(), ContractError> {
    if sender == required {
        Ok(())
    } else {
        Err(ContractError::Unauthorized)
    }
}

pub fn derived_lifecycle(
    now: u64,
    config: &Config,
    lifecycle: &Lifecycle,
    challenge: Option<&Challenge>,
) -> LifecycleStatus {
    if lifecycle.payout.is_some() {
        LifecycleStatus::Resolved
    } else if challenge.is_some() {
        LifecycleStatus::PendingArbitration
    } else if !lifecycle.activated {
        LifecycleStatus::Initializing
    } else if now < config.close_ts {
        LifecycleStatus::Trading
    } else {
        LifecycleStatus::AwaitingResolution
    }
}

pub fn trading(env: &Env, config: &Config, lifecycle: &Lifecycle) -> Result<(), ContractError> {
    if !lifecycle.activated {
        return Err(ContractError::NotActivated);
    }
    if lifecycle.payout.is_some() {
        return Err(ContractError::AlreadyResolved);
    }
    if env.block.time.seconds() >= config.close_ts {
        return Err(ContractError::MarketClosed);
    }
    Ok(())
}

pub fn user_deadline(env: &Env, deadline: u64) -> Result<(), ContractError> {
    if env.block.time.seconds() > deadline {
        Err(ContractError::DeadlineExpired)
    } else {
        Ok(())
    }
}

pub fn unresolved(lifecycle: &Lifecycle) -> Result<(), ContractError> {
    if lifecycle.payout.is_some() {
        Err(ContractError::AlreadyResolved)
    } else {
        Ok(())
    }
}

pub fn governance_verdict(
    env: &Env,
    sender_addr: &Addr,
    config: &Config,
    challenge: Option<&Challenge>,
) -> Result<(), ContractError> {
    sender(sender_addr, &config.verdict_authority)?;
    let challenge = challenge.ok_or(ContractError::NoPendingChallenge)?;
    if env.block.time.seconds() >= challenge.deadline {
        return Err(ContractError::ArbitrationDeadlineReached);
    }
    Ok(())
}
