# ADR-017 — Immutable verdict authority profiles

**Status:** Amended 2026-07-17 by issue #45; supersedes the 2026-07-16 x/gov-first authority choice
**Decision:** Configure each market as its `cw-reality` arbitrator-controller and pin one immutable `verdict_authority` address at instantiation. A bonded public challenge lets the market request arbitration. Only that exact authority may call `GovernanceVerdict` and forward answer/payee before the arbitration deadline. The v1 profile pins the Juno Agents DAO core; Juno `x/gov` remains a future compatible profile.

## V1 authority

The initial authority is the Juno Agents DAO core:

- `juno18k65at7fkf8elhece0fnhsvuxggqg6cved6trp5fyk3lftfn93xsmpeaac`
- proposals: <https://daodao.zone/dao/juno18k65at7fkf8elhece0fnhsvuxggqg6cved6trp5fyk3lftfn93xsmpeaac/proposals>

A passed DAO DAO proposal must execute the market verdict message from this core address. Members, voting modules, proposal modules, arbitrary EOAs, and other contracts are not equivalent authorities.

## Alternatives

- `x/gov` directly as oracle arbitrator: only `x/gov` could request the freeze, so a public challenge could not atomically freeze;
- `x/gov` as the v1 market verdict authority: technically plausible, but slower and operationally heavier than the active DAO; retained for future issues #4 and #13;
- mutable DAO/multisig or authority rotation: rejected because funded markets must not change settlement authority;
- modify `cw-reality` or add a generic adapter: rejected by the unchanged-oracle and no-generic-relay constraints;
- market controller with a narrow immutable authority relay: accepted.

## Evidence

`cw-reality` requires its configured arbitrator for `RequestArbitration` and `SubmitArbitration`, so the market must remain the oracle arbitrator while separately authenticating the verdict authority. DAO DAO core contracts execute passed proposal messages as the DAO core; issue #45 must prove the exact sender behavior in contract tests and produce a non-broadcast proposal packet. No live Juno Agents DAO arbitration proposal is claimed here.

Historical `x/gov` evidence remains relevant: Cosmos SDK v0.50 says proposal messages execute as the governance module, and passed Juno proposals 357 and 363 establish generic governance-originated `MsgExecuteContract` precedent. That evidence does not make `x/gov` a v1 blocker or prove the prediction-market verdict path.

## Consequences

- The verdict authority is trusted to select answer bytes and payee only after a bonded challenge. It has no market admin, migration, pause, collateral, payout-mapping, or authority-rotation power.
- Pinning the DAO core address does not freeze the DAO's voting rules, modules, membership, or code. DAO governance and upgrade behavior are explicit external trust assumptions that must be disclosed and rechecked before deployment.
- Unknown/noncanonical answer bytes resolve neutrally at the market layer.
- The authority field is address-based so a later market version can pin the Juno `x/gov` module account without changing consensus-critical challenge or settlement semantics.

## Safe default

Implementation may encode the immutable authority boundary and Juno Agents DAO v1 profile. It must not automatically create, fund, vote, execute, or claim evidence for a live DAO proposal. A live proposal or funded canary requires separate authorization and belongs to the launch/readiness gate. Issues #4 and #13 are deferred `x/gov` compatibility work and do not block DAO-based v1.

## Revisit

Revisit the v1 DAO profile if its core address, code, governance modules, voting rules, or operational activity changes. Revisit the future `x/gov` profile only through #4/#13 with authoritative sender, encoding, timing, gas, failure, answer, and payee evidence.