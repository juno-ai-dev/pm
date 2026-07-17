use std::collections::BTreeSet;

use serde_json::Value;

fn fields(schema: &Value, response: &str) -> BTreeSet<String> {
    schema["responses"][response]["properties"]
        .as_object()
        .unwrap_or_else(|| panic!("missing response schema: {response}"))
        .keys()
        .cloned()
        .collect()
}

fn hardcoded_emitted_actions(source: &str) -> BTreeSet<String> {
    let compact: String = source.chars().filter(|ch| !ch.is_whitespace()).collect();
    [".add_attribute(\"action\",\"", "complete_set_event(\""]
        .into_iter()
        .flat_map(|marker| {
            compact
                .split(marker)
                .skip(1)
                .map(|tail| tail.split('"').next().unwrap().to_string())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn expected(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}

#[test]
fn every_v1_query_response_has_an_explicit_golden_field_set() {
    let schema: Value = serde_json::from_str(include_str!("../schema/binary-market.json")).unwrap();
    let goldens: &[(&str, &[&str])] = &[
        (
            "config",
            &[
                "protocol_version",
                "factory",
                "creator",
                "initial_lp",
                "oracle",
                "verdict_authority",
                "tier",
                "collateral_denom",
                "close_ts",
                "opening_ts",
                "initial_liquidity",
                "oracle_bounty",
                "oracle_initial_bond",
                "answer_timeout_secs",
                "arbitration_timeout_secs",
                "fee_bps",
                "min_trade",
                "max_trade_bps",
                "max_position_per_side",
                "collateral_cap",
                "challenge_bond",
            ],
        ),
        (
            "identity",
            &[
                "protocol_version",
                "factory",
                "market",
                "nonce",
                "question_id",
            ],
        ),
        ("state", &["status", "activated", "challenge_used"]),
        (
            "accounting",
            &[
                "principal",
                "fees",
                "challenge",
                "terminal_liability_twice",
                "total_yes",
                "total_no",
                "lp_supply",
                "lp_burned",
                "lp_paid",
                "neutral_half_dust",
                "lp_accrual",
                "principal_at_resolution",
                "fees_at_resolution",
                "pool_yes_at_resolution",
                "pool_no_at_resolution",
                "total_yes_at_resolution",
                "total_no_at_resolution",
            ],
        ),
        ("pool", &["yes", "no"]),
        ("position", &["address", "yes", "no"]),
        (
            "lp_position",
            &["owner", "supply", "burned", "paid", "later_accrual"],
        ),
        (
            "challenge",
            &[
                "challenger",
                "answer",
                "answer_hex",
                "answer_base64",
                "oracle_bond",
                "challenge_bond",
                "started_at",
                "deadline",
                "oracle_snapshot",
            ],
        ),
        (
            "resolution",
            &[
                "answer",
                "answer_hex",
                "answer_base64",
                "payout",
                "height",
                "time",
                "principal_at_resolution",
                "terminal_liability_twice",
                "pool_yes_at_resolution",
                "pool_no_at_resolution",
                "total_yes_at_resolution",
                "total_no_at_resolution",
            ],
        ),
        (
            "question",
            &[
                "text",
                "hash",
                "hash_hex",
                "hash_base64",
                "nonce",
                "question_id",
                "oracle",
                "opening_ts",
                "close_ts",
                "yes_answer_hex",
                "yes_answer_base64",
                "no_answer_hex",
                "no_answer_base64",
                "invalid_answer_hex",
                "invalid_answer_base64",
                "unresolved_answer_hex",
                "unresolved_answer_base64",
            ],
        ),
        (
            "solvency",
            &[
                "height",
                "block_time",
                "bank_balance",
                "principal_or_terminal_liability",
                "fee_liability",
                "challenge_liability",
                "lp_whole_coin_accrual",
                "accounted_liability",
                "forced_excess",
                "shortfall",
                "solvent",
            ],
        ),
        (
            "quote_buy",
            &[
                "height",
                "block_time",
                "outcome",
                "gross",
                "net",
                "fee",
                "input",
                "output",
                "reserve_yes_before",
                "reserve_no_before",
                "reserve_yes_after",
                "reserve_no_after",
                "average_price",
                "marginal_before",
                "marginal_after",
                "fee_rate",
                "price_impact",
                "impact_direction",
                "min_out",
                "max_in",
            ],
        ),
        (
            "quote_sell",
            &[
                "height",
                "block_time",
                "outcome",
                "gross",
                "net",
                "fee",
                "input",
                "output",
                "reserve_yes_before",
                "reserve_no_before",
                "reserve_yes_after",
                "reserve_no_after",
                "average_price",
                "marginal_before",
                "marginal_after",
                "fee_rate",
                "price_impact",
                "impact_direction",
                "min_out",
                "max_in",
            ],
        ),
    ];

    assert_eq!(
        schema["responses"].as_object().unwrap().len(),
        goldens.len()
    );
    for (response, names) in goldens {
        assert_eq!(fields(&schema, response), expected(names), "{response}");
    }
}

#[test]
fn emitted_v1_actions_are_confined_to_the_frozen_allowlist() {
    let market = include_str!("../src/contract.rs");
    let approved: BTreeSet<&str> = [
        "market_activated",
        "split",
        "merge",
        "trade",
        "challenge_requested",
        "governance_verdict_forwarded",
        "challenge_refunded",
        "challenge_slashed",
        "arbitration_stalled",
        "market_resolved",
        "positions_redeemed",
        "lp_redeemed",
        "lp_accrual_claimed",
    ]
    .into_iter()
    .collect();

    for action in &approved {
        assert!(
            market.contains(&format!("\"{action}\"")),
            "approved action is not emitted: {action}"
        );
    }

    for action in hardcoded_emitted_actions(market) {
        if action == "instantiate" {
            continue;
        }
        assert!(
            approved.contains(action.as_str()),
            "hardcoded emitted action is outside frozen v1 schema: {action}"
        );
    }

    let compact: String = market.chars().filter(|ch| !ch.is_whitespace()).collect();
    assert_eq!(
        compact.matches("action:&str").count(),
        1,
        "new dynamic action-bearing helpers must be added to this exhaustive golden parser"
    );
}

#[test]
fn action_extractor_covers_dynamic_complete_set_helper_calls() {
    let source = r#"
        complete_set_event(
            "brand_new_action",
            &env,
            config,
            &caller,
            amount,
            &accounting,
        )
    "#;
    assert_eq!(
        hardcoded_emitted_actions(source),
        BTreeSet::from(["brand_new_action".to_string()])
    );
}
