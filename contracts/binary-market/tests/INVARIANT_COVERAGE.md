# Binary-market invariant coverage

This matrix binds the 20 accepted invariants in
`docs/prediction-market/security-and-economics.md` to executable evidence. Test
names are stable review handles. `state_machine_model.rs` is an independent
`BigUint` model: it does not import the production math module, compares every
implemented pre-resolution transition plus challenge, resolution, position, and
LP-terminal transitions with `cw-multi-test`, checks rejection atomicity, and
prints the shrunk action trace in addition to proptest's seed.

| # | Invariant | Executable coverage |
|---:|---|---|
| 1 | Complete sets | `arbitrary_precision_model_matches_every_success_and_rejection`; `activation_split_merge_and_cap_boundaries_reconcile` |
| 2 | Coverage | `arbitrary_precision_model_matches_every_success_and_rejection` checks bank = P + F + forced excess pre-resolution; `challenge_resolution_and_terminal_claims_match_biguint_model` checks terminal liabilities and leaves only generated forced excess after all claims |
| 3 | Terminal conservation | `challenge_resolution_and_terminal_claims_match_biguint_model` independently tracks BigUint T2 through generated YES/NO/neutral position and partitioned LP claims; `neutral_seeded_partitions_never_outpay_aggregate_and_abandoned_units_remain` |
| 4 | Positive pool | `arbitrary_precision_model_matches_every_success_and_rejection`; `rejects_boundaries_without_saturation_or_zero_denominators` |
| 5 | Product direction | independent `buy_matches_arbitrary_precision_reference` / `sell_matches_arbitrary_precision_reference`; `repeated_calls_never_decrease_product` |
| 6 | Slippage symmetry | `documented_buy_sell_vector_matches_quote_and_ledgers`; `stale_quote_slippage_and_failure_guards_are_atomic`; `stale_sell_quote_enforces_max_input_atomically` |
| 7 | Fee conservation | `arbitrary_precision_model_matches_every_success_and_rejection`; `challenge_resolution_and_terminal_claims_match_biguint_model` checks independent cumulative LP fee floors and zero final F; `seeded_random_trade_sequences_reconcile_with_forced_funds` |
| 8 | Principal separation | state-model bank/P/F comparison; `forced_funds_and_seeded_random_sequences_do_not_create_claims` |
| 9 | Close | generated `Close` transitions in the state model; `close_and_deadline_boundaries_are_exact`; `split_rejects_bad_amount_funds_and_close_without_state_change` |
| 10 | Resolution binding | `exact_yes_no_and_every_noncanonical_class_map_without_transfers`; `finality_boundary_is_exact_and_failed_query_leaves_resolution_empty` |
| 11 | Redemption safety | `redemption_rejects_empty_overdraw_attached_funds_and_repeated_claims`; `failed_redemption_send_rolls_back_burn_and_allows_exact_retry` |
| 12 | LP subordination | `challenge_resolution_and_terminal_claims_match_biguint_model` derives LP pool/fee/accrual entitlement independently while tracking user T2; `neutral_half_dust_pairs_to_lp_and_address_splitting_cannot_help_callers` |
| 13 | Bounded arithmetic | both arbitrary-precision quote properties; state-model boundary-biased amounts; `rejects_boundaries_without_saturation_or_zero_denominators` |
| 14 | Dust ownership | `fee_extremes_are_checked_and_caller_adverse`; `neutral_half_dust_pairs_to_lp_and_address_splitting_cannot_help_callers` |
| 15 | Path independence | generated two-part cumulative LP floors in `challenge_resolution_and_terminal_claims_match_biguint_model`; `aggregate_and_partitioned_complete_sets_are_path_independent`; `neutral_partial_partitions_equal_aggregate_and_finalize_odd_half` |
| 16 | Challenge segregation | `challenge_resolution_and_terminal_claims_match_biguint_model` generates no-challenge, differing-verdict refund, matching-verdict slash, and timeout-slash paths and asserts C never changes P/F and reaches LP accrual exactly once |
| 17 | Verdict authorization | generated successful exact-authority verdicts plus `spoofed_callers_funds_bad_payload_state_and_deadline_fail_before_mutation` |
| 18 | Immutability | `instantiate_validation_rejects_bad_boundaries`; schema has no config mutation or migration execute surface; oracle/market multi-test instantiations use no admin |
| 19 | Cap | generated split/buy amounts around and above cap in the state model; `activation_split_merge_and_cap_boundaries_reconcile` |
| 20 | No sweep | both state models generate forced funds; the terminal model proves all legitimate claims leave exactly the generated forced excess and a rejected post-terminal LP claim cannot mutate it |

## Model boundary

The model now spans the full implemented v1 financial lifecycle. Social oracle
correctness, gas benchmarking, live deployment, legal review, and the deferred
`x/gov` authority profile remain outside this executable accounting model.
