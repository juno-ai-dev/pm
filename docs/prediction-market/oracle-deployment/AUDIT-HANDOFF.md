# Independent audit handoff — cw-reality frozen artifact

**Audit status:** not performed; this is the required handoff contract
**Source:** `https://github.com/juno-ai-dev/pm.git` at `454f9777b0bafa71c43b427f7451e626d860269e`
**Root tree:** `e7cad35d114197fbae3cb0ff9e44ac05d0309bfa`
**Contract tree:** `cc53d2ea0aa9fcd04fe11ba910b02db11276a0b1`
**Artifact SHA-256:** pending two-builder reproducibility evidence

## In-scope code and assumptions

Review `contracts/cw-reality/**`, its locked Rust dependency graph, root release
profile, checked-in schema, and the exact optimizer-produced wasm. Treat the
CosmWasm VM/Juno host as an external dependency, but review its assumed message,
fund, address, time, and migration semantics. Historical UI, prediction-market
contracts not yet implemented, and protocol redesign are out of scope.

The intended frozen instance has no x/wasm chain admin and stores
`InstantiateMsg.admin = None`; it pins `min_initial_bond_floor = 10000000` and
`min_answer_timeout_secs = 86400`. A finding that permits migration, config
mutation, unauthorized arbitration, invalid claim payout, denom confusion,
state-machine bypass, fund loss/lock, overflow, or practical denial of finality
is deployment-blocking until explicitly resolved and retested.

## Required review coverage

- every execute/query/instantiate/migrate entry point and authorization check;
- question-ID and history-hash domain separation/canonicalization;
- native and CW20 receive paths, denom pinning, exact-fund checks, pull payments;
- bond escalation, front-run guards, timestamps, arbitration and cancellation;
- claim walk ordering, partial/resumed claims, payout conservation, withdrawal;
- all state transitions and malformed/unknown answer behavior;
- migration reachability when both admin surfaces are absent;
- dependency advisories, unsafe code/build scripts, schema/API drift;
- wasm/runtime compatibility and correspondence between reviewed source and the
  supplied artifact/build logs.

Run at minimum root validation, contract tests/proptests, strict clippy,
schema-drift check, cargo-deny, and relevant additional static/dynamic tests.
Document tool versions and commands. Do not infer source/artifact equivalence
from matching filenames or metadata: independently reproduce or verify bytes.

## Required report format

The signed or immutable report must state:

1. reviewer identity/independence and review dates;
2. exact repository commit, root/contract trees, artifact SHA-256/size, optimizer
   digest, and build-record links;
3. scope, exclusions, methodology, tools, test outputs, and limitations;
4. each finding ID, severity, affected code, exploit/impact, recommendation,
   remediation commit, retest evidence, and final disposition;
5. an explicit list of unresolved blocking and non-blocking findings;
6. whether the exact source and exact artifact are approved for the stated
   frozen parameters—without representing deployment or chain state as audited
   unless those were separately observed.

A maintainer then records the immutable report URL and exact reviewed commit/hash
in the machine manifest. `accepted` is forbidden if either binding differs or
any blocking finding remains unresolved. Audit independence and acceptance are
human evidence gates; repository tooling cannot self-attest them.
