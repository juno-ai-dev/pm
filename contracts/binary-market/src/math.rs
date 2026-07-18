//! Pure, caller-adverse integer FPMM quote arithmetic.
//!
//! Query and execute paths must call these routines over the same reserve
//! snapshot. This module intentionally performs no storage or bank operations.

use cosmwasm_std::{Uint128, Uint256};
use pm_types::Outcome;
use thiserror::Error;

/// Basis-point denominator used by market fees.
pub const FEE_SCALE: u16 = 10_000;
/// Accepted canary minimum gross buy or requested sell return, in ujuno.
pub const MIN_TRADE_UJUNO: u128 = 10_000;
/// Accepted canary per-call bound: net split/merge may use at most one quarter
/// of the smaller reserve.
pub const MAX_TRADE_RESERVE_RATIO_DENOMINATOR: u128 = 4;

/// The two positive pool reserves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reserves {
    pub yes: Uint128,
    pub no: Uint128,
}

impl Reserves {
    fn validate(self) -> Result<(), MathError> {
        if self.yes.is_zero() || self.no.is_zero() {
            return Err(MathError::ZeroReserve);
        }
        Ok(())
    }

    fn selected(self, outcome: Outcome) -> (Uint128, Uint128) {
        match outcome {
            Outcome::Yes => (self.yes, self.no),
            Outcome::No => (self.no, self.yes),
        }
    }

    fn from_selected(outcome: Outcome, selected: Uint128, opposite: Uint128) -> Self {
        match outcome {
            Outcome::Yes => Self {
                yes: selected,
                no: opposite,
            },
            Outcome::No => Self {
                yes: opposite,
                no: selected,
            },
        }
    }

    /// Checked reserve product in the 256-bit arithmetic domain.
    pub fn product(self) -> Result<Uint256, MathError> {
        self.validate()?;
        checked_mul(self.yes.into(), self.no.into())
    }
}

/// An exact, unreduced non-negative ratio.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuoteRatio {
    pub numerator: Uint256,
    pub denominator: Uint256,
}

/// Complete result of a buy-exact-collateral quote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuyQuote {
    pub gross_collateral: Uint128,
    pub net_collateral: Uint128,
    pub fee: Uint128,
    pub outcome_out: Uint128,
    pub reserves_before: Reserves,
    pub reserves_after: Reserves,
    /// Reserve-derived marginal quote before the trade.
    pub marginal_before: QuoteRatio,
    /// Reserve-derived marginal quote after the trade.
    pub marginal_after: QuoteRatio,
    /// Gross collateral paid per outcome unit received.
    pub average_execution: QuoteRatio,
}

/// Complete result of a sell-for-exact-collateral quote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SellQuote {
    /// Complete sets that must be merged (net return plus fee).
    pub gross_collateral: Uint128,
    /// Exact collateral returned to the seller.
    pub net_collateral: Uint128,
    pub fee: Uint128,
    pub outcome_in: Uint128,
    pub reserves_before: Reserves,
    pub reserves_after: Reserves,
    /// Reserve-derived marginal quote before the trade.
    pub marginal_before: QuoteRatio,
    /// Reserve-derived marginal quote after the trade.
    pub marginal_after: QuoteRatio,
    /// Net collateral received per outcome unit supplied.
    pub average_execution: QuoteRatio,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum MathError {
    #[error("pool reserves must both be positive")]
    ZeroReserve,
    #[error("trade must be at least {MIN_TRADE_UJUNO} ujuno")]
    TradeBelowMinimum,
    #[error("fee must be below {FEE_SCALE} basis points")]
    InvalidFee,
    #[error("trade produces zero net amount or zero outcome amount")]
    ZeroTradeResult,
    #[error("net split or merge exceeds one quarter of the smaller reserve")]
    TradeRatioExceeded,
    #[error("sell merge must remain below the opposite reserve")]
    OppositeReserveExhausted,
    #[error("checked arithmetic overflow or underflow")]
    Arithmetic,
    #[error("256-bit result does not fit a stored 128-bit amount")]
    AmountOutOfRange,
    #[error("post-trade reserve product decreased")]
    ProductDecreased,
}

/// Exact reserve-derived marginal quote for an outcome.
///
/// This ratio is answer-independent display information, not a probability.
pub fn marginal_quote(reserves: Reserves, outcome: Outcome) -> Result<QuoteRatio, MathError> {
    reserves.validate()?;
    let numerator: Uint256 = match outcome {
        Outcome::Yes => reserves.no.into(),
        Outcome::No => reserves.yes.into(),
    };
    let denominator = checked_add(reserves.yes.into(), reserves.no.into())?;
    Ok(QuoteRatio {
        numerator,
        denominator,
    })
}

/// Quote a purchase funded by an exact gross collateral amount.
pub fn buy_exact_collateral(
    reserves: Reserves,
    outcome: Outcome,
    gross: Uint128,
    fee_bps: u16,
) -> Result<BuyQuote, MathError> {
    reserves.validate()?;
    validate_trade_and_fee(gross, fee_bps)?;

    let fee = to_u128(ceil_div(
        checked_mul(gross.into(), Uint256::from(fee_bps))?,
        Uint256::from(FEE_SCALE),
    )?)?;
    let net = gross.checked_sub(fee).map_err(|_| MathError::Arithmetic)?;
    if net.is_zero() {
        return Err(MathError::ZeroTradeResult);
    }
    enforce_trade_ratio(net, reserves)?;

    let (selected, opposite) = reserves.selected(outcome.clone());
    let product = reserves.product()?;
    let new_opposite = opposite
        .checked_add(net)
        .map_err(|_| MathError::Arithmetic)?;
    let ending_selected = to_u128(ceil_div(product, new_opposite.into())?)?;
    let outcome_out = selected
        .checked_add(net)
        .map_err(|_| MathError::Arithmetic)?
        .checked_sub(ending_selected)
        .map_err(|_| MathError::Arithmetic)?;
    if outcome_out.is_zero() || ending_selected.is_zero() {
        return Err(MathError::ZeroTradeResult);
    }

    let reserves_after = Reserves::from_selected(outcome.clone(), ending_selected, new_opposite);
    ensure_product_direction(reserves, reserves_after)?;

    Ok(BuyQuote {
        gross_collateral: gross,
        net_collateral: net,
        fee,
        outcome_out,
        reserves_before: reserves,
        reserves_after,
        marginal_before: marginal_quote(reserves, outcome.clone())?,
        marginal_after: marginal_quote(reserves_after, outcome)?,
        average_execution: QuoteRatio {
            numerator: gross.into(),
            denominator: outcome_out.into(),
        },
    })
}

/// Quote the outcome input required to receive an exact net collateral amount.
pub fn sell_for_exact_collateral(
    reserves: Reserves,
    outcome: Outcome,
    requested_net: Uint128,
    fee_bps: u16,
) -> Result<SellQuote, MathError> {
    reserves.validate()?;
    validate_trade_and_fee(requested_net, fee_bps)?;

    let merge = to_u128(ceil_div(
        checked_mul(requested_net.into(), Uint256::from(FEE_SCALE))?,
        Uint256::from(FEE_SCALE - fee_bps),
    )?)?;
    let fee = merge
        .checked_sub(requested_net)
        .map_err(|_| MathError::Arithmetic)?;
    enforce_trade_ratio(merge, reserves)?;

    let (selected, opposite) = reserves.selected(outcome.clone());
    if merge >= opposite {
        return Err(MathError::OppositeReserveExhausted);
    }
    let new_opposite = opposite
        .checked_sub(merge)
        .map_err(|_| MathError::Arithmetic)?;
    if new_opposite.is_zero() {
        return Err(MathError::OppositeReserveExhausted);
    }
    let product = reserves.product()?;
    let selected_before_merge = to_u128(ceil_div(product, new_opposite.into())?)?;
    let outcome_in = merge
        .checked_add(selected_before_merge)
        .map_err(|_| MathError::Arithmetic)?
        .checked_sub(selected)
        .map_err(|_| MathError::Arithmetic)?;
    if outcome_in.is_zero() {
        return Err(MathError::ZeroTradeResult);
    }
    let new_selected = selected
        .checked_add(outcome_in)
        .map_err(|_| MathError::Arithmetic)?
        .checked_sub(merge)
        .map_err(|_| MathError::Arithmetic)?;
    let reserves_after = Reserves::from_selected(outcome.clone(), new_selected, new_opposite);
    ensure_product_direction(reserves, reserves_after)?;

    Ok(SellQuote {
        gross_collateral: merge,
        net_collateral: requested_net,
        fee,
        outcome_in,
        reserves_before: reserves,
        reserves_after,
        marginal_before: marginal_quote(reserves, outcome.clone())?,
        marginal_after: marginal_quote(reserves_after, outcome)?,
        average_execution: QuoteRatio {
            numerator: requested_net.into(),
            denominator: outcome_in.into(),
        },
    })
}

fn validate_trade_and_fee(amount: Uint128, fee_bps: u16) -> Result<(), MathError> {
    if amount.u128() < MIN_TRADE_UJUNO {
        return Err(MathError::TradeBelowMinimum);
    }
    if fee_bps >= FEE_SCALE {
        return Err(MathError::InvalidFee);
    }
    Ok(())
}

fn enforce_trade_ratio(amount: Uint128, reserves: Reserves) -> Result<(), MathError> {
    let smaller = reserves.yes.min(reserves.no);
    let max = smaller / Uint128::from(MAX_TRADE_RESERVE_RATIO_DENOMINATOR);
    if amount > max {
        return Err(MathError::TradeRatioExceeded);
    }
    Ok(())
}

fn ensure_product_direction(before: Reserves, after: Reserves) -> Result<(), MathError> {
    after.validate()?;
    if after.product()? < before.product()? {
        return Err(MathError::ProductDecreased);
    }
    Ok(())
}

fn ceil_div(numerator: Uint256, denominator: Uint256) -> Result<Uint256, MathError> {
    if denominator.is_zero() {
        return Err(MathError::Arithmetic);
    }
    let quotient = numerator
        .checked_div(denominator)
        .map_err(|_| MathError::Arithmetic)?;
    let remainder = numerator
        .checked_rem(denominator)
        .map_err(|_| MathError::Arithmetic)?;
    if remainder.is_zero() {
        Ok(quotient)
    } else {
        checked_add(quotient, Uint256::one())
    }
}

fn checked_add(left: Uint256, right: Uint256) -> Result<Uint256, MathError> {
    left.checked_add(right).map_err(|_| MathError::Arithmetic)
}

fn checked_mul(left: Uint256, right: Uint256) -> Result<Uint256, MathError> {
    left.checked_mul(right).map_err(|_| MathError::Arithmetic)
}

fn to_u128(value: Uint256) -> Result<Uint128, MathError> {
    value.try_into().map_err(|_| MathError::AmountOutOfRange)
}
