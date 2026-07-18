use binary_market::math::{
    buy_exact_collateral, marginal_quote, sell_for_exact_collateral, MathError, Reserves,
    FEE_SCALE, MIN_TRADE_UJUNO,
};
use cosmwasm_std::{Uint128, Uint256};
use num_bigint::BigUint;
use num_traits::{ToPrimitive, Zero};
use pm_types::Outcome;
use proptest::prelude::*;

fn reserves(yes: u128, no: u128) -> Reserves {
    Reserves {
        yes: Uint128::new(yes),
        no: Uint128::new(no),
    }
}

fn ceil_big(numerator: BigUint, denominator: BigUint) -> BigUint {
    let (quotient, remainder) = (&numerator / &denominator, numerator % &denominator);
    if remainder.is_zero() {
        quotient
    } else {
        quotient + BigUint::from(1_u8)
    }
}

#[test]
fn worked_buy_and_sell_match_every_documented_integer() {
    let start = reserves(100_000_000, 100_000_000);
    let buy = buy_exact_collateral(start, Outcome::Yes, Uint128::new(10_000_000), 200).unwrap();

    assert_eq!(buy.fee, Uint128::new(200_000));
    assert_eq!(buy.net_collateral, Uint128::new(9_800_000));
    assert_eq!(buy.outcome_out, Uint128::new(18_725_318));
    assert_eq!(buy.reserves_after, reserves(91_074_682, 109_800_000));
    assert_eq!(
        buy.reserves_after.product().unwrap(),
        Uint256::from(10_000_000_083_600_000_u128)
    );
    assert_eq!(
        buy.average_execution.numerator,
        Uint256::from(10_000_000_u128)
    );
    assert_eq!(
        buy.average_execution.denominator,
        Uint256::from(18_725_318_u128)
    );

    let sell = sell_for_exact_collateral(
        buy.reserves_after,
        Outcome::Yes,
        Uint128::new(5_000_000),
        200,
    )
    .unwrap();
    assert_eq!(sell.gross_collateral, Uint128::new(5_102_041));
    assert_eq!(sell.fee, Uint128::new(102_041));
    assert_eq!(sell.outcome_in, Uint128::new(9_540_206));
    assert_eq!(sell.reserves_after, reserves(95_512_847, 104_697_959));
    assert_eq!(
        sell.reserves_after.product().unwrap(),
        Uint256::from(10_000_000_139_179_273_u128)
    );
}

#[test]
fn published_buy_table_vectors_match() {
    let vectors = [
        (10_000_000, 100_000, 195_048),
        (10_000_000, 1_000_000, 1_872_531),
        (100_000_000, 100_000, 195_904),
        (100_000_000, 1_000_000, 1_950_489),
        (100_000_000, 10_000_000, 18_725_318),
        (1_000_000_000, 100_000, 195_990),
        (1_000_000_000, 1_000_000, 1_959_040),
        (1_000_000_000, 10_000_000, 19_504_892),
    ];

    for (pool, gross, expected_out) in vectors {
        let quote =
            buy_exact_collateral(reserves(pool, pool), Outcome::Yes, Uint128::new(gross), 200)
                .unwrap();
        assert_eq!(quote.outcome_out, Uint128::new(expected_out));
    }
}

#[test]
fn no_outcome_swaps_the_reserve_axes_exactly() {
    let pool = reserves(80_000_000, 120_000_000);
    let no = buy_exact_collateral(pool, Outcome::No, Uint128::new(1_000_000), 200).unwrap();
    let swapped = buy_exact_collateral(
        reserves(pool.no.u128(), pool.yes.u128()),
        Outcome::Yes,
        Uint128::new(1_000_000),
        200,
    )
    .unwrap();
    assert_eq!(no.outcome_out, swapped.outcome_out);
    assert_eq!(no.reserves_after.yes, swapped.reserves_after.no);
    assert_eq!(no.reserves_after.no, swapped.reserves_after.yes);

    let no_sell =
        sell_for_exact_collateral(pool, Outcome::No, Uint128::new(1_000_000), 200).unwrap();
    let swapped_sell = sell_for_exact_collateral(
        reserves(pool.no.u128(), pool.yes.u128()),
        Outcome::Yes,
        Uint128::new(1_000_000),
        200,
    )
    .unwrap();
    assert_eq!(no_sell.outcome_in, swapped_sell.outcome_in);
    assert_eq!(no_sell.reserves_after.yes, swapped_sell.reserves_after.no);
    assert_eq!(no_sell.reserves_after.no, swapped_sell.reserves_after.yes);
}

#[test]
fn marginal_quotes_are_exact_reserve_ratios() {
    let pool = reserves(3, 7);
    let yes = marginal_quote(pool, Outcome::Yes).unwrap();
    let no = marginal_quote(pool, Outcome::No).unwrap();
    assert_eq!(
        (yes.numerator, yes.denominator),
        (7_u128.into(), 10_u128.into())
    );
    assert_eq!(
        (no.numerator, no.denominator),
        (3_u128.into(), 10_u128.into())
    );
}

#[test]
fn rejects_boundaries_without_saturation_or_zero_denominators() {
    let pool = reserves(100_000_000, 100_000_000);
    assert_eq!(
        buy_exact_collateral(pool, Outcome::Yes, Uint128::new(MIN_TRADE_UJUNO - 1), 200),
        Err(MathError::TradeBelowMinimum)
    );
    assert_eq!(
        sell_for_exact_collateral(pool, Outcome::Yes, Uint128::new(MIN_TRADE_UJUNO), FEE_SCALE),
        Err(MathError::InvalidFee)
    );
    assert_eq!(
        buy_exact_collateral(
            reserves(0, 1),
            Outcome::Yes,
            Uint128::new(MIN_TRADE_UJUNO),
            0
        ),
        Err(MathError::ZeroReserve)
    );
    assert_eq!(
        buy_exact_collateral(pool, Outcome::Yes, Uint128::new(25_000_001), 0),
        Err(MathError::TradeRatioExceeded)
    );
    assert_eq!(
        sell_for_exact_collateral(pool, Outcome::Yes, Uint128::new(25_000_001), 0),
        Err(MathError::TradeRatioExceeded)
    );
    assert!(buy_exact_collateral(pool, Outcome::Yes, Uint128::new(25_000_000), 0).is_ok());
    assert!(sell_for_exact_collateral(pool, Outcome::Yes, Uint128::new(25_000_000), 0).is_ok());
    assert_eq!(
        buy_exact_collateral(
            reserves(u128::MAX, u128::MAX),
            Outcome::Yes,
            Uint128::new(u128::MAX / 4),
            0,
        ),
        Err(MathError::Arithmetic)
    );
}

#[test]
fn fee_extremes_are_checked_and_caller_adverse() {
    let pool = reserves(1_000_000_000, 1_000_000_000);
    let free = buy_exact_collateral(pool, Outcome::Yes, Uint128::new(10_000), 0).unwrap();
    assert_eq!(free.fee, Uint128::zero());
    assert_eq!(free.net_collateral, Uint128::new(10_000));

    let high = buy_exact_collateral(pool, Outcome::Yes, Uint128::new(20_000), 9_999).unwrap();
    assert_eq!(high.fee, Uint128::new(19_998));
    assert_eq!(high.net_collateral, Uint128::new(2));
    assert!(!high.outcome_out.is_zero());
}

#[test]
fn repeated_calls_never_decrease_product() {
    let mut pool = reserves(1_000_000_000, 1_000_000_000);
    for outcome in [Outcome::Yes, Outcome::No, Outcome::Yes, Outcome::No] {
        let before = pool.product().unwrap();
        let quote = buy_exact_collateral(pool, outcome, Uint128::new(1_000_000), 200).unwrap();
        assert!(quote.reserves_after.product().unwrap() >= before);
        pool = quote.reserves_after;
    }
}

proptest! {
    #[test]
    fn buy_matches_arbitrary_precision_reference(
        yes in 1_000_000_u128..1_000_000_000_000_u128,
        no in 1_000_000_u128..1_000_000_000_000_u128,
        fee_bps in 0_u16..1_000_u16,
        fraction in 1_u128..=25_u128,
    ) {
        let pool = reserves(yes, no);
        let smaller = yes.min(no);
        let gross = (smaller / 100).saturating_mul(fraction).max(MIN_TRADE_UJUNO);
        prop_assume!(gross <= smaller / 4);

        let quote = buy_exact_collateral(pool, Outcome::Yes, Uint128::new(gross), fee_bps).unwrap();
        let gross_big = BigUint::from(gross);
        let fee = ceil_big(
            &gross_big * BigUint::from(fee_bps),
            BigUint::from(FEE_SCALE),
        );
        let net = &gross_big - &fee;
        prop_assume!(!net.is_zero());
        let ending = ceil_big(
            BigUint::from(yes) * BigUint::from(no),
            BigUint::from(no) + &net,
        );
        let output = BigUint::from(yes) + &net - &ending;

        prop_assert_eq!(quote.fee.u128(), fee.to_u128().unwrap());
        prop_assert_eq!(quote.net_collateral.u128(), net.to_u128().unwrap());
        prop_assert_eq!(quote.outcome_out.u128(), output.to_u128().unwrap());
        prop_assert!(quote.reserves_after.product().unwrap() >= pool.product().unwrap());
    }

    #[test]
    fn sell_matches_arbitrary_precision_reference(
        yes in 1_000_000_u128..1_000_000_000_000_u128,
        no in 1_000_000_u128..1_000_000_000_000_u128,
        fee_bps in 0_u16..1_000_u16,
        fraction in 1_u128..=20_u128,
    ) {
        let pool = reserves(yes, no);
        let smaller = yes.min(no);
        let requested = (smaller / 100).saturating_mul(fraction).max(MIN_TRADE_UJUNO);
        let requested_big = BigUint::from(requested);
        let merge = ceil_big(
            &requested_big * BigUint::from(FEE_SCALE),
            BigUint::from(FEE_SCALE - fee_bps),
        );
        prop_assume!(merge <= BigUint::from(smaller / 4));
        prop_assume!(merge < BigUint::from(no));

        let quote = sell_for_exact_collateral(pool, Outcome::Yes, Uint128::new(requested), fee_bps).unwrap();
        let ending_selected = ceil_big(
            BigUint::from(yes) * BigUint::from(no),
            BigUint::from(no) - &merge,
        );
        let input = &merge + ending_selected - BigUint::from(yes);

        prop_assert_eq!(quote.gross_collateral.u128(), merge.to_u128().unwrap());
        prop_assert_eq!(quote.fee.u128(), (&merge - &requested_big).to_u128().unwrap());
        prop_assert_eq!(quote.outcome_in.u128(), input.to_u128().unwrap());
        prop_assert!(quote.reserves_after.product().unwrap() >= pool.product().unwrap());
    }
}
