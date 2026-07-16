# Binary-market invariant coverage

This matrix binds the 20 accepted invariants in
`docs/prediction-market/security-and-economics.md` to executable evidence. Test
names are stable review handles. `state_machine_model.rs` is an independent
`BigUint` model: it does not import the production math module, compares every
implemented pre-resolution transition with `cw-multi-test`, checks rejection
atomicity, and prints the shrunk action trace in addition to proptest's seed.

| # | Invariant | Executable coverage |
|---:|---|---|
| 1 | Complete sets | `arbitrary_precision_model_matches_every_success_and_rejection`; `activation_split_merge_and_cap_boundaries_reconcile` |
| 2 | Coverage | `arbitrary_precision_model_matches_every_success_and_rejection` checks bank = P + F + forced excess after every action; `assert_reconciles` in `trading.rs` |
| 3 | Terminal conservation | `valid_redemption_pays_only_winners_and_debits_t2_twice`; `neutral_seeded_partitions_never_outpay_aggregate_and_abandoned_units_remain` |
| 4 | Positive pool | `arbitrary_precision_model_matches_every_success_and_rejection`; `rejects_boundaries_without_saturation_or_zero_denominators` |
| 5 | Product direction | independent `buy_matches_arbitrary_precision_reference` / `sell_matches_arbitrary_precision_reference`; `repeated_calls_never_decrease_product` |
| 6 | Slippage symmetry | `documented_buy_sell_vector_matches_quote_and_ledgers`; `stale_quote_slippage_and_failure_guards_are_atomic`; `stale_sell_quote_enforces_max_input_atomically` |
| 7 | Fee conservation | `arbitrary_precision_model_matches_every_success_and_rejection`; `seeded_random_trade_sequences_reconcile_with_forced_funds` |
| 8 | Principal separation | state-model bank/P/F comparison; `forced_funds_and_seeded_random_sequences_do_not_create_claims` |
| 9 | Close | generated `Close` transitions in the state model; `close_and_deadline_boundaries_are_exact`; `split_rejects_bad_amount_funds_and_close_without_state_change` |
| 10 | Resolution binding | `exact_yes_no_and_every_noncanonical_class_map_without_transfers`; `finality_boundary_is_exact_and_failed_query_leaves_resolution_empty` |
| 11 | Redemption safety | `redemption_rejects_empty_overdraw_attached_funds_and_repeated_claims`; `failed_redemption_send_rolls_back_burn_and_allows_exact_retry` |
| 12 | LP subordination | `neutral_half_dust_pairs_to_lp_and_address_splitting_cannot_help_callers` verifies that only finalized paired dust reaches LP accrual; no implemented execute path can debit user backing for an LP |
| 13 | Bounded arithmetic | both arbitrary-precision quote properties; state-model boundary-biased amounts; `rejects_boundaries_without_saturation_or_zero_denominators` |
| 14 | Dust ownership | `fee_extremes_are_checked_and_caller_adverse`; `neutral_half_dust_pairs_to_lp_and_address_splitting_cannot_help_callers` |
| 15 | Path independence | `aggregate_and_partitioned_complete_sets_are_path_independent`; `neutral_partial_partitions_equal_aggregate_and_finalize_odd_half` |
| 16 | Challenge segregation | `no_funds_actions_reject_attached_coins_before_stub` and `governance_is_exact_sender_and_strictly_before_deadline` cover the current fail-closed boundary. Full C release/slash model coverage is added with issue #13, when those successful transitions exist. |
| 17 | Verdict authorization | `governance_is_exact_sender_and_strictly_before_deadline` |
| 18 | Immutability | `instantiate_validation_rejects_bad_boundaries`; schema has no config mutation or migration execute surface; oracle/market multi-test instantiations use no admin |
| 19 | Cap | generated split/buy amounts around and above cap in the state model; `activation_split_merge_and_cap_boundaries_reconcile` |
| 20 | No sweep | state-model forced-fund bank equality; `forced_funds_and_seeded_random_sequences_do_not_create_claims`; the execute schema contains no sweep/admin action |

## Model extension contract

Issue #13 (challenge) and issue #15 (LP terminal claims) add successful state
transitions that do not exist at this commit. Their PRs must extend the same
action generator/model and replace the fail-closed/vacuous rows above with
success, rejection, and rollback comparisons. This file deliberately does not
claim that unimplemented transitions were exercised.
