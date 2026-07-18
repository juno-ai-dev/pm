# ARBITRATION — JunoReality

## The position

**The arbitrator slot in `cw-reality` is an address. Any bech32 authorized to call `SubmitArbitration` for a given question can serve. No adapter contracts. No trait. Permission, not abstraction.**

The address can be:

- A **DAO DAO core** — the DAO passes a proposal whose msgs include `SubmitArbitration { question_id, winning_answer, payee }`. When the proposal executes, the call lands on `cw-reality` with `info.sender == dao_core`. `cw-reality` checks `sender == question.arbitrator`, finalizes.
- **The Juno x/gov module account** — a gov proposal carries `MsgExecuteContract { sender: gov_module_account, contract: cw-reality, msg: SubmitArbitration {...} }`. When gov passes, the call lands with `info.sender == gov_module_account`. Same check, same path.
- **A multisig, an EOA, a future Kleros-clone, a federation** — anything that controls a bech32.
- **`None`** — bond-exhaustion-only resolution. Suitable for low-stakes questions where the escalation game is sufficient on its own.

The "pluggable arbitrator interface" is one permission check on one message. That is the whole thing.

## Why this is the right shape

Reality.eth's `IArbitrator` is a trait because Solidity is class-shaped and EVM permission models reward type abstractions. CosmWasm is message-shaped and permissions are bech32-shaped. Forcing a Solidity-style trait onto a CosmWasm oracle imports complexity that does not pay rent: adapter contracts that just pre-format proposal payloads, adapter contracts that just track timeouts the oracle already tracks, adapter contracts that just hand off addresses the oracle can store itself.

Off-chain tools (`reality-ui`, a CLI, a wallet) format the proposal payload. `cw-reality` tracks the arbitration timeout itself. `cw-reality` stores the arbitrator address itself. An adapter contract would be a redundant layer dressed up as a primitive.

## Default and recommendation

**Recommended arbitrator for a new question: a DAO DAO DAO.** Specialized, recallable, exit-respecting. A "Reality Council" DAO can recruit subject-matter experts and run 24–48-hour voting periods. The asker sets the arbitrator field to the DAO's core address at ask time.

**Available: Juno x/gov.** When the dispute is chain-level — a question about Juno itself, or whose blast radius is every JUNO holder — gov is the right answer. The voting period (around two weeks) is the *feature*: friction forces the asker to ask whether the dispute really merits chain-level attention.

**Available: `None`.** Bond-exhaustion-only. Reality.eth's "no arbitrator" mode. Cheaper, faster, riskier.

The reference UI may default the arbitrator field to a configured Reality Council DAO; users can override per question.

## What we are not building

- **A jury system / Kleros-clone.** Not in scope. If someone wants one, they deploy a contract that controls a bech32 and set it as the arbitrator.
- **A meta-arbitrator (arbitrator of arbitrators).** Out-of-protocol. Disputes about an arbitrator's decision are resolved by *next question, pick a different arbitrator*.
- **Adapter contracts.** No `reality-arbitrator-dao`, no `reality-arbitrator-gov`. The address is the interface.

## Implementation note

The JSON message shape is:

~~~json
{
  "submit_arbitration": {
    "question_id": "<base64-encoded 32-byte question ID>",
    "winning_answer": "<base64-encoded arbitrary bytes>",
    "payee": "<address accepted by the chain address validator>"
  }
}
~~~

The `SubmitArbitration { question_id, winning_answer, payee }` handler:

- requires a 32-byte ID for an existing question with a configured arbitrator;
- checks `info.sender == question.arbitrator` and rejects every other sender;
- requires the question to be in `PendingArbitration`;
- does not reject merely because `arbitration_deadline` has passed. The question remains pending until submission or cancellation; after the deadline anyone may take the separate cancellation path;
- validates `payee` with `deps.api.addr_validate` (normally a valid bech32 address on Juno);
- accepts **any `Binary`** as `winning_answer`, including bytes and lengths never submitted by an answerer. It performs no history-membership or answer-schema proof. `UNRESOLVED_ANSWER` is recognized for the event flag and claim behavior, but is not the only new value the arbitrator may author; and
- appends a zero-bond history entry whose answerer is `payee`, clears the pending flag and deadline, stores the answer as best, and sets `finalize_ts` to the current block time. A later `Claim` walk uses that history entry when distributing the bounty and answer bonds.

This makes both fields a security boundary: the arbitrator chooses the final bytes **and** the validated payee. A consumer can safely map unrecognized bytes to a neutral market payout, preventing unknown bytes from blocking redemption or selecting an unintended binary side. That mapping cannot prevent oracle-bond redirection: `cw-reality` records the arbitrator-selected payee as the latest winning answerer, independently of how the consuming market interprets the bytes.

That's the entire arbitrator interface. Anyone implementing arbitration on top of `cw-reality` writes whatever they need to *produce a `SubmitArbitration` call from the right sender*. The oracle does not care how the answer was decided, only that the configured authority is the one calling.

## Open question (deferred to stage 5)

The first real arbitrator for the first real mainnet question. A DAO DAO DAO with what membership? A temporary multisig? Juno gov? `None`? Settle before stage 5. The choice shapes the demo more than the contract.

## Closing

Kleros is one shape. A DAO DAO DAO is another. The Cosmos x/gov module account is a third. The arbitrator slot is an address because *every shape that matters resolves to one*. Treat arbitration as a permission, ship the contract, let the social structure of dispute resolution live in whatever entity controls the bech32.
