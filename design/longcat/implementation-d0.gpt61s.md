---
type: ImplementationReport
title: Stock Feedback production composition — slice D checkpoint
description: Ordinary stock runner, configuration, owned scheduling, recovery, limited-result holds, scripted acceptance and a fresh-session native acceptance handoff.
resource: design/longcat/implementation-d0.gpt61s.md
status: checkpointed
generated: { at: 2026-11-12, by: 'model:openai/gpt-6.1-sol#high' }
sources:
  - resource: opencode://ses_e32c4da49ffekNe2490gCdHuLp
    title: Implement production stock Feedback composition and scripted acceptance, verify feature isolation and preserve the integrated native acceptance boundary for a fresh session
  - resource: feedback0.gpt6a.md
    title: Selected destination and commissioned slice D
  - resource: implementation-a0.gpt61s.md
    title: Stock typed source and verified qualified command boundary
  - resource: implementation-b0.gpt61s.md
    title: Pure Feedback preferences, command evidence and completion mailbox
  - resource: implementation-c0.gpt61s.md
    title: Frozen geometry, donor selection, ordered refresh and exact call sequence
  - resource: ../../app/zk/src/runner/feedback.rs
    title: Implemented semantic bridge, quiet acquisition, motion classification and execution limitation holds
  - resource: ../../app/zk/src/runner/stock.rs
    title: Implemented production clocks, history admission, owned command/probe/refresh futures and bounded exit drain
  - resource: ../../app/zk/src/runner/stock_tests.rs
    title: Eighteen scripted application composition and lifecycle regressions
  - resource: ../../app/zk/tests/longcat_cli.rs
    title: Actual binary startup selection and configuration precedence
  - resource: ../../app/zk/src/runner/README.md
    title: Maintained actual stock and preserved Episode runner contracts
  - resource: ../../desktop/umbriel/tests/native/check.sh
    title: Existing isolated A/C native harness entrypoint for the next acceptance session
verification:
  method: Scoped Nextest, warning-denying all-target Clippy, owned-file rustfmt, Rustdoc, ten application feature configurations and document checks.
  result: 200 desktop/layout/application tests pass, two opt-in native tests remain skipped; strict Clippy and owned formatting pass. Ten feature configurations pass all-target checks and backend/service CLI tests. Rustdoc passes. Maintained contracts and this handoff pass Rekon document checks.
  limits: Production route plus scripted peers, not integrated native stock user-guidance acceptance. No native test was run in D, no desktop write, installed-service change, compositor-source change, installation, squash or push.
extensions:
  ticket: zk-longcat-return
  related_work: [zk-longcat-user-layout-changes]
---

# D production checkpoint

Ordinary **`zk longcat --backend umbriel` now composes A–C through stock
`Feedback`**. Extension presence cannot select the old permanent-conflict policy.
`--umbriel-control conditional` explicitly preserves the bespoke route; niri
continues through its unchanged Episode executor. This is running production code
with scripted acceptance, **not yet the full native product acceptance**.

## Commits and owned scope

| Commit | Coherent stage |
|---|---|
| `d55a2f86` | Stock routing/config/service arguments; semantic Feedback bridge; frozen geometry composition; timers/owned command/refresh/probe loop; recovery/shutdown; initial scripted fixtures; small B integration corrections. |
| `16ec8020` | Additional diagnostic-failure receipt drain, same-epoch healthy-after-loss refusal and service-argument preservation regressions. |
| `3b3162c8` | Retire expired acquisition timers when groups become too small/unprojectable, with a regression preventing quiet-wait busy loops. |
| `e39d2da9` | Preserve unattributed overlap when the handling reply arrives before the independent/reversing position snapshot; exercise both receipt orders. |
| `7cbacc6c` | Format the owned integration regression updates without changing behavior. |

Application package is now **0.2.0**; `zk-umbriel` is **0.3.0** for its new public
`Reconnect` reexport, with the receiving lock updated. Library source defaults
remain no-reconnect; only ordinary stock application startup selects bounded retry.
`zk-longcat` remains 0.3.0; its corrections are internal behavior, not a new API.
No sibling repository source changed. Initial working state was clean. The
existing beads history was inspected/preserved and only appended to for this work.
The final contract/handoff commit is separate from the implementation checkpoints.

## Implemented production contract

- **Routing/configuration:** stock subscribes to windows/workspaces/outputs, never
  probes `layout-state`, and uses separately verified qualified down/right edges.
  Explicit conditional control still probes/requires its extension. Width and
  height minima are distinct, default 60, with file → environment → optional CLI
  precedence. Active/other weights retain their existing normalized-share meaning
  on both axes. Service generation carries width and explicit route arguments.
- **Semantic bridge:** full live census retains unsupported windows/preferences;
  only meaningful reported ordered groups enter geometry. Independent context
  contains output/presentation/primary facts, not title, activation, positional
  ordinals or ordinary secondary envelope edits. Group size remains 2–128.
- **Quiet geometry:** relevant motion starts/restarts real trailing quiet timers;
  a single still-current snapshot can acquire. Frozen B/strips persist through
  own intermediate sums, focus switches and native handling. A mixed-time axis
  disappearance uses C's private frozen authority, never invented candidate facts.
- **Exact execution ordering:** donor selection → latest compile → B admission
  using compiled capped travel → geometry `begin` with the same key → IPC future.
  Current capture installation precedes B's position response. A native reply is
  not a position response or committed convergence. The next edge needs both
  observations/receipt; it is never an irrevocable prequeued tail.
- **Scheduling:** independent native receive, at most eight queued semantic
  updates per catch-up turn, every intervening loss/focus/topology marker assessed,
  latest-scope/version and empty-history admission recheck, completion priority.
  One command future globally, one independent timed refresh and one capability
  probe. Dedup/silence still fires quiet, 32ms position and convergence timers.
- **Refresh:** exactly one same-stream query per silent edge. Its 32–250ms timeout
  derives from configured convergence timeout; a pending query suppresses the
  second position timer. After completion, history drains and latest facts are
  reassessed before blockage. Tracker remains the row owner at true receive order.
- **Guidance versus constraint:** unrelated/reversing motion yields; outstanding
  overlap can be explicitly unattributed guidance after receipt drain and quietness,
  including reply-first scheduling when the position response is still unresolved.
  Requested-boundary movement coupled with other boundaries, and incompatible
  commit-only geometry during own unsettled execution, are conservative native
  execution limitations instead. The failed budget/reference is held; late
  commit-only changes do not teach. Stable quiet mismatch cannot reacquire and
  retry forever. Independent target movement, focus/topology/context change can
  release that hold. This classification is geometric evidence, not causality.
- **Late commits:** C's cumulative most-recent key remains authoritative in its
  scope after completion. B now preserves that key across a replacement that
  admitted **no** new command, instead of rejecting later compatible evidence as
  unknown. Topology/context retirement is reported before missing-model waiting.
- **Loss/recovery:** loss immediately cancels unsent authority; possibly-sent
  deltas are never replayed. The native source retries transport EOF/loss with
  250ms spacing, up to eight connection attempts. Fresh epochs need all three
  families and fresh qualified probes. Old receipts and the completion mailbox
  drain before new authority. Per-family EpochChanged recovery markers cannot
  accidentally mark the new acquiring epoch lost. A healthy same-epoch snapshot
  cannot undo a processed loss; protocol/observer-lag terminal failures stop.
- **Blur/shutdown/output:** current neutral preferences use the same executor;
  active edits win. SIGINT/capture limit latches restore under the configured
  shutdown bound. Every exit, including diagnostic failure, revokes unsent work
  and drains the command within the remaining budget; timeout records uncertainty
  and never replays. Intent-before-future output failure is genuinely unsent.
  Existing stdout 16-row/2MiB-per-row bounds, native cleanup and two-second output
  drain behavior remain intact. Assessment text is capped at 32 with omitted count.
- **Finite evidence:** persist completion before taking B's one-slot mailbox;
  retain live preferences/models, one finite goal and cumulative counters. No
  stock 32-episode/64-effect historical veto. JSONL contains preferences, Taffy
  goal, reported/inferred geometry, actual edge fraction, pending command, outcome,
  achieved geometry, guidance and limitation. Dry-run has no mutation/probe or
  fabricated native-handling evidence; its controller/goal lifetime is hypothetical.

## Verification receipts

```sh
cargo nextest run --locked -p zooming-kittens -p zk-desktop -p zk-niri \
  -p zk-umbriel -p zk-longcat -p zk-layout --status-level fail --final-status-level fail
cargo clippy --locked -p zooming-kittens -p zk-desktop -p zk-niri \
  -p zk-umbriel -p zk-longcat -p zk-layout --all-targets -- -D warnings
cargo doc --locked -p zk-longcat -p zk-umbriel --no-deps
```

**200 passed**, two opt-in native tests skipped; strict Clippy, Rustdoc and
explicit owned default-style `rustfmt --edition 2024 --config-path /dev/null
--config skip_children=true --check FILE...` pass. Whole inherited source trees
were not reformatted. Eighteen stock composition tests cover both-axis quiet
acquisition, response-before-reply, focus/topology cancellation, reconnect with
pending receipt, fresh-family recovery markers, healthy-after-loss refusal,
silent one-refresh limitation, continuous motion/guidance/blur/focus return,
cumulative late commits, coupled/commit-only constraints, output changes,
70 focus changes, bounded unanswered shutdown, diagnostic-failure receipt drain
and retirement of acquisition timers for unprojectable/shrunken groups.
An additional B regression preserves the old key through no-command replacement.
The actual CLI peer confirms ordinary stock subscription and no bespoke probe;
configuration and service CLI regressions cover both new arguments.

All ten application configurations pass `cargo check --locked -p zooming-kittens
--all-targets` and scoped `backend_features`/`systemd_cli` Nextest tests: no features,
each singleton, all three pairs, all three together, export-facet and all-features.
The scratch runner/logs are under `.test-agent/longcat-feedback-d/`; each feature
case uses `--no-default-features --features LIST`, or `--all-features`.
The inherited niri-without-kitty optional `tokio-stream` warning remains nonfatal.
This is actual-checkout composition, not a claim about every sibling consumer.

## Fresh-session continuation: integrated native acceptance

This session rolls over after production/scripted acceptance rather than stretching
through the full native interaction matrix. **Do not close the full effort or the
user-layout task yet.** Parent retains architecture and acceptance review.

1. Read this receipt and A–C, then the maintained runner/desktop/layout contracts.
   Inspect source/current working state; do not infer deployment from old receipts.
   Use fresh Sol high context and the requested rollover range.
2. Use only `~/src/umbriel-working`'s isolated headless harness. Build the actual
   application with `cargo build --locked -p zooming-kittens --no-default-features
   --features umbriel`. `bash desktop/umbriel/tests/native/check.sh` reruns A/C's
   native checks; **it is not D's end-to-end acceptance**.
3. Add an ordinary application check, e.g.
   `app/zk/tests/headless/umbriel_stock_feedback.sh`, against the existing native
   harness (not the desktop socket). After creating that check, run with
   `ZK_TEST_BIN="$PWD/target/debug/zk" CHECK_DIR="$PWD/app/zk/tests/headless" bash
   ../umbriel-working/tests/harness/check.sh ../umbriel-working/build-debug/umbriel
   umbriel_stock_feedback -j 1`. Keep the explicit conditional check separate; it
   now supplies `--umbriel-control conditional` and needs its compatible compositor.
4. Demonstrate **live subscription → Feedback/Taffy → qualified resize → observed
   convergence → accepted user adjustment → continued focus behavior** through the
   real executable. Assert an exact unconstrained committed fixture, blur/current
   neutral restore, learned focus return, continuous drag/yield, member moves,
   mixed title bars/gaps/outer envelope, stale IDs, unchanged seat focus/unrelated
   columns. Add constrained/slow-client/strut cases with truthful bounded limits.
   Native source/client constraints and dedup pacing cannot be established by the
   scripted ideal peer alone. Scope and diagnose failures rather than weakening
   attribution or reacquiring failed targets to make tests pass.
5. Check native transport reconnect/probes with the real source if practical;
   current D recovery tests inject semantic updates and mock services. A/C's lower
   boundary tests verify actual sockets separately, not this full composition.
6. Extend output/receipt-order stress and membership churn as useful. Preserve
   output failure/no-replay guarantees. Re-run scoped checks and affected feature
   cases after corrections, append native evidence, refresh maintained contracts
   and ticket pointers, then return to parent review. No desktop write, install,
   service mutation or push is authorized by this handoff.

## Connections

The [selected design](feedback0.gpt6a.md#d--application-integration-and-usable-service-acceptance)
defines the still-open native success bar. [B's mailbox contract](implementation-b0.gpt61s.md#public-call-sequence-for-cd)
and [C's incompatible-geometry boundary](implementation-c0.gpt61s.md#d-call-sequence-and-boundaries)
explain why this composition holds native limitations rather than learning failed
geometry. [The effort index](README.md#delivered-related-integration) keeps these
implementation checkpoints distinct from installed-service and parent acceptance.
