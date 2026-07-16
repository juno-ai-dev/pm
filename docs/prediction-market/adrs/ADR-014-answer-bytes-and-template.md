# ADR-014 — Exact answer bytes and canonical question

**Status:** Accepted 2026-07-16
**Decision:** 32-byte big-endian 0=NO, 1=YES, all-ff=INVALID, ...fffe=UNRESOLVED; all other finalized bytes neutral. Bind JCS juno-pm-question/1 bytes.

## Alternatives

- loose string/bool parsing;
- trust AnswerType/filter;
- exact bytes and immutable resolution document.

## Evidence

Reality.eth documents the 0/1/max sentinels. cw-reality accepts opaque Binary and stores Bool without encoding enforcement.

## Consequences

Deterministic payouts for every Binary. Tooling must show hex/base64 and golden canonical JSON bytes. Contract can enforce structure/bounds, not semantic clarity.

## Revisit

Only through a versioned template/market code; never reinterpret a live question.
