# Juno x/gov arbitration rehearsal packet

This directory prepares issue #4 for an **authorized rehearsal**. Nothing here deploys, signs, submits a proposal, pays a deposit, votes, executes, or proves that the live path works. A generated packet is always marked `UNSIGNED_UNAUTHORIZED_DO_NOT_BROADCAST`.

The accepted path is:

`Juno x/gov module account → rehearsal market GovernanceVerdict → frozen cw-reality SubmitArbitration`

The market, not x/gov, is cw-reality's configured arbitrator. The proposal therefore targets the exact market and attaches no funds.

## Offline preparation

Create a request matching `request.schema.json` from same-height direct queries. Do not copy stale fixture values. The request binds:

- `juno-1`, the live governance module account, and distinct exact market/oracle addresses;
- canonical base64 encodings of an exact 32-byte question ID and answer;
- a checksummed Juno payee;
- matching pending-arbitration market/oracle snapshots and one deadline; and
- an observation time strictly before that deadline.

Build and independently revalidate:

```sh
python3 scripts/governance/prepare_rehearsal.py build request.json packet.json
python3 scripts/governance/prepare_rehearsal.py validate packet.json
```

The proposal body contains exactly one `/cosmwasm.wasm.v1.MsgExecuteContract`, with the governance module as inner sender, the market as contract, a canonical base64 `governance_verdict` message, and `funds: []`. The tool fails closed on identity, state, deadline, encoding, address checksum, attached-fund, or payload drift. It deliberately does not build a signed `MsgSubmitProposal`, choose a depositor, estimate gas, or broadcast.

## Authorization gate

Before any live action, record all of the following outside this repository's public packet as applicable:

1. explicit rehearsal environment, governance, transaction, and funding authorization;
2. named proposer/deposit source and a confirmed non-production/value bound;
3. two-provider, one-height queries of chain ID, gov module account/parameters, market Config/Identity/Challenge, oracle Question, code checksums, and both admin surfaces;
4. independent decode of the final proposal and confirmation that the inner sender is the live gov module, target is the rehearsal market, fields match the active challenge, and funds are empty;
5. gas simulation and an abort rule for any mismatch or deadline margin breach; and
6. named evidence reviewer independent of the proposal preparer.

Issue #4's authorization/funding/governance-coordination gate remains open until those facts exist. Never infer authority from this runbook or a merged preparation PR.

## Evidence manifest

Create a dated, immutable evidence directory with hashes and source heights. Do not store keys, tokens, private advice, or unnecessary personal data. For each case record:

- chain ID, height, block time/hash/app hash, provider URLs, response hashes;
- proposal ID/status, deposit/vote/execute transaction hashes, gas wanted/used, and raw events;
- exact market/oracle addresses, code IDs/checksums/admins/config, question ID, pre/post state, challenge amount and deadline;
- decoded outer proposal and inner execute JSON, answer/payee, and proof of empty funds;
- expected outcome, actual outcome, error text/code, balance/liability deltas, and reviewer/date.

Required cases:

| Case | Expected evidence |
|---|---|
| Success before deadline | executed sender is x/gov; market and oracle finalize atomically; exact answer/payee; challenge refund/slash matches accepted rule |
| Wrong sender | market rejects; market/oracle/challenge state and balances unchanged |
| Wrong market | no mutation of intended market/oracle; proposal target discrepancy is explicit |
| Wrong question | market rejects; pending challenge and segregated bond remain unchanged |
| Attached funds | market rejects; no funds retained and no state transition |
| Malformed/short answer | preparation tool rejects; if a separately authorized live negative test is required, contract behavior and neutral-mapping implications are recorded |
| Invalid payee | preparation or chain address validation rejects without mutation |
| Stale/nonpending state | market rejects without double refund/slash/finalization |
| Oracle submessage/reply failure | transaction rolls back atomically; pending challenge and bond remain |
| At deadline | market rejects (`now < deadline` is required) |
| Passed proposal, failed execution | proposal status and failure reason/events are preserved; no false success claim |
| Retry before deadline | a corrected proposal is independently decoded, remains pre-deadline, and succeeds or records authoritative failure |
| No executed verdict by deadline | permissionless stalled synchronization produces the accepted one-time slash/cancellation behavior |

Negative cases should use rehearsal-only contracts and bounded authorized funds. Do not endanger a live value-bearing market merely to satisfy a matrix row.

## Issue closure evidence

This preparation only **advances #4**. Closure still requires authoritative transaction/proposal/event/state evidence for the live Juno governance-module execution path, the required safe failures, deadline behavior, gas, challenge disposition, and the ADR-017 acceptance/replacement decision. A passing offline test is not that evidence.
