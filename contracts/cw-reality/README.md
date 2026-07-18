# JunoReality

A CosmWasm port of **[Reality.eth](https://reality.eth.link/)** — a
bond-escalating crowdsourced oracle for contested social facts. Arbitration
is pluggable through an address permission: Juno governance, DAO DAO, a
multisig, or another authority can be configured directly. No adapter contract
ships in v1.

## Credit & inspiration

JunoReality is a port — the mechanism design isn't ours. Bond escalation,
the right-answer redistribution rule, the history-hash chain, the
arbitrator-as-address permission model, the `UNRESOLVED_ANSWER` semantics
— all of it comes from **Edmund Edgar and the Reality.eth contributors**
([reality.eth](https://reality.eth.link/),
[RealityETH/reality-eth-monorepo](https://github.com/RealityETH/reality-eth-monorepo)).
The canonical reference we ported against is `RealityETH-3.0.sol` at
[commit `b996b0a0`](https://github.com/RealityETH/reality-eth-monorepo/blob/b996b0a0899451b95887b59243a118a467f602d0/packages/contracts/flat/RealityETH-3.0.sol).

We also lean on the wider bond-escalation-oracle literature that informed
Reality.eth: Augur's REP dispute rounds, UMA's optimistic oracle, and
Kleros' juror-bond mechanics.

**`cw-reality` is a clean-room reimplementation under Apache-2.0.** Reality.eth
itself ships under GPL-3.0; this port studied the source for behavioural
equivalence but does not copy code. Where docs cite line numbers in
`RealityETH-3.0.sol`, those are pointers for auditors verifying the port,
not licensed inclusions.

## Substantial differences from Reality.eth

Most of the mechanism is preserved literally — the Alice/Bob/Carol worked
example from the Reality.eth whitepaper produces identical payouts here. The
deltas (multi-denom denom binding, contract-level timeout floor, dispute
round cap, sha-256 history hash, explicit state enum, cw-filter answer
schemas, removal of commit-reveal) are catalogued in:

→ **[`docs/juno-reality/differences-from-reality-eth.md`](../../docs/juno-reality/differences-from-reality-eth.md)**

## Repository

- **What we're building + why** → [`GOAL.md`](../../GOAL.md)
- **The arbitration design call** → [`ARBITRATION.md`](../../ARBITRATION.md)
- **Historical reading gate** → [`reality-eth-reading-list.md`](../../docs/juno-reality/reality-eth-reading-list.md)
- **Port-time lessons digest** → [`reality-eth-lessons.md`](../../docs/juno-reality/reality-eth-lessons.md)
- **Self-audit** → [`self-audit-checklist.md`](../../docs/juno-reality/self-audit-checklist.md)

## Live on juno-1

| Instance | Address | Notes |
| --- | --- | --- |
| v1 | `juno1g0pveeymzn3a3asu6v2dhkclqhwsndmvjugjx8a4qx554esp5yessuceur` | Production defaults (24h answer timeout, 7d arbitration window). |
| v2 | `juno1ys6ynhlvv3c2s0kpdn29jpgw43rwpzw9gqz6hjafzp4yqn6rww7qplg8l5` | Fast-cycle demo (1h floor); first question lifecycle closed 2026-05-28. |

Code ID: `5121` (sha256 `e25473e7eb08b5fc23b66926073958458b01a7b9b5642855249bc3d9b7f7f3e2`).

## Licenses

- `contracts/cw-reality` — **Apache-2.0**
- The historical `depricated_ui/` records an **AGPL-3.0** claim but is
  unsupported; see the repository licensing decision record before reuse.
