---
type: ImplementationContract
title: Longcat application runner
description: Native acquisition, history-gated admission, retained baseline budgets and bounded conditional shutdown.
resource: app/zk/src/runner/README.md
status: implemented
generated: { at: 2026-10-15, by: 'model:openai/gpt-6.1-sol#medium' }
sources:
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
waits, command receipts, admission refusals or final shutdown evidence. Niri and
legacy Umbriel wait for both inventory families; control-capable Umbriel waits for
its coherent layout family. A silent/unready peer still requires SIGINT;
there is no periodic acquisition polling. `--socket` overrides native discovery.
Umbriel probes the read-only `layout-state` query. A compatible version-1 server
enables native conditional group control. An older server's explicit unsupported
reply permits legacy read-only acquisition under `--dry-run`; mutation explains
the missing extension and stops. Transport/protocol failure is not a reason to
fall back or attempt a focus workaround.

## Configuration

The app loads its existing file and `ZK_` environment configuration without global
CLI defaults, then applies only optional Longcat overrides. Invalid configuration
is an error, never a fallback. The two positive finite active/other weights are
normalized as `active / (active + other)`; for example `3` and `2` mean `.6`.
`minimum_height` remains a per-member pixel minimum. `resize_debounce_ms` is
preserved and reported, but automatic manual-resize learning is disabled. It is
**not** reused as a convergence deadline. `--convergence-timeout-ms` is independent;
`--shutdown-timeout-ms` bounds conditional restoration and pending-receipt drain.

## Observation and baseline bridge

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

### Umbriel native authority

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

Retention is explicit: at most 32 episodes, 128 members per group, 64 effects per
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

The native contract is in
[`layout-control.md`](file:///home/rektide/src/umbriel-longcat-control/docs/user/layout-control.md).
Carry into the user's composed compositor and the graphical-session transition
remain `zk-longcat-return-native-deploy`; passing a headless test is not deployment.

The real-checkout feature matrix passes `cargo check --locked --all-targets` for
all eight combinations of `niri`, `umbriel` and `kitty`, plus `export-facet` and
all features. Existing backend-feature tests, service CLI tests and service-unit
generation tests pass in every configuration. Niri builds without kitty currently
report an unused optional `tokio-stream` manifest dependency; this is a warning,
not a backend isolation or build failure.
