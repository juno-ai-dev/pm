# Changelog

## Unreleased

### Added

- Explicit immutable `juno1` and `uni7` contract profiles. The uni-7 profile binds market collateral, oracle bonds, canonical question documents, question IDs, settlement checks, and payouts to `ujunox`; the production profile remains bound to `ujuno`.
- Additive `contract_profile` and `collateral_denom` fields on binary-market identity responses, factory market records, and canonical `juno_pm_v1` events. Existing event type and action names are unchanged.
