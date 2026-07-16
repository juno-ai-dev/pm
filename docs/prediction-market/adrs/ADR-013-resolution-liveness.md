# ADR-013 — Unanswered and stalled resolution

**Status:** Accepted 2026-07-16
**Decision:** Fund a 1-JUNO bounty and operate alerts/keepers. Unanswered remains nonterminal indefinitely. Challenged arbitration cancels publicly after 21 days, slashes challenge bond, and restarts 24-hour answer finality.

## Alternatives

- privileged emergency neutral: violates finalized-oracle binding;
- automatic neutral without oracle: same violation;
- re-question: unsupported by current source;
- disclose nontermination plus incentives.

## Evidence

cw-reality OpenUnanswered never time-finalizes. Its post-deadline CancelArbitration restarts finalize_ts.

## Consequences

No maximum settlement time if nobody answers or counter-answers continue. Complete-set Merge remains available. One challenge only bounds repeated governance freezes.

## Revisit

After a bounded oracle-preserving re-question design; no live override.
