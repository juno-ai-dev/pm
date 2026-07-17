# Current-main lifecycle assurance map

**Scope:** issue #19 assurance on current main after LP settlement issue #15 and before
factory issue #17.
This map names authoritative tests instead of duplicating their fixtures. Unless noted,
paths are under `contracts/binary-market/tests/`; cw-reality names are unit tests under
`contracts/cw-reality/src/tests.rs`.

## On-chain acceptance coverage

| User-journey acceptance | Passing evidence |
| --- | --- |
| Market asks cw-reality, verifies the exact reply, activates, and rolls back a rejected nested Ask | `question_activation::instantiate_asks_and_exact_reply_activates_atomically`; `question_activation::rejected_oracle_ask_rolls_back_question_and_funds` |
| Current factory boundary requires the configured factory sender and exact single-denom funds | `market_boundary::instantiate_rejects_wrong_no_multiple_funds_and_sender` |
| Buy/sell worked vector, both outcomes, fees, direct bank/state/event reconciliation | `trading::documented_buy_sell_vector_matches_quote_and_ledgers`; `trading::both_outcomes_trade_symmetrically`; `lifecycle_assurance::three_market_yes_no_neutral_lifecycles_reconcile_and_never_cross_contaminate` |
| Slippage, cap, amount, wrong-denom, attached-funds and failed-send rejection are atomic | `trading::stale_quote_slippage_and_failure_guards_are_atomic`; `trading::stale_sell_quote_enforces_max_input_atomically`; `trading::failed_sell_bank_send_rolls_back_all_ledgers`; `complete_sets::split_rejects_bad_amount_funds_and_close_without_state_change`; `complete_sets::merge_works_after_close_and_failed_bank_send_rolls_back` |
| Trading and Split reject exactly at close; a time jump cannot reopen them; Merge remains available before resolution | `trading::close_and_deadline_boundaries_are_exact`; `market_boundary::trading_rejects_exact_close_boundary_and_deadline_is_inclusive`; `complete_sets::merge_works_after_close_and_failed_bank_send_rolls_back` |
| Exact YES, exact 32-byte NO, INVALID, UNRESOLVED, short zero and arbitrary bytes resolve once to the specified payout | `resolution::exact_yes_no_and_every_noncanonical_class_map_without_transfers`; `lifecycle_assurance::three_market_yes_no_neutral_lifecycles_reconcile_and_never_cross_contaminate` |
| Resolve fails one second before finality without mutation and succeeds at finality | `resolution::finality_boundary_is_exact_and_failed_query_leaves_resolution_empty` |
| Position redemption pays/burns exactly once; neutral partitions, half-dust and address splitting conserve value | `redemption::valid_redemption_pays_only_winners_and_debits_t2_twice`; `redemption::neutral_partial_partitions_equal_aggregate_and_finalize_odd_half`; `redemption::neutral_half_dust_pairs_to_lp_and_address_splitting_cannot_help_callers`; `redemption::redemption_rejects_empty_overdraw_attached_funds_and_repeated_claims` |
| A failed redemption BankMsg rolls back burns and permits an exact retry | `resolution::failed_redemption_send_rolls_back_burn_and_allows_exact_retry` |
| YES, NO, and neutral resolution snapshots freeze the LP pool; partial/final immutable-LP burns pay only proportional frozen pool value plus fees | `lifecycle_assurance::three_market_yes_no_neutral_lifecycles_reconcile_and_never_cross_contaminate`; `lp_redemption::yes_no_and_neutral_pay_only_frozen_pool_value_and_fees`; `lp_redemption::partial_burns_equal_aggregate_with_cumulative_position_and_fee_floors` |
| A failed LP BankMsg rolls back LP units, fees, pool positions, and terminal liability | `lp_redemption::bank_send_failure_rolls_back_lp_burn_fee_and_position_debits` |
| Neutral pool/user half-dust can pair after every base LP unit is burned and remains claimable only by the immutable LP | `lifecycle_assurance::neutral_half_dust_accrues_and_claims_only_after_full_base_lp_burn`; `lp_redemption::odd_pool_numerator_uses_shared_dust_and_late_accrual_survives_full_burn` |
| Counter-answer must double the bond, resets finality, and direct final-answer queries enforce current bond/denom | `cw-reality::tests::dispute_answer_below_double_rejected`; `cw-reality::tests::dispute_answer_doubling_happy_path`; `cw-reality::tests::query_final_answer_before_finalize_errors`; `cw-reality::tests::query_final_answer_if_matches_fails_min_bond`; `cw-reality::tests::query_final_answer_if_matches_fails_denom_mismatch` |
| Unanswered questions cannot finalize; later valid first answer starts normal finality | `cw-reality::tests::request_arbitration_without_answer_rejected`; `cw-reality::tests::query_final_answer_before_finalize_errors`; `cw-reality::tests::submit_answer_first_round_happy_path` |
| Answerer history claims and withdrawals conserve bond/bounty credits | `cw-reality::tests::claim_three_round_dispute_balances`; `cw-reality::tests::claim_round_by_round_equals_all_at_once`; `cw-reality::tests::withdraw_drains_balance`; `cw-reality::proptests::claim_conserves_bond_total` |
| Challenge uses the dynamic oracle bond and keeps challenge liability separate | `arbitration::challenge_funding_matrix_and_dynamic_oracle_bond_are_exact`; `arbitration::authority_is_exact_immutable_and_different_verdict_refunds_without_contamination` |
| Only the immutable DAO core may verdict; wrong caller/funds/question/payee/state and replay fail before mutation | `arbitration::spoofed_callers_funds_bad_payload_state_and_deadline_fail_before_mutation`; `arbitration::verdict_deadline_finalize_funds_replay_and_pending_snapshot_matrix` |
| Different verdict refunds; identical verdict and timeout slash once; noncanonical verdict resolves neutral | `arbitration::authority_is_exact_immutable_and_different_verdict_refunds_without_contamination`; `arbitration::identical_verdict_slashes_to_lp_and_noncanonical_resolves_neutral`; `arbitration::exact_arbitration_events_cover_refund_identical_and_timeout_slash` |
| A timeout challenge slash remains a separate immutable-LP accrual after full base-LP burn and forced funds remain excess | `lifecycle_assurance::challenge_slash_stays_claimable_after_full_lp_burn_and_forced_funds_stay_excess`; `arbitration::timeout_slash_is_claimable_after_resolution_and_full_base_lp_burn` |
| Nested Request/Submit/Cancel failures and corrupt replies roll back market, oracle and bank state and are retryable | `arbitration::nested_oracle_failures_and_reply_verification_failures_are_atomic_and_retryable` |
| Verdict at the exact deadline loses to timeout; direct cancellation synchronizes without double slash or a second challenge | `arbitration::verdict_deadline_finalize_funds_replay_and_pending_snapshot_matrix`; `arbitration::timeout_and_direct_cancellation_synchronize_once_and_second_challenge_stays_rejected` |
| Forced ujuno changes only direct bank/solvency excess, including after redemption; random operations reconcile | `lifecycle_assurance::three_market_yes_no_neutral_lifecycles_reconcile_and_never_cross_contaminate`; `trading::seeded_random_trade_sequences_reconcile_with_forced_funds`; `complete_sets::forced_funds_and_seeded_random_sequences_do_not_create_claims` |
| Multiple markets sharing one oracle cannot cross-contaminate accounting or positions | `lifecycle_assurance::three_market_yes_no_neutral_lifecycles_reconcile_and_never_cross_contaminate` |
| Chain admins are empty; market wire protocol has no pause/sweep; migration is rejected | `lifecycle_assurance::frozen_contracts_and_closed_wire_protocol_have_no_admin_migrate_pause_or_sweep` |
| Event-derived trade and redemption amounts equal direct Accounting/Position queries | `lifecycle_assurance::three_market_yes_no_neutral_lifecycles_reconcile_and_never_cross_contaminate`; arbitration event reconciliation is in `arbitration::exact_arbitration_events_cover_refund_identical_and_timeout_slash` |

After resolution, `Solvency.principal_liability` uses the landed #15 ceiling formula
`(terminal_liability_twice + 1) / 2`, not the immutable principal-at-resolution
snapshot. The lifecycle harness reconciles that liability, fees, challenge funds, LP
accrual, direct bank balance, and forced excess after every partial/final exit. The
worked YES/NO/neutral vector proves exactly 105 JUNO is distributed between the pool
plus fee LP entitlement, user terminal entitlement, and any paired half-dust accrual.

## Explicit follow-up (not claimed by this slice)

- **#17 Factory:** `Factory.CreateMarket`, tier validation, adminless child
  instantiation as performed by the real factory, activated-market registry, multiple
  factory-created markets, spoofed factory inputs, and nested factory/market/oracle
  rollback. The harness calls the already-implemented market factory boundary
  directly and does not fake registry behavior.
- **#16/#23 query and indexer:** replaying a complete external event projection and
  reconciling it across finalized RPC heights belongs to the complete query/event
  surface and replayable indexer. Current tests require no RPC and reconcile emitted
  financial attributes directly against contract queries and bank state.
- **#21/#24 UI:** exact-byte previews, stale/disagreeing RPC UX, local-time refusal,
  warnings, unlisting, and transaction-construction policy are off-chain client/UI
  acceptance.
- **#25 Ops:** unanswered alerts, keeper monitoring, and DAO proposal lifecycle
  classification are off-chain operations; permissionless on-chain calls and clean
  rejection are covered above.
- **#3/#4/#13/#26 deployment/governance gates:** reproducible live artifact evidence,
  future x/gov compatibility/rehearsal, legal/security approval, and canary launch are
  explicitly outside cw-multi-test.

Until #17 lands, this work **Tracks #19** and does not satisfy all of #19's acceptance
criteria.
