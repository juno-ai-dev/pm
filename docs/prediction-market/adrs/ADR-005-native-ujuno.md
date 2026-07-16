# ADR-005 — Native ujuno collateral

**Status:** Accepted 2026-07-16
**Decision:** V1 accepts only native denom ujuno. One JUNO is displayed as 1,000,000 ujuno.

## Alternatives

- CW20: callback/withdraw and issuer risk;
- IBC assets: trace/channel/bridge/issuer risk;
- native ujuno.

## Evidence

The owner selected JUNO. cw-reality native input and native Withdraw are end-to-end coherent; its CW20 receive path still ends in a native BankMsg.

## Consequences

All accounting is integer ujuno. Experimental wording must acknowledge value-bearing/volatile JUNO and its dual collateral/governance role. Look-alike denoms reject.

## Revisit

After a separately audited end-to-end token withdrawal design; a new collateral requires a new market/factory version.
