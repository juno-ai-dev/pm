# ADR-017 — Juno governance arbitration

**Status:** Accepted 2026-07-16; issue #4 retains end-to-end/mainnet rehearsal evidence and transaction authorization
**Decision:** Configure market as cw-reality arbitrator-controller. A bond lets it request arbitration; only pinned Juno x/gov, the accepted ultimate authority, may call GovernanceVerdict, forwarding answer/payee before a 21-day deadline.

## Alternatives

- x/gov directly as oracle arbitrator: only x/gov could request freeze, so public challenge cannot atomically freeze;
- DAO/multisig: contrary to owner authority choice;
- modify cw-reality/add adapter: contrary to unchanged-oracle constraint;
- market controller with narrow relay.

## Evidence

Source requires configured arbitrator for Request/SubmitArbitration. Cosmos SDK v0.50 says proposal messages execute as gov module. Height 39,830,878 records the gov address, 5,000-JUNO deposit, 10-day deposit, and 5-day vote. Passed Juno proposals 357 and 363 contain `MsgExecuteContract` messages whose sender is that exact module account, establishing generic on-chain precedent. Neither exercised the now-accepted market/cw-reality verdict and payee path.

## Consequences

Governance is trusted for canonical answer and arbitrary payee. Unknown answer is neutral. It has no market admin power. The accepted 21-day timeout leaves a six-day margin after the maximum standard deposit and vote periods.

## Safe default

Implementation and rehearsal tooling may encode this accepted path and 21-day value. Deployment and any mainnet rehearsal transaction remain unauthorized until issue #4 proves signer, encoding, deposit, gas, success, failure, deadline, answer, and payee under a separate transaction safety gate. Do not silently substitute another authority.

## Revisit

After rehearsal or changed governance parameters/module address. A failure requires an owner decision on replacement authority.
