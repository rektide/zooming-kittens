---
type: ImplementationContract
title: Longcat application runner
description: Native acquisition, history-gated admission, retained baseline budgets and bounded conditional shutdown.
resource: app/zk/src/runner/README.md
status: implemented
generated: { at: 2026-10-15, by: 'model:openai/gpt-6.1-sol#medium' }
sources:
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
waits, command receipts, admission refusals or final shutdown evidence. Thus one
waits for both native families. A silent/unready peer still requires SIGINT;
there is no periodic acquisition polling. `--socket` overrides native discovery.
Umbriel mutation is rejected before opening a socket: current native actions are
focus-targeted, not by ID, and exact native grouping/axis/restore state is absent.
Read-only Umbriel acquisition emits an explicit unsupported planning result rather
than deriving fictitious groups from window geometry.

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

The content budget is captured once as the sum of baseline committed content
heights. Subsequent observations retain that budget while carrying **actual**
per-member extents, including temporary partial-write sums. It is never recalculated
from the requested targets or from a partially converging native capture.
Independent context checks compare workspace output, per-member content width,
tile/content vertical decoration difference (two-pixel tolerance), and group span
only when its geometry evidence is reported. Niri's inferred tile bounding span
is deliberately ignored as an independent context signal. Output-height changes
not exposed independently cannot be proven from this feed; incompatible geometry
or convergence expiry suspends rather than learns a replacement budget.

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
- Native outcomes are installed with `record_command`; `finish_execution` is
  called only when no further command can be submitted. Native handling is not
  visible convergence. `PossiblySent` is retained and never blindly replayed.
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
The output queue holds 16 records, each at most 2 MiB, with a two-second write
deadline. Overflow or broken output stops control rather than silently discarding
diagnostics; final output may itself be unavailable after a consumer failure.

Runner replay/scripted-peer tests cover both backend acquisitions, by-ID native
execution and conditional restoration, no-focus, cross-workspace moves, immutable
partial-write budget, timer expiry in silence, dry-run non-submission, partial
prefix cancellation, uncertain pending command on loss, queued loss followed by
healthy latest state, and file/environment/CLI precedence. No live desktop
mutation was performed. Native handling/compatible geometry do not prove causal
attribution or exact compositor-weight restoration.
