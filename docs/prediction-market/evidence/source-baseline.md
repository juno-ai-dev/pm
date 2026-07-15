# Source baseline

**Captured:** 2026-07-15  
**Classification:** observed fact unless a row says otherwise

## Local canonical source

| Item | Pin |
| --- | --- |
| Repository base reviewed | 227edbb5e7fc472a62cb724827fec11fbaad39ae |
| Executable cw-reality handler baseline | ee641534fd7b7b3677bd48d30390422ee3fbe5ed |
| Contract package | cw-reality 0.1.0-alpha.1 |
| Reconciled combined schema SHA-256 | a50ecbb01d358daddf937bc5704fbdfdee8863be8658619f206eae132b1120af |
| Source-declared CosmWasm dependency | cosmwasm-std 1.5.4 |
| Locked/resolved CosmWasm dependency in the verification build | cosmwasm-std 1.5.11 |

Canonical files are [msg.rs](../../../contracts/cw-reality/src/msg.rs), [state.rs](../../../contracts/cw-reality/src/state.rs), [query.rs](../../../contracts/cw-reality/src/query.rs), [id.rs](../../../contracts/cw-reality/src/id.rs), and the handlers under [execute](../../../contracts/cw-reality/src/execute). The checked-in [combined schema](../../../contracts/cw-reality/schema/cw-reality.json) is generated evidence of the message surface. CI regenerates it and rejects any uncommitted difference so source commentary and schema descriptions cannot silently drift.

The command cargo test --locked was run against the package on 2026-07-15 before and after this documentation reconciliation. Both runs reported 57 passed, 0 failed, 0 ignored; doc tests 0 passed/failed. The current root validation gate also passes strict Clippy without warnings. This only verifies the behavior covered by that suite.

## Direct source observations

| Behavior | Evidence and consequence |
| --- | --- |
| Answer bytes | AnswerType is metadata. SubmitAnswer and DisputeAnswer accept opaque Binary, so a consuming market must do exact-byte interpretation itself. |
| Question ID | id.rs hashes canonical contract address, canonical asker, nonce, SHA-256 text hash, tagged optional arbitrator, answer timeout, 128-bit initial bond, length-prefixed denom, and opening timestamp. |
| Fields omitted from ID | answer_type, answer_schema, arbitration_timeout_secs, and bounty are stored but not hashed. They must be queried and compared. |
| Ask result | AskQuestion emits attributes/events but sets no response data and exposes no prediction query. Event scraping alone is not a safe binding. |
| Guarantee query | FinalAnswerIfMatches checks finality, final bond, answer timeout, arbitrator, and denom. It does not check text, opening time, answer type, answer schema, asker, arbitration timeout, code checksum, or chain migration admin. |
| Arbitration request | Only the configured arbitrator may call RequestArbitration; the question must be OpenAnswered. cw-reality collects no public challenge fee. |
| Stalled arbitration | At or after arbitration_deadline, anyone may CancelArbitration. This restarts finalize_ts at now plus answer_timeout_secs. |
| Arbitrator result | SubmitArbitration accepts any Binary and any payee accepted by `deps.api.addr_validate` while PendingArbitration. It does not prove the answer appeared in history. The payee becomes the answerer on the appended zero-bond history entry and can therefore receive oracle bounty and bond winnings. |
| Unanswered result | With finalize_ts unset, OpenUnanswered never becomes Finalized merely through time. |
| Withdrawal | Withdraw always emits a native BankMsg even though CW20 funding paths exist. Native-only use is therefore the safe v1 integration. |
| Migration | InstantiateMsg stores an optional admin and migrate performs no sender check of its own because chain-level wasmd admin authorization controls entry. Address pinning is not code immutability when a chain admin exists. |

## Arbitration documentation reconciliation

| Public description | Compiled/source behavior | Disposition |
| --- | --- | --- |
| ARBITRATION.md and msg.rs state that the arbitrator may author any Binary without a history-membership proof. | execute/arbitration.rs deliberately accepts any answer. | Consistent; the market maps unknown bytes to neutral, but this does not constrain oracle payout history. |
| ARBITRATION.md and msg.rs include the arbitrator-selected, validated payee. | The schema requires payee and the handler calls `addr_validate`. | Consistent; governance rehearsal must encode and validate payee, and reviewers must treat oracle-bond redirection as residual trust. |
| README states no governance/DAO adapter contract ships in v1. | No adapter contract or adapter message surface exists. | Consistent; treat the arbitrator as an address permission only. |
| README calls the production arbitration window seven days. | Seven days is a default for newly asked questions; the proposed market must explicitly request 21 days. | Never infer a question value from the instance README. |
| Comments say a captured cw-filter address means later filter migrations cannot brick a question. | The address is captured, but code at a migratable address can change. | Filters are optional UX only; payout safety must not depend on them. |

## External mechanism/source pins

Pins are the versions inspected for concepts, not permission to copy code.

| System | Version/pin and access date | Use |
| --- | --- | --- |
| Gnosis FPMM | conditional-tokens-market-makers commit [6814c024](https://github.com/gnosis/conditional-tokens-market-makers/tree/6814c0247c745680bb13298d4f0dd7f5b574d0db), accessed 2026-07-15 | Formula and rounding precedent; LGPL-3.0 |
| Gnosis Conditional Tokens | commit [eeefca66](https://github.com/gnosis/conditional-tokens-contracts/tree/eeefca66eb46c800a9aaab88db2064a99026fde5), accessed 2026-07-15 | Split/merge/redeem semantics |
| Reality.eth | RealityETH-3.0 reference commit [b996b0a0](https://github.com/RealityETH/reality-eth-monorepo/blob/b996b0a0899451b95887b59243a118a467f602d0/packages/contracts/flat/RealityETH-3.0.sol), plus upstream main 6b12b99e observed 2026-07-15 | Oracle precedent and sentinel encodings |
| Augur whitepaper | repository commit [69accf63](https://github.com/AugurProject/whitepaper/tree/69accf630d20af5aee5ff3d78fcf6560f069ccfd), observed 2026-07-15 | Dispute security and invalid-market precedent |
| Zeitgeist | repository main 39ad8d60 and [release history](https://github.com/zeitgeistpm/zeitgeist/releases), accessed 2026-07-15 | Mechanism migration and numerical-risk precedent |
| Injective core | master 0000000000b3bf6f65cd809081f5750205565d87, observed 2026-07-15 | Binary lifecycle comparison |

Project documentation without a source commit is cited with its access date in the relevant memo. Such pages establish author claims or current project policy, not executable behavior.

## Reproducibility boundary

The [reproducibility attempt](oracle-wasm-reproducibility.md) retrieved the 361,624-byte deployed wasm independently from two providers and reproduced its `e25473…f3e2` code-info hash. It did **not** reproduce those bytes from source. A best-effort Rust 1.86.0 plus exact Binaryen 116 build produced 361,648 bytes and `fa96b8…b0f6`; the exact optimizer container could not run in this environment.

The repository recipe also selects different optimizer images by host architecture, while the upstream optimizer warns that ARM and Intel outputs differ and recommends Intel for production. A future deployment must pin one OCI digest, reproduce it on two independent builders, and retain the attestation.

Until an independently audited source build reproduces the selected checksum, architecture references to local source behavior are compatibility requirements, not proof about every byte running at the existing production address.
