# Reality.eth — pre-implementation reading list

> Stage 1 gate (per `PLAN.md`). **No contract code lands until this file is filled with notes and `docs/reality-eth-lessons.md` exists.**

This is engineering due diligence, not a curiosity. Reality.eth has multiple production versions, multiple integrations (SafeSnap, Omen, conditional tokens), and multiple post-mortems. We are inheriting their lessons, not relitigating them.

## Required reading

### 1. Reality.eth contracts (Solidity source)
- Mainnet deployment: latest `Realitio_v2_1` / `RealityETH-3.0` contract on Etherscan
- Specifically map:
  - State enum + transitions
  - Bond escrow path
  - `claimWinnings` / loser-bond redistribution math
  - Arbitrator interface (`IArbitrator`)
  - `notifyOfArbitrationRequest` flow
  - Finalization timing rules (`min_timeout`, `delay_finalization`)

**Output:** annotated state machine + bond accounting notes. One markdown doc per concern.

### 2. Reality v2 → v3 changelog + post-mortems
- Edmund Edgar's blog (realit.io / edmundedgar.com posts on Reality versions)
- Reality.eth GitHub issues + closed-PRs touching `Realitio.sol` / `RealityETH.sol`
- Specifically map:
  - What changed v2 → v3 and *why* (security? UX? gas?)
  - Known edge cases that drove revisions
  - Any defaults that were revised — and where they landed

**Output:** "lessons learned" doc with concrete cases we must cover in tests.

### 3. SafeSnap deployment history
- Gnosis Zodiac → SafeSnap module
- Real-world deployments that used it (look at the published Snapshot ↔ Safe integrations)
- What worked, what got disabled, what got forked

**Output:** specific failure modes we should not reproduce — informs the test suite for `cw-reality` and any future DAO integration patterns.

### 4. Bond-economics literature
- Augur v1/v2 dispute escalation paper(s)
- UMA "Data Verification Mechanism" paper
- Kleros yellow-paper
- Comparison table: bond multiplier, voting cost, attack cost

**Output:** working defaults table for `cw-reality` justified against the literature, not against vibes.

### 5. CosmWasm prior art (in-repo)
- `dao-proposal-single` — proposal deposit handling, refund paths (`memory/dao-proposal-single-patterns.md`)
- `cw-abc` — bond accounting in a CW context (`memory/cw-abc-branch.md`)
- `gauges-are-cool` — streaming + per-block accrual patterns (`memory/gauges-are-cool-branch.md`)
- `dao-proposal-wavs` — cw-filter integration pattern, envelope handling

**Output:** which patterns to copy, which to skip, why.

## Open questions to answer during reading

- Does Reality.eth allow asking the *same question* twice? If yes, how is duplication handled?
- What is the actual loser-bond redistribution math when there are >2 escalation rounds with different bidders per round?
- How does Reality.eth handle a question whose asker disappears? Bond reclamation paths?
- What is the gas profile of escalation as the bond grows? Any griefing vectors via tiny-bond escalation?
- How does Reality.eth deal with answer-schema validation (or does it)? We are adding cw-filter; what failure modes does that introduce?
- What's the "uncle answer" rule (if any) — can a previous-round answer that no one defended still claim if a later round is invalidated?
- How do arbitrators decline? Is there an explicit "I refuse" outcome, or does timeout fall through?

## When this file is "done"

- Each section above has at least one set of notes with citations (URL + commit hash where possible)
- The "open questions" section has been turned into resolved positions
- A `docs/reality-eth-lessons.md` exists with the digested findings
- Stage 2 can start without re-reading every source

## Hard rule

The temptation will be to skim this and start coding. Do not. Reality.eth's mechanism is subtle; the bugs that destroyed UMA v1 / Augur v1 came from "looks reasonable" defaults. We read first.

---

## Stage 1 close (2026-05-28)

Reading happened. Source artifacts:

- **§1 source walkthrough** → `research-notes-reality-source.md` (v3.0 Solidity, state machine, bond escrow path, `claimWinnings` math, `IArbitrator`, `notifyOfArbitrationRequest` flow, finalization timing, commit-reveal).
- **§2 + §3 version history + SafeSnap** → `research-notes-v2v3-safesnap.md` (v1→v2→v3→v4 reconstructed from commits + audits; G0 audit issues; SuDAO and Gnosis Guild post-mortems; 12 numbered failure modes).
- **§4 bond-economics literature** → `research-notes-bond-economics.md` (Augur v2 whitepaper, UMA OOv2/v3 + DVM, Kleros yellow-paper, Reality.eth design rationale; comparison table; defaults table for cw-reality).
- **§5 in-repo CosmWasm prior art** → `research-notes-cw-prior-art.md` (dao-pre-propose-base deposit handling, cw-abc state shape, gauges delta-update primitive, dao-proposal-wavs cw-filter integration, `workspace-optimize` recipe, cw-std-2 migration state).

**Digested lessons** (defaults, failure modes, resolved open questions, port-time decisions) live in `reality-eth-lessons.md`. **Stage 2 unblocks.**
