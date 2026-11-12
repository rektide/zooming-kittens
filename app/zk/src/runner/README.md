---
type: ImplementationContract
title: Longcat application runner
description: Stock Umbriel Feedback composition, history-gated owned edges and recovery, plus preserved niri and explicit conditional episodes.
resource: app/zk/src/runner/README.md
status: implemented
generated: { at: 2026-10-15, by: 'model:openai/gpt-6.1-sol#medium' }
sources:
  - resource: opencode://ses_e32c4da49ffekNe2490gCdHuLp
    title: Implement ordinary stock Feedback runner, qualified-edge scheduling, fresh-epoch recovery, bounded shutdown and scripted acceptance
  - resource: ../../../../design/longcat/implementation-d0.gpt61s.md
    title: Production composition receipts and remaining native acceptance
  - resource: opencode://ses_e9a70297dffe6dQ5UJP0up8m7T
    title: Integrate whole-group Umbriel control, coherent authority and native restoration; verify against isolated native compositor
  - resource: ../../tests/headless/umbriel_longcat.sh
    title: Real ZK binary and headless compositor emphasis, switching, restoration and external-edit verification
  - resource: ../../../../desktop/umbriel/src/layout.rs
    title: Native snapshot projection, request compiler and receipt verifier
  - resource: opencode://ses_ec2f9e1a4ffeOo3Z1LQUDdetQ5
    title: Rust quality cleanup, observer-loss resynchronization and subprocess verification of bounded stdout shutdown
  - resource: opencode://ses_ec310f020ffe5d6kIV42ehdO2z
    title: Longcat application integration, scripted peers, configuration composition and continuity-admission correction
  - resource: ../../../../desktop/README.md
    title: Native source, bounded observer and height-actuator contracts
  - resource: ../../../../layout/README.md#episode-api-and-observation-bridge
    title: Episode preview, execution evidence, convergence and restoration contracts
  - resource: ../../../../design/remake/architecture0.gpt6a.md
    title: Observation, policy and execution separation with direct caller-budgeted scheduling
---

# Longcat runner

`zk longcat` uses niri and executes by default. Read-only acquisition:

```sh
cargo run -- longcat --backend niri --dry-run --limit 1
cargo run -- longcat --backend umbriel --dry-run --socket /path/to/socket --limit 1
```

`--limit` counts **ready** capture/decision records, not initial unseen-family
waits, receipts or final shutdown evidence. Stock Umbriel requires independently
current windows, workspaces and outputs in one source epoch; niri requires its
two families. An idle/unready peer still requires SIGINT. `--socket` overrides
native discovery. No periodic acquisition query or focus workaround is used.

Ordinary `--backend umbriel` now selects stock Feedback, regardless of whether a
bespoke extension exists. `--umbriel-control conditional` explicitly selects the
preserved `layout-state`/`layout-allocate` Episode route and requires that extension.
Dry-run does not probe qualified actions or submit native mutations.

## Configuration

The app loads its existing file and `ZK_` environment configuration without global
CLI defaults, then applies only optional Longcat overrides. Invalid configuration
is an error, never a fallback. The two positive finite active/other weights are
normalized as `active / (active + other)`; for example `3` and `2` mean `.6`.
`minimum_height` remains the vertical per-member content minimum; `minimum_width`
is the horizontal secondary minimum (both default 60). Stock uses
`resize_debounce_ms` (default 500) for trailing geometry quietness and guidance.
Episode routes do not learn resizes. It is **not** the convergence deadline.
`--convergence-timeout-ms` is independent;
`--shutdown-timeout-ms` bounds conditional restoration and pending-receipt drain.

## Stock Feedback composition

`feedback.rs` joins the full live census (including unsupported windows) to pure
`zk_longcat::Feedback`. Source/epoch-qualified identities retain neutral weights,
learned focus shares and contextual manual floors across ordinary reorder/moves.
Only reported ordered untabbed groups of 2–128 members with a meaningful physical
axis enter execution. Output/presentation/primary geometry are independent context;
title, activation and secondary outer-envelope edits are not context resets.

Relevant geometry must be quiet before `FrozenGeometry::acquire`. Its measured
content budget B and inter-content strips survive own partial commits, focus
changes and handled replies. Target extents remain model-derived; actual committed
sizes and eventual coherence are separate. A disappearing mixed-time candidate
axis can use only C's validated frozen authority, never a relaxed acquisition gate.

`stock.rs` owns one command future, one optional timed windows refresh and one
epoch-bound capability probe. It processes each history item (eight per catch-up
turn), then verifies latest version/scope and an empty history immediately before
admission. Completion has priority. Each edge follows: select donor-ready step →
compile latest capture → B `begin_command` with **compiled capped travel** → model
`begin` with that exact key → construct/poll IPC. A position capture is installed
in B **before** its response; native receipt and position progress are independent
gates. Completed attempts are persisted before the one-slot mailbox is taken.

Real timers drive acquisition/guidance, geometry's 32ms position wait and B's
convergence deadline even with no duplicate snapshots. A silent edge requests
one same-stream windows refresh with a 32–250ms configured-timeout-derived bound;
the second geometry timer cannot classify absence while that future is pending.
Its reply installs through Tracker at its actual native receive position. The
runner drains history and assesses latest geometry, not a returned older capture.

Independent movement yields immediately, drains the owned future and reacquires
after quietness. Overlap/reversal while a receipt is outstanding can become
explicit **unattributed guidance**. A requested-boundary response coupled with
other boundaries, or incompatible commit-only geometry during own unsettled
execution, instead becomes limited. That hold retains the failed budget/reference
and ignores late commit-only changes as preferences. It cannot reacquire/retry
merely because the same failed geometry became quiet; independent target movement,
focus, topology or context can release it. This is conservative geometric
classification, not a stock causal tag or a claim to know client constraints.

Quiet accepted guidance teaches the emphasized member's focus share and visible
sibling ratios, not its hidden neutral weight. Owned cumulative late commits
never teach. Blur and shutdown solve **current neutral preferences**, not original
process-start pixels or native weights. Active user motion wins; bounded shutdown
can leave achieved geometry in place. A shutdown/diagnostic failure revokes unsent
work and drains the owned future within the remaining shutdown budget; expiry is
recorded possibly-sent with no replay. An admitted intent whose future was never
constructed is genuinely unsent.

Transport recovery is native-owner reconnection (250ms delay, at most eight
attempts per loss), with a fresh epoch and newly current families/probes. Old
goals/preferences/geometry do not cross epochs. Fresh acquisition from pixels may
contain an old emphasis; hidden neutral continuity is not claimed. Pending receipts
and completion evidence drain **before** new capability/execution authority.
Same-epoch healthy snapshots cannot undo a processed loss. Protocol/observer-lag
terminal failures stop instead of silently regaining continuity.

Stock has no 32-episode/64-effect lifetime veto: live preferences, live geometry,
one finite attempt and completion mailbox plus cumulative counters replace the
historical ledger. JSONL `longcat-feedback`, `longcat-edge-intent`,
`longcat-edge-command`, `longcat-feedback-completed`, refresh/capability and stopped
rows expose preferences, Taffy goal, reported versus inferred facts, native
fraction, guidance, achieved geometry and limitation. Per-turn assessment strings
are capped at 32 with an explicit omitted count. Dry-run uses a hypothetical
controller lifetime with zero command/handling evidence, not executable work.

## Episode routes: niri and explicit conditional control

The following baseline/retention semantics apply to these preserved routes, not
to ordinary stock Feedback.

### Observation and baseline bridge

The native source owner stays alive until joined shutdown. Its I/O drains
independently. The app waits for current windows **and** workspaces, reported
ordered membership, tiled placement, vertical secondary axis and usable content
geometry. Native source/epoch, workspace, group position and ordered member IDs
all survive the bridge. A same-numbered column on another workspace is not the
same group. Explicit focus none requests conditional restoration; unknown focus
does not invent blur.

For niri, the content budget is captured once as the sum of baseline committed content
heights. Subsequent observations retain that budget while carrying **actual**
per-member extents, including temporary partial-write sums. It is never recalculated
from the requested targets or from a partially converging native capture.
Independent context checks compare workspace output, per-member content width,
tile/content vertical decoration difference (two-pixel tolerance), and group span
only when its geometry evidence is reported. Niri's inferred tile bounding span
is deliberately ignored as an independent context signal. Output-height changes
not exposed independently cannot be proven from this feed; incompatible geometry
or convergence expiry suspends rather than learns a replacement budget.

### Explicit conditional Umbriel native authority

Umbriel's control capture subscribes only to `layout-state`, composed through the
upstream reusable `Tracker`. Active identity, membership, committed sizes, native
targets, constraints and complete basis originate in that one snapshot. Immediate
control does not consume the activity service's five-second dwell or journal.
Only supported untabbed groups with a vertical secondary axis enter Longcat.

The solver receives native tile row budget minus member title-bar heights,
content minimum/maximum constraints (also subtracting title bars), and the stricter
configured minimum. Original rounded edge gaps are preserved. Acquisition waits
for baseline committed content to match native targets. The native original raw
weights and target contents are retained independently of the conserving solver
baseline. That native restore target vector can legitimately have a different sum.

One emphasis request submits the complete Taffy vector, with title bars added back
to native row weights and native exact-content preview validation enabled. One
restore request submits the original raw weights and native baseline targets.
Both compare against the **last accepted native basis**, never newly observed
external weights. A receipt advances that authority only after validation; the
runner also waits to observe the accepted basis before treating geometry as settled.
Old-basis snapshots in that bounded interval do not become a new baseline.

The conditional predicate includes exact weights, so a manual edit hidden by
pixel rounding still revokes authority. Source loss and uncertain transmission
also revoke it without replay. Current-state ABA remains the native API's explicit
limit. Late receipts remain evidence and never revive a lost episode.

## History-gated admission and scheduling

The direct `kubeless_work_schedule::Schedule` has a single coalescing key and no
stream-driver intake loop. Command completion, SIGINT and shutdown deadlines get
priority over more intake. Each history catch-up turn drains at most eight queued
updates, checks **every** continuity marker, then yields/rearms if more remain.
Policy never jumps from an old episode directly to a healthy `current()` across
unprocessed history. An intervening discontinuity revokes authority even if the
newest snapshot is healthy in the same epoch. The initial Umbriel epoch-zero to
first-native-epoch acquisition is recognized separately, before any episode.

Immediately before a send, any newly queued event or a version/scope mismatch
defers admission to another bounded history turn. The native actuator additionally
requires exact current capture version, narrowing the publication/send race but
not providing remote compare-and-swap. Lag/error/loss stops control; no automatic
reconnect or baseline acquisition across that gap is attempted.

## Preview, effects and timers

- Dry-run uses `Episode::preview`: immutable baseline, hypothetical plans, no
  requested effects, no fake `Handled` evidence and no mutation calls.
- Actual operation uses `decide`. At most one sequence is active globally, with
  one owned command future. Before its first send, `Plan::validate` checks full
  geometry; between commands `validate_identity` permits self-induced geometry
  but still checks focus/topology/context. Every command uses a fresh admitted
  native basis. Changed focus/topology stops the definitely-unsent tail.
- Sequential native outcomes are installed with `record_command`; `finish_execution` is
  called only when no further command can be submitted. Native handling is not
  visible convergence. `PossiblySent` is retained and never blindly replayed.
- Umbriel uses one `record_group` outcome and a `longcat-group-command` diagnostic
  per actual whole-column request. It does not fabricate N separate native sends.
- Observation continues while a command awaits its reply. A real schedule timer
  drives `next_reconsideration` even after the final native event. Completion
  evidence also triggers a new decision. `Noop::NotSubmitted` prevents identical
  wholly-unsent attempts from busy retrying.
- SIGINT or the capture limit latches `request_restore` and follows the same
  evidence/observation/deadline protocol. It does not write stale baseline pixels
  unconditionally. At the shutdown bound, an outstanding command is conservatively
  recorded as possibly sent, its tail remains unsent, and authority is revoked.

Episode-route retention is explicit: at most 32 episodes, 128 members per group, 64 effects per
episode; reaching a bound stops instead of pruning evidence or silently rebasing.
Lost/conflicted episodes remain in the bounded runner lifetime. New groups may
not acquire members overlapping retained possible effects unless restoration
already completed. Restart/reacquisition is deliberate, not automatic cleanup.

## Diagnostics and verification limits

JSONL records contain native readiness/health, committed content/tile geometry,
baseline, desired decision, preview flag and actual requested execution evidence.
Observed window detail is capped at 128 rows with explicit truncation/count fields.
Command receipts and final retained-effect summaries are separate records.
The output queue holds 16 records, each at most 2 MiB, plus at most one row
owned by the writer. Overflow or broken output stops control rather than silently
discarding diagnostics; final output may itself be unavailable after a consumer
failure. A plain OS thread writes a duplicated stdout descriptor, without holding
the global stdout lock or using Tokio's blocking pool. It owns diagnostics only,
never native command futures or receipts. Native cleanup still runs first (up to
three seconds); total output drain then has a two-second deadline. This is **not**
a per-write deadline or a two-second bound on the entire command lifetime.
On drain timeout the writer remains detached, potentially blocked until process
exit; it is neither joined nor cancelled. The CLI can exit without waiting for
that ordinary thread. An idle source still requires SIGINT or another normal
termination condition; a blocked row alone does not manufacture a shutdown signal.

Runner replay/scripted-peer tests cover both backend acquisitions, by-ID native
execution and conditional restoration, no-focus, cross-workspace moves, immutable
partial-write budget, timer expiry in silence, dry-run non-submission, partial
prefix cancellation, uncertain pending command on loss, queued loss followed by
healthy latest state, and file/environment/CLI precedence. An unread-pipe
subprocess test uses padded diagnostics from a scripted read-only source: the
former Tokio writer times out but hangs during runtime shutdown, whereas the
detached writer reports drain timeout and exits within the test's five-second
bound. No live desktop
mutation was performed. Native handling/compatible pixels alone do not prove
causal attribution. Umbriel additionally validates a complete native basis for
raw-weight restoration; niri retains its conditional pixel-restoration semantics.

Stock runner scripts additionally cover both axes, deduplicated quiet timers,
receipt ordering, topology/focus cancellation, loss/fresh acquisition, same-epoch
loss holds, current-preference resize/blur/focus return, cumulative late commits,
coupled constraints, output changes, >64 focus changes and bounded shutdown/output
failure drain. These are scripted peers, **not integrated native acceptance**.
The full ordinary stock application-to-native user-guidance check remains open;
A/C's isolated native boundary/geometry tests are separate receipts.

### Isolated native integration

The added Umbriel check runs the real ZK executable against the separately built
native extension, under Umbriel's contained headless harness. It enables forced
title bars and nonzero edge gaps, verifies focus-driven emphasis and intra-group switching, restores
exact original raw weights on shutdown, and confirms a same-pixel external weight
edit prevents automatic restoration. All client windows and native writes belong
to the harness instance; the running desktop is not involved.

```sh
cargo build -p zooming-kittens --no-default-features --features umbriel
ZK_TEST_BIN="$PWD/target/debug/zk" CHECK_DIR="$PWD/app/zk/tests/headless" \
  bash ../umbriel-longcat-control/tests/harness/check.sh \
  ../umbriel-longcat-control/build-debug/umbriel umbriel_longcat -j 1
```

For the composed candidate that predates server-side title bars, set
`ZK_NATIVE_TITLEBARS=0` and select `../umbriel-longcat-deploy/build-debug/umbriel`.
This changes only the test's title-bar configuration. Nonzero edge gaps and all
sizing, restoration and external-edit assertions still run. See the
[prepared deployment record](../../../../design/longcat/deployment-candidate0.gpt6a.md).

The native contract is in
[`layout-control.md`](https://github.com/rektide/umbriel/blob/96b118cf/docs/user/layout-control.md).
Carry into the user's composed compositor and the graphical-session transition
remain `zk-longcat-return-native-deploy`; passing a headless test is not deployment.

The real-checkout feature matrix passes `cargo check --locked --all-targets` for
all eight combinations of `niri`, `umbriel` and `kitty`, plus `export-facet` and
all features. Existing backend-feature tests, service CLI tests and service-unit
generation tests pass in every configuration. Niri builds without kitty currently
report an unused optional `tokio-stream` manifest dependency; this is a warning,
not a backend isolation or build failure.
