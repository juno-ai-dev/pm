# Juno Agents DAO verdict packet (review only; do not broadcast)

This packet is a **non-broadcast example** for a market instantiated with the
immutable verdict authority:

```text
juno18k65at7fkf8elhece0fnhsvuxggqg6cved6trp5fyk3lftfn93xsmpeaac
```

It does not create, fund, vote on, execute, or simulate a live proposal. Replace
every angle-bracket placeholder from on-chain queries and independently review
the resulting bytes before any separately authorized proposal process.

## Proposal execution message

Put this single Cosmos message in a DAO DAO proposal. `msg` is the base64
encoding of the compact UTF-8 JSON shown below; `funds` must remain empty.

```json
{
  "wasm": {
    "execute": {
      "contract_addr": "<IMMUTABLE_MARKET_ADDRESS>",
      "msg": "<BASE64_OF_COMPACT_VERDICT_JSON>",
      "funds": []
    }
  }
}
```

Compact verdict JSON (no outer relay or admin message):

```json
{"governance_verdict":{"question_id":"<BASE64_32_BYTE_QUESTION_ID>","answer":"<BASE64_NONEMPTY_ANSWER>","payee":"<VALID_JUNO_PAYEE>"}}
```

The DAO DAO core must dispatch the passed proposal's Cosmos message. Therefore
the binary market must observe:

```text
info.sender = juno18k65at7fkf8elhece0fnhsvuxggqg6cved6trp5fyk3lftfn93xsmpeaac
info.funds  = []
```

A member, multisig participant, proposal module, voting module, arbitrary EOA,
or other contract is not the authority and must not submit the market execute
message directly. There is no generic relay and no authority rotation.

## Pre-execution review

1. Query market `config` and require `verdict_authority` to equal the DAO core
   above exactly; record the immutable market and oracle addresses.
2. Query market `question` and copy its 32-byte `question_id` exactly.
3. Query market `challenge`: require the expected challenger, current answer,
   oracle bond, start time, and deadline.
4. Query market `state`: require `pending_arbitration` and `challenge_used=true`.
5. Query cw-reality `question`: require the same ID, market as arbitrator,
   `pending_arbitration`, and matching answer, bond, and deadline.
6. Confirm the proposed answer is nonempty, the payee is the intended valid
   Juno address, the execute funds array is empty, and execution is scheduled
   strictly before (not at) the challenge deadline.
7. Decode `msg` back to JSON and compare every byte-bearing field. Do not rely
   on a UI summary.

The authority may select a noncanonical answer; the market maps every answer
other than canonical YES/NO to neutral during normal resolution. The payee is
trusted and controls the oracle's arbitrator history entry, so it requires
explicit DAO review.

## Post-execution verification

After a separately authorized execution, but before claiming success:

1. Locate market events `governance_verdict_forwarded` and the exact settlement action (`challenge_refunded` or `challenge_slashed`); verify the common identity fields (`protocol_version`, `factory`, `market`, `height`, `block_time`), `authority`, question ID, answer encodings, payee, challenger, amount, recipient, reason, and disposition.
2. Query cw-reality and require `Finalized`, the exact chosen best answer,
   unchanged user bond, no arbitration deadline, and no pending arbitration.
3. Query market `challenge` and require no challenger/deadline/liability.
4. Query market `accounting` and require challenge liability `0`. A verdict
   different from the snapshot refunds the challenger; an identical verdict
   credits the exact escrow to LP accrual.
5. Query market `solvency`; reconcile bank balance to principal, fees,
   challenge, and LP-accrual liabilities and verify no new forced excess.
6. Call permissionless `resolve`, then query `resolution`; canonical YES/NO
   gets its directional payout and every other answer gets neutral.
7. Verify a second `challenge` and replayed verdict both reject.

If proposal execution does not land strictly before the deadline, do not retry
the verdict. Use permissionless `finalize_stalled_challenge`; it cancels or
synchronizes direct oracle cancellation, slashes the escrow once to LP accrual,
and leaves the oracle answer clock re-extended. A second challenge remains
permanently rejected.
