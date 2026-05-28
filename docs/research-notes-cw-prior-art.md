---
title: cw-reality — CosmWasm prior-art catalogue
author: Juno (workspace agent)
date: 2026-05-28
scope: Research notes for the `cw-reality` contract — a bond-escalating crowdsourced oracle (CosmWasm port of Reality.eth) targeting juno-1.
---

# cw-reality — CosmWasm prior-art catalogue

The new contract needs:

- A `Map<QuestionId, QuestionState>` state machine
- Native + cw20 + (eventually) IBC bond escrow
- cw-filter integration for `answer_schema` validation
- An optional arbitrator slot (`Option<Addr>`)
- Loser-bond redistribution math (Reality.eth's bonded escalation game)
- Reproducible optimised-wasm build

The catalogue below maps each in-repo pattern that already solves a sub-problem of this contract, with explicit "copy / adapt / skip" verdicts and the file paths to bookmark in `cw-reality`'s implementation.

All paths are absolute and refer to checkouts inside `/workspace/`. Several memory files referenced by the task brief have alternate locations or different scope than the brief implied — those are noted inline.

---

## 1. dao-proposal-single deposit handling

**What the pattern is.** dao-proposal-single does *not* implement deposit handling itself — deposit lifecycle lives in `dao-pre-propose-base`, a generic pre-propose contract that the single-choice proposal module instantiates as its `ProposalCreationPolicy::Module`. The flow is:

1. **Instantiate:** `UncheckedDepositInfo` → `into_checked` validates denom (native via `into_checked` parse, cw20 via a `TokenInfo {}` query) and rejects zero amounts. Stored once in `Config.deposit_info: Option<CheckedDepositInfo>`.
2. **Take on propose:** native deposits are pushed (`check_native_deposit_paid` asserts `must_pay(info, denom) == amount`); cw20 deposits are pulled (the contract emits a `Cw20ExecuteMsg::TransferFrom { owner: depositor, recipient: contract, amount }`). The proposer + deposit-info tuple is snapshotted under `self.deposits: Map<u64, (Option<CheckedDepositInfo>, Addr)>` keyed by `proposal_id`. The proposal id is obtained *before* creating the proposal via `dao_interface::proposal::Query::NextProposalId {}` — important for the order of messages.
3. **Return on completion:** the proposal module fires a `ProposalCompletedHook { proposal_id, new_status }` back to the pre-propose contract. The hook looks up the snapshotted deposit, applies the refund policy (`Always` / `OnlyPassed` / `Never`), and either returns the deposit to the proposer (`get_return_deposit_message`) or to the DAO (forfeiture path).

**Files / line ranges (canonical).**

- `/workspace/dao-contracts/packages/dao-voting/src/deposit.rs` — the data types and helpers:
  - `UncheckedDepositInfo` / `CheckedDepositInfo` / `DepositRefundPolicy` (lines 39–93)
  - `into_checked` validation (lines 94–152)
  - `check_native_deposit_paid` (lines 155–179) — native push-payment assertion
  - `get_take_deposit_messages` (lines 181–214) — cw20 `TransferFrom` builder
  - `get_return_deposit_message` (lines 216–223) — refund message builder
- `/workspace/dao-contracts/packages/dao-pre-propose-base/src/execute.rs` — orchestration:
  - `execute_propose` deposit-take (lines 131–190)
  - `execute_proposal_completed_hook` refund branch (lines 385–452)
- `/workspace/dao-contracts/contracts/pre-propose/dao-pre-propose-single/src/contract.rs` — trivial wrapper; the substantive code is in `dao-pre-propose-base`.
- Memory note `/workspace/memory/dao-proposal-single-patterns.md` exists and confirms the wiring — written for the dao-proposal-wavs port but mostly applies verbatim.

**Excerpt — the asymmetric native/cw20 take pattern** (`deposit.rs:181–214`):

```rust
pub fn get_take_deposit_messages(&self, depositor: &Addr, contract: &Addr) -> StdResult<Vec<CosmosMsg>> {
    let take_deposit_msg: Vec<CosmosMsg> = if let Self {
        amount, denom: CheckedDenom::Cw20(address), ..
    } = self {
        vec![WasmMsg::Execute {
            contract_addr: address.to_string(),
            funds: vec![],
            msg: to_json_binary(&cw20::Cw20ExecuteMsg::TransferFrom {
                owner: depositor.to_string(),
                recipient: contract.to_string(),
                amount: *amount,
            })?,
        }.into()]
    } else {
        // Deposits are pushed, not pulled for native deposits.
        // See: `check_native_deposit_paid`.
        vec![]
    };
    Ok(take_deposit_msg)
}
```

**Verdict — adapt with tweaks.** Reuse `CheckedDepositInfo` / `CheckedDenom` and the two helper functions directly via the existing `dao-voting` and `cw-denom` crates. cw-reality's bond model is materially different from a *one-shot deposit refunded once*: bonds escalate per answer, multiple bonders coexist per question, losers' bonds are redistributed to winners (not forfeited to the DAO). So we should:

- Copy: the `Native(push) | Cw20(pull-via-TransferFrom)` asymmetry, the `must_pay` + amount-match assertion, and the `get_take_deposit_messages` / `get_return_deposit_message` API shape.
- Adapt: replace the single `Option<CheckedDepositInfo>` in `Config` with per-question `bond_token: CheckedDenom` and per-answer `Bond { bonder: Addr, amount: Uint128 }` records under `BONDS: Map<(QuestionId, AnswerNonce), Bond>`. Refund-policy logic becomes a bond-redistribution sweep over the answer history when the question finalises.
- Skip: the `DepositRefundPolicy` enum — Reality.eth's payout rule is fixed by the protocol (winning answer's bonders get their bonds back + losers' bonds prorated by stake). No user-configurable refund policy.

For IBC bonds: extend `CheckedDenom` to include an IBC-voucher variant, or compose: hold IBC vouchers as native denoms (which is what they already are at the bank-module level — `ibc/...` denoms are bank-native on the receiving chain).

---

## 2. cw-abc bond accounting

**What the pattern is.** cw-abc is an Augmented Bonding Curve — buyers deposit a reserve denom and receive token-factory-minted curve-priced tokens; sellers burn tokens for reserve. The accounting is dual-pool: `reserve` (locked against burning), `funding` (project treasury) and `supply` (issued tokens). Curve math computes `(new_reserve, new_supply, funded)` from each buy/sell quote in pure functions.

**Branch & file paths.** The contract is on the `origin/cw-abc` branch of `dao-contracts` (not on `feat/gauges` which is currently checked out). Paths reachable via `git show origin/cw-abc:contracts/external/cw-abc/src/...`:

- `contracts/external/cw-abc/src/state.rs` — storage shape
- `contracts/external/cw-abc/src/abc.rs` — curve types, phase config
- `contracts/external/cw-abc/src/commands.rs` — `buy` / `sell` handlers (the accounting hot path)
- `contracts/external/cw-abc/src/helpers.rs` — `calculate_buy_quote` / `calculate_sell_quote`
- `contracts/external/cw-abc/src/test_tube/` — integration tests via test-tube (real-VM)

Memory note `/workspace/memory/cw-abc-branch.md` is present and accurate (the branch is ~2 years stale; tracked as a rebase target, not currently buildable on `development`).

**State excerpt** (`state.rs:10–24`):

```rust
#[cw_serde]
pub struct CurveState {
    /// reserve is how many native tokens exist bonded to the validator
    pub reserve: Uint128,
    /// funding is how many native tokens exist unbonded and in the contract
    pub funding: Uint128,
    /// supply is how many tokens this contract has issued
    pub supply: Uint128,
    /// the denom of the reserve token
    pub reserve_denom: String,
    /// how to normalize reserve and supply
    pub decimals: DecimalPlaces,
}
```

Per-user accounting is tracked via `HATCHERS: Map<&Addr, Uint128>` (hatch-phase contributions) and `DONATIONS: Map<&Addr, Uint128>`. The hot-path invariant for `buy` (`commands.rs:19–124`) is: every payment is `must_pay`'d against the reserve denom, the quote is computed, then `curve_state.{supply, reserve}` are updated and the (optionally forwarded) `funded` cut is split. All three invariants — `supply` monotone, `reserve` monotone non-negative, `funded ≤ payment` — fall out of the quote math, not run-time asserts.

**No property tests.** The brief asks about "property-test patterns" on cw-abc. There are none — cw-abc's tests are example-based (`src/testing.rs` unit tests + `src/test_tube/` integration tests). I grepped the entire `dao-contracts` workspace; **`proptest` is not used anywhere** in dao-contracts. The only proptest consumers in `/workspace` are `astroport-core/contracts/pair*` and `cosmwasm/packages/std`. If we want property tests for cw-reality, we are establishing the convention in dao-contracts, not following one.

**Verdict — adapt with tweaks (for the accounting shape); skip (for the curve math).** The relevant pattern for cw-reality is the *triple-bookkeeping idiom* — one `Item<GlobalAggregate>` for invariants checked on every state-mutating call, plus per-actor `Map<Addr, _>` for refund/payout, plus per-event records (here `HATCHERS`; for cw-reality, per-answer `Bond`). Copy that shape. The curve math itself (`CurveType`, `Constant/Linear/SquareRoot`) is **skip — different problem**: cw-reality's bond escalation is a fixed-rule doubling (typical Reality.eth deployment uses 2× minimum-bond per answer), not a continuous curve. For property tests, look outside this workspace (astroport's curve invariant tests in `/workspace/astroport-core/contracts/pair_concentrated/tests/` are the closest in-repo example of property-testing CosmWasm state).

---

## 3. gauges-are-cool streaming / payout math

**What the pattern is.** The brief implies "per-block accrual with lazy checkpointing and an O(1)-per-claim primitive." That is *not* what the gauges contracts actually do — and reading the memory note carefully, it isn't claimed either. The gauges streaming primitive is:

- **Discrete epochs** (e.g. `epoch: 7 * 86400` seconds). No per-block accrual; the gauge has a `next_epoch: u64` field and `Execute { gauge_id }` is only callable when `env.block.time.seconds() >= gauge.next_epoch`.
- **Delta-only updates to a sorted tally.** Two parallel maps form a sorted index: `TALLY: Map<(GaugeId, &str), u128>` (primary, by option) and `OPTION_BY_POINTS: Map<(GaugeId, u128, &str), u8>` (secondary, *(power, option)* — natural sorted order via the composite key). Voters' updates only touch their own slot; the tally aggregate is incrementally maintained, so reading "top N options" at execute time is O(N), not O(total voters).
- **No per-voter accrual / claim flow.** Distribution happens once per epoch: `selected_set` is the top-N options by tally, ratios are computed (`Decimal::from_ratio(power, sum)`), and the adapter is queried for concrete `CosmosMsg`s that dispatch from the DAO core's treasury. There is no "lazy claim" — recipients are paid in the same tx that closes the epoch.

**Locations.**

- `/workspace/dao-contracts/contracts/gauges/gauge/src/state.rs` — `TALLY`, `OPTION_BY_POINTS`, `TOTAL_CAST` (lines 289–294); `update_tallies` (lines 333–365) — the delta primitive.
- `/workspace/dao-contracts/contracts/gauges/gauge/src/contract.rs` — `execute::execute` epoch tick (lines 849–906): tally → ratios → adapter query → DAO `ExecuteProposalHook`.
- `/workspace/dao-contracts/contracts/gauges/gauge-adapter/` — the marketing-gauge adapter (per-submission payouts).
- `/workspace/dao-contracts/contracts/gauges/budget-allocator/` — minimal proportional payout adapter (no submissions; admin-curated option set).
- Memory note `/workspace/memory/gauges-are-cool-branch.md` — present and current. The "O(N) cost per claim" framing in the task brief doesn't appear there; the actual claim is about sorted-index secondary keys + delta updates.

**Excerpt — the delta-update primitive that keeps the sorted index consistent** (`state.rs:333–365`):

```rust
pub fn update_tallies(
    storage: &mut dyn Storage,
    gauge: GaugeId,
    updates: Vec<(&str, u128, u128)>, // (option, old_vote, new_vote)
) -> StdResult<()> {
    let mut old_votes = 0u128;
    let mut new_votes = 0u128;
    for (option, old_vote, new_vote) in updates {
        old_votes += old_vote;
        new_votes += new_vote;
        let old_count = TALLY.may_load(storage, (gauge, option))?;
        let count = old_count.unwrap_or_default() + new_vote - old_vote;
        TALLY.save(storage, (gauge, option), &count)?;
        if let Some(old) = old_count {
            OPTION_BY_POINTS.remove(storage, (gauge, old, option));
        }
        OPTION_BY_POINTS.save(storage, (gauge, count, option), &1u8)?;
    }
    let total = TOTAL_CAST.may_load(storage, gauge)?.unwrap_or_default();
    let total = total + new_votes - old_votes;
    TOTAL_CAST.save(storage, gauge, &total)
}
```

**Verdict — adapt with tweaks.** Reality.eth's bond escalation has the same shape: each `SubmitAnswer` is a delta on the question's "current leading answer" record. We do not want to iterate the whole answer history every read; we want a sorted index `LEADING_ANSWER_BY_QUESTION: Map<QuestionId, (Hash, Uint128)>` updated incrementally as each escalation lands. The two takeaways:

- **Copy the delta primitive.** A `Map<(QuestionId, AnswerHash), Bond>` plus a single `Item<LeadingAnswer>` (or `Map<QuestionId, LeadingAnswer>`) updated on every `SubmitAnswer` is the cw-reality analogue of `TALLY` + `OPTION_BY_POINTS`. Don't compute the leading answer by iterating the answer history at read time.
- **Adapt the epoch tick.** cw-reality's "epoch" is the question timeout reset — every accepted higher-bond answer pushes `finalisation_at` forward. The `next_epoch` field in `Gauge` is the structural analogue.
- **Skip:** the `Decimal::from_ratio` proportional payout — Reality.eth's payout to winners is *not* proportional to bond ratios within the winning answer; it is the protocol's specific loser-bond split rule. The math is sui generis.

---

## 4. dao-proposal-wavs cw-filter integration

**What the pattern is.** dao-proposal-wavs treats cw-filter as a stateless validator. On every `WavsHandleSignedEnvelope`, after the envelope verifies and the payload decodes, the contract iterates each `CosmosMsg` in the payload and dispatches a `Filter { filter, msg }` smart query to the cw-filter contract address stored in `Config.mandate_filter`. The response is a three-way enum — `Pass {}` continues, `Fail { reason }` rejects this proposal cleanly, `Fatal { reason }` is treated as protocol corruption and also rejects (the contract maps both to dedicated `ContractError` variants that surface the upstream reason).

**Files.**

- `/workspace/contracts/dao-proposal-wavs/src/contract.rs:441–461` — the call site.
- `/workspace/contracts/dao-proposal-wavs/src/filter.rs` — local wire-format mirror (`FilterQueryMsg`, `FilterResponse`). The `cw-filter` crate is *not* imported as a direct dep because its transitive `cw-jsonfilter` pulls in `alloy-rpc-types-eth` that conflicts at compile time. cw-reality should do the same: mirror the wire types.
- `/workspace/dao-contracts/contracts/external/cw-filter/src/msg.rs` — canonical wire types (`QueryMsg::Filter`, `FilterResponse::{Pass, Fail, Fatal}`).
- `/workspace/dao-contracts/contracts/external/cw-filter/src/contract.rs:135–168` — `query_filter` implementation. Importantly, **the filter is a contract instance plus a filter-spec JSON** — the spec is passed in the query and validated by `CwJsonFilter` against the JSON-serialised msg.
- Memory note `/workspace/memory/wavs-proposal-module.md` covers the full design including the v0.3.0 wire-format reconciliation. The cw-filter integration section there matches the implementation.
- **The standalone `/workspace/cw-filter` directory referenced in the brief does not exist.** The cw-filter contract lives in `dao-contracts/contracts/external/cw-filter/` only.

**Excerpt — the call shape we will mirror in cw-reality** (`dao-proposal-wavs/src/contract.rs:441–461`):

```rust
if let Some(filter_cfg) = &cfg.mandate_filter {
    for (i, msg) in payload.msgs.iter().enumerate() {
        let resp: FilterResponse = deps.querier.query_wasm_smart(
            filter_cfg.filter_contract.as_str(),
            &FilterQueryMsg::Filter {
                filter: filter_cfg.filter.clone(),
                msg: msg.clone(),
            },
        )?;
        match resp {
            FilterResponse::Pass {} => continue,
            FilterResponse::Fail { reason } => {
                return Err(ContractError::MandateFilterFail { index: i, reason });
            }
            FilterResponse::Fatal { reason } => {
                return Err(ContractError::MandateFilterFatal { index: i, reason });
            }
        }
    }
}
```

The local `FilterResponse` mirror (`dao-proposal-wavs/src/filter.rs`):

```rust
#[cw_serde]
pub enum FilterResponse {
    Pass {},
    Fail { reason: String },
    Fatal { reason: String },
}
```

**Verdict — copy directly.** This is the cleanest pattern in the workspace and it maps 1-to-1 to cw-reality's `answer_schema` validation:

- cw-reality `Config.answer_schema_filter: Option<AnswerSchemaFilterConfig>` with `{ filter_contract: Addr, filter: serde_json::Value }`.
- On `SubmitAnswer { question_id, answer_bytes, .. }` and `DisputeAnswer { ... }`: build the canonical `serde_json::Value` representation of the answer, dispatch `cw_filter::QueryMsg::Filter { filter, msg: <synthesised CosmosMsg or generic Value> }`, branch on `Pass / Fail / Fatal`.
- **Important caveat**: cw-filter's `QueryMsg::Filter` takes `msg: CosmosMsg` (not `serde_json::Value`). For cw-reality answer validation we need a `CosmosMsg`-shaped envelope or we need to add a new filter QueryMsg variant. Cheapest path: synthesise a `CosmosMsg::Custom(Empty)`-style stub with the answer bytes embedded in a memo field, *or* PR a `FilterValue { filter, value: serde_json::Value }` query variant upstream. Decide before coding; the wrapping is a 30-line decision.
- Mirror the wire types in `cw-reality/src/filter.rs` exactly as dao-proposal-wavs does — do **not** take a direct `cw-filter` crate dep (alloy conflict).
- Error-mapping convention: `Fail` and `Fatal` both reject the call, but they are distinct `ContractError` variants so off-chain consumers can distinguish a clean "wrong shape" reject from a "filter contract is broken" reject. Copy this convention.

---

## 5. cw-multi-test + property-test setup

**What the pattern is.** dao-contracts standardises on `cw-multi-test`'s `App` + `ContractWrapper` pattern via the `dao-testing` package. Each integration test:

1. Spins up an `App` (`cw_multi_test::App::default()` or `App::new(|router, _, _| ...)` for seeded balances).
2. Stores contract code via `app.store_code(box_of_wrapper)` for every contract in the dependency graph — wrapper factories live in `dao-testing/src/contracts/latest.rs` (`dao_proposal_single_contract`, `cw20_base_contract`, `cw4_group_contract`, etc.).
3. Instantiates the DAO via `dao_dao_core_contract` with a full `ModuleInstantiateInfo` payload for voting + proposal modules.
4. Drives the test through `app.execute_contract` / `app.wrap().query_wasm_smart`.

dao-proposal-single's tests are split into `testing/{instantiate, execute, do_votes, queries, adversarial_tests, migration_tests}.rs` — a clean convention.

**Files / line ranges.**

- `/workspace/dao-contracts/packages/dao-testing/src/lib.rs` — re-exports `contracts`, `helpers`, `suite`, `tests`.
- `/workspace/dao-contracts/packages/dao-testing/src/contracts/latest.rs` — the `ContractWrapper` factories. The pattern is consistent: each contract gets a `pub fn <name>_contract() -> Box<dyn Contract<Empty>>`.
- `/workspace/dao-contracts/contracts/proposal/dao-proposal-single/src/testing/instantiate.rs:26–58` — pre-propose-info setup convention.
- For the dao-proposal-wavs test pattern (closer in shape to what cw-reality will need — single-contract integration tests with mocked deps rather than the full DAO multi-contract harness), see `/workspace/contracts/dao-proposal-wavs/tests/integration.rs` — uses mock cw-filter and mock DAO core defined as inline closures via `ContractWrapper`. 15 tests covering happy + sad paths.

**Property tests:** **none in dao-contracts.** A workspace-wide grep finds `proptest` declared only in `/workspace/astroport-core/contracts/pair{,_stable,_concentrated}/Cargo.toml` and `/workspace/cosmwasm/packages/std/Cargo.toml`. Astroport's `pair_concentrated/tests/` is the in-repo reference for property-testing CosmWasm contracts (curve invariants under fuzzed swap sequences).

**Verdict — copy directly (for cw-multi-test); cross-repo (for proptest).**

- For cw-reality unit/integration tests, copy the **dao-proposal-wavs single-contract harness pattern** (mocked deps as inline closures) over the full dao-testing suite. cw-reality doesn't sit inside a DAO module model; it's a standalone oracle contract. The dao-testing helpers will be overkill and pull in 20+ deps.
- File layout: `cw-reality/tests/integration.rs` with one `App` per test, mocking cw-filter via a closure-based `ContractWrapper` that returns Pass/Fail/Fatal based on a flag in the filter spec — this is exactly what `dao-proposal-wavs/tests/integration.rs` does.
- For property tests: model after `/workspace/astroport-core/contracts/pair_concentrated/tests/` — that's the in-tree reference. cw-reality's natural invariants: total bonded value monotone before finalisation; loser-bond redistribution conserves total; bond escalation strictly increasing; finalisation timeout is monotone non-decreasing. These are textbook proptest material.
- Add `proptest = "1"` as a dev-dep when needed; we will not be following any in-repo convention because none exists in dao-contracts.

---

## 6. `workspace-optimize` Docker build pattern

**What the pattern is.** dao-contracts' `justfile` recipe for reproducible release wasm — picks `cosmwasm/optimizer-arm64:0.17.0` or `cosmwasm/optimizer:0.17.0` based on host arch, mounts the workspace into `/code`, persists the target dir + cargo registry in named docker volumes (`<pwd>_cache` + `registry_cache`).

**File / lines.** `/workspace/dao-contracts/justfile:56–67`:

```just
workspace-optimize:
    #!/bin/bash
    if [[ $(uname -m) == 'arm64' ]] || [ $(uname -m) == 'aarch64' ]]; then docker run --rm -v "$(pwd)":/code \
            --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
            --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
            --platform linux/arm64 \
            cosmwasm/optimizer-arm64:0.17.0; \
    elif [[ $(uname -m) == 'x86_64' ]]; then docker run --rm -v "$(pwd)":/code \
            --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
            --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
            --platform linux/amd64 \
            cosmwasm/optimizer:0.17.0; fi
```

**Verdict — copy directly.** Drop this verbatim into `/workspace/junoreality/justfile` (or `cw-reality/justfile`, depending on where the contract lives). The same image (`cosmwasm/optimizer:0.17.0`) is what dao-contracts' upstream CI uses, so the wasm we produce is bit-for-bit reproducible against the same optimizer image. The only required change is the optional Cargo workspace path — if cw-reality is a single-crate project (not a workspace), the optimizer image handles it the same way; if a workspace, it loops over members automatically.

**Caveat from `/workspace/memory/cw-filter-build.md`**: when Docker is unavailable, an `RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown` + `wasm-opt -Oz --signext-lowering --enable-bulk-memory --enable-reference-types` pipeline gets within ~11% of optimal size but is **not bit-for-bit reproducible**. Use Docker for any mainnet upload that we want auditors to reproduce.

---

## 7. cosmwasm-std v2 migration state

**What the pattern is.** dao-contracts is currently on `cosmwasm-std 1.5.4` (per `/workspace/dao-contracts/Cargo.toml`'s workspace deps block). The `origin/cw-std-2` branch is *aspirational* — one commit that started the migration in 2024-06 and ~51 commits behind `development`. The strategy memo (`/workspace/memory/dao-contracts-v3-upgrade-strategy.md`) recommends a three-stage path:

1. Stage 1 — bump dao-contracts to `cosmwasm-std 2.x` using published `cw-plus 2.0.0` + latest `cw-nfts`. ~10–14 days. Absorbs ~80% of breaking-change work.
2. Stage 2 — upstream cw-plus / cw-nfts to v3 in parallel. ~5–10 days. Falls back to `[patch.crates-io]` if upstream stalls.
3. Stage 3 — bump dao-contracts to `cosmwasm-std 3.x`. ~5–8 days post-Stage-2.

**Important corrections from the memory note vs the brief framing.**

- **cw-std-2 branch is not the source of truth** — `/workspace/memory/cw-std-2-migration.md` and the strategy memo both explicitly say "start clean from current `development`, do not resurrect `origin/cw-std-2`."
- **The brief asks for the "current cw-std-2.x pin."** There is no current pin — dao-contracts is still on `1.5.4`. The cw-std-2.x migration has not landed. cw-reality is a new contract that gets to pick its pin.
- **Soft coupling to Juno v30.** wasmvm v3 (the v30 target) runs cosmwasm-std v1/v2/v3 contracts unchanged. We can ship cw-reality on cosmwasm-std 1.5.4 today (matching dao-contracts), or on 2.x (matching cw-plus 2.0.0), or wait for the workspace-wide migration. There is no hard gate from chain-side.

**Files.**

- `/workspace/dao-contracts/Cargo.toml` — current pin (`cosmwasm-std = "1.5.4"` with features `ibc3`, `cosmwasm_1_2`; `cw-storage-plus = "1.1"`; `cw-multi-test = "0.20.1"`; `cw-utils = "1.0"`).
- `/workspace/memory/cw-std-2-migration.md` — full surface analysis.
- `/workspace/memory/dao-contracts-v3-upgrade-strategy.md` — three-stage strategy memo (referenced by `cw-std-2-migration.md`'s 2026-05-12 update section).
- `/workspace/contracts/dao-proposal-wavs/Cargo.toml` — what a brand-new contract in this workspace actually pins. It matches dao-contracts `1.5.4` because it ships into the same workspace.

**Verdict — adapt with tweaks (recommendation).** Pin cw-reality to **`cosmwasm-std = "1.5.4"`** to match dao-contracts current `development` and the dao-proposal-wavs precedent. Rationale:

- Lets us reuse dao-contracts crates (`dao-voting`, `cw-denom`, `dao-pre-propose-base`) without `[patch.crates-io]` gymnastics.
- Matches the workspace toolchain pin `nightly-2024-01-08` (see `/workspace/memory/feedback_juno_pinned_toolchain.md` family of memories) — `cosmwasm-std 2.x+` needs Rust 1.86+, which the dao-contracts CI does not yet have.
- Migrates cleanly when dao-contracts Stage 1 lands — same shape, just version-bump in `Cargo.toml` and chase the breaking changes documented in the migration memo.
- The cw-filter contract we depend on (only via wire types — not as a crate) is on workspace `2.8.0-alpha.2` and built against the same 1.5.4 pin (see `/workspace/memory/cw-filter-build.md`), so cross-contract queries will roundtrip cleanly.

**Gotchas to watch for once we do migrate** (from `cw-std-2-migration.md`):

- `Addr` semantics tighten (no `Addr::unchecked` in non-test code on 2.x).
- `cw-storage-plus 2.x` changes `IndexedMap` lifetime parameter (one of the changes cw-abc's commands.rs already hit).
- `cosmwasm-schema` macro output stabilises differently — `gen-schema` outputs need re-checking under CI's `git diff --exit-code`.
- The existing `cw-utils-v1` aliasing in dao-contracts is precedent for `cw-utils-v2` aliasing during incremental migration.

---

## Summary

cw-reality should be built as a standalone crate pinned to `cosmwasm-std 1.5.4` (matching dao-contracts current state and the dao-proposal-wavs precedent), with the following adoption matrix:

| Pattern | Verdict | Source |
| --- | --- | --- |
| Deposit / refund helpers | Adapt — reuse `CheckedDepositInfo` types; replace one-shot deposit semantics with per-answer bond escalation | `dao-voting/src/deposit.rs` |
| Triple-bookkeeping state shape | Adapt — global aggregate + per-actor map + per-event records | `cw-abc/src/state.rs` (`CurveState` + `HATCHERS` + `DONATIONS`) |
| Sorted-index delta updates | Adapt — `TALLY` + `OPTION_BY_POINTS` shape maps to per-question leading-answer index | `gauges/gauge/src/state.rs:289–365` |
| cw-filter call shape | Copy directly — same call, same Pass/Fail/Fatal mapping, mirror wire types locally to dodge alloy conflict | `dao-proposal-wavs/src/{contract.rs:441,filter.rs}` |
| Single-contract multi-test harness | Copy directly — closure-mocked deps, 15-test happy/sad coverage shape | `dao-proposal-wavs/tests/integration.rs` |
| Property tests | Cross-repo — no proptest in dao-contracts; model after `astroport-core/pair_concentrated` | `/workspace/astroport-core` |
| `workspace-optimize` recipe | Copy directly — same `cosmwasm/optimizer:0.17.0` Docker invocation | `dao-contracts/justfile:56–67` |
| cosmwasm-std version pin | Pin to `1.5.4` now; migrate alongside dao-contracts Stage 1 | `dao-contracts/Cargo.toml` |

The largest deliberate deviation from in-repo precedent is the bond-redistribution math itself (Reality.eth's loser-bond split rule), which has no analogue in any of these contracts — cw-abc's curve, gauges' proportional payout, and dao-proposal-single's binary refund policy all solve different problems. The bond math belongs in a dedicated `cw-reality/src/escalation.rs` module with property tests covering bond monotonicity, redistribution conservation, and finalisation-timeout monotonicity. The `cw-filter` mandate-check and `CheckedDepositInfo` deposit helpers cover the contract's entire "trust boundary" surface — everything else is bond-state-machine bookkeeping over `Map<QuestionId, QuestionState>` using the gauges delta primitive.
