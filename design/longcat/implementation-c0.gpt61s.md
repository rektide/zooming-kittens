---
type: ImplementationReport
title: Frozen stock geometry and corrective edges — slice C receipt
description: Implemented frozen native content geometry, donor-ready selection, qualified mixed-time admission, ordered refresh and real headless correction tests; interfaces and obligations for D.
resource: design/longcat/implementation-c0.gpt61s.md
status: checkpointed
generated: { at: 2026-11-12, by: 'model:openai/gpt-6.1-sol#high' }
sources:
  - resource: opencode://ses_e32e12889ffeQe51yGGtHKv78g
    title: Implement slice C geometry, donor selection, native admission and refresh; verify isolated native corrections and preserve the ordinary route for D
  - resource: feedback0.gpt6a.md
    title: Selected feedback design and commissioned C success tests
  - resource: implementation-a0.gpt61s.md
    title: Actual stock capture, command boundary and frozen-admission continuation
  - resource: implementation-b0.gpt61s.md
    title: Actual goal, motion key, receipt, completion and clock contracts
  - resource: ../../desktop/umbriel/src/geometry.rs
    title: Implemented frozen budget, strips, cumulative prediction, scale and deadlines
  - resource: ../../desktop/umbriel/src/edge.rs
    title: Strict ordinary admission and separately validated frozen authority
  - resource: ../../layout/longcat/src/feedback/edges.rs
    title: Pure donor-ready prefix residual selection with contextual blockage
  - resource: ../../desktop/umbriel/src/source.rs
    title: Timed same-stream refresh through native ownership and Tracker installation
  - resource: ../../app/zk/src/runner/native.rs
    title: Typed stock executor mapping without startup-route changes
  - resource: ../../desktop/umbriel/tests/native_geometry.rs
    title: Taffy goals and real qualified correction on the isolated compositor
  - resource: file:///home/rektide/src/umbriel-rs/crates/umbriel-client/src/driver.rs
    title: Refresh normalization at the actual subscription reply receive position
  - resource: file:///home/rektide/src/umbriel-working/src/config/resolve.cpp
    title: Actual output workspace-axis control of physical scrolling orientation
  - resource: file:///home/rektide/src/umbriel-working/tests/harness/check.sh
    title: Existing contained headless compositor lifecycle, not the desktop socket
verification:
  method: Scoped Nextest, strict all-target Clippy, Rustdoc, owned-file formatting, scripted native peers and the real isolated headless harness.
  result: 178 ZK desktop/layout/application tests and 55 native client/tracking tests pass. Eight real geometry scenarios and the existing stock-boundary check pass; the positive-strut three-member round trips also assert exact committed Taffy vectors. Owned Rust formatting, Rustdoc and document checks pass.
  limits: No ordinary stock runner composition, service change, live desktop write, compositor-source change, full final feature matrix, installation or push. Native fixtures use a four-second per-goal test bound; D must integrate configured production deadlines and user-guidance acceptance.
extensions:
  ticket: zk-longcat-return
  related_work: [zk-longcat-user-layout-changes]
---

# Slice C delivered

[Assignment C](feedback0.gpt6a.md#c--geometry-compiler-and-corrective-edge-execution)
is implemented. `zk-umbriel` is now **0.2.0**, `zk-longcat` **0.3.0**; application
receiving locks are updated. Stock still does **not** become the ordinary runner
route in this slice. Niri and the explicit bespoke implementation remain intact.

## Checkpoints

| Repository | Commit | Stage |
|---|---|---|
| umbriel-rs | `64254f93` | Same-subscription windows refresh published at its actual reply receive position, with an ordering regression. |
| umbriel-rs | `30fb116e` | Format the refresh interfaces and regression. |
| zooming-kittens | `428fc6f4` | Frozen geometry, private native authority, pure donor selection, source refresh and initial regressions. |
| zooming-kittens | `ecece1a6` | Bounds-versus-tolerance correction, cumulative late-commit/B integration regression, tighter post-response attribution and typed runner mapping. |
| zooming-kittens | `ff8364b1` | Real headless arrangement fixtures, constrained/delayed xdg client and exact committed-vector assertions. |

The ZK starting working copy was clean. Beads auto-export subsequently refreshed
existing ticket history; it is preserved, not rewritten. The sibling native
repository's pre-existing `.beads/issues.jsonl` change remains uncommitted and
unmodified by this implementation. No commits were squashed or pushed.

## Implemented geometry and actuation

[`FrozenGeometry`](../../desktop/umbriel/src/geometry.rs) captures ordered scoped
members, inferred axis, output/presentation/primary geometry, content start/end,
**B = sum of quiet committed content extents**, and measured inter-content strips.
It never interprets a strip as a reported native gap or guesses title-bar height.
For each current position vector it derives target extents using those strips
and the residual B; committed extents remain separate in `GeometryFrame`.
`coherent` additionally checks committed agreement and the final content endpoint
within the configured tolerance. Partial commits cannot recalibrate B or strips.

The same model supports both physical axes. Qualified commands always use the
cross-end of a member with an actual following member: height-down vertically,
width-right horizontally, also for negative travel. Ordinary `edge::prepare`
retains A's candidate-axis gate. `prepare_frozen` and `execute_frozen` require a
private model-issued authority token bound to the current capture, ordered
members, geometry and unchanged native context. They can admit an otherwise
ambiguous **mixed-time own** candidate; they do not admit missing families,
unknown acquisition axes, changed presentation/output, stale bases or outer edges.
The post-connect local recheck validates the frozen token again.

[`select_edge`](../../layout/longcat/src/feedback/edges.rs) is pure and
adapter-neutral. Taffy's vector is its input, not replaced by native fraction
policy. It recomputes prefix residuals from latest target estimates, caps by
donor slack and reported recipient capacity, prefers the largest realizable
correction, and ties by row index. Blocked direction evidence is scoped to this
goal and adjacent estimates; either adjacent extent changing reopens assessment.
No unreported client minimum is learned from a clamp.

Compilation caps the first useful move at 16 estimated pixels using output
workarea, not a claimed exact tiledArea. An isolated positive-gain response
calibrates empirical travel/fraction, including initial strut overshoot. Overshoot
after calibration invalidates the gain and damps the next step. Tiny rounded
responses do not overwrite an established gain. Protocol fractions stay finite,
nonzero and within ±0.9. Native feedback, donor slack, B's submission limit and
the separate convergence deadline bound correction; none promises exact native
weights or unconstrained reachability.

An isolated target response extends bounded O(n) cumulative late-commit
corridors and carries the latest command's `MotionId`. Clients may commit old
or intermediate owned extents before catching up to the cumulative target.
Corridors collapse per member on catch-up. Further target movement **after** an
observed position response needs a new intent; a later same-boundary drag is not
silently swallowed as unlimited owned motion. Topology, epoch, independent context,
primary envelope and incompatible geometry retire the model. Attribution remains
geometric compatibility, never proof of human or compositor causality.

### The bounded B correction

The real positive-strut fixture produced `[372,102,98]` against desired
`[372,100,100]`, with a configured 100px floor. B formerly considered the two-pixel
mismatch converged and its `begin_command` gate rejected the small corrective
edge. The correction keeps effective bounds distinct from positional tolerance
in selection, convergence and command admission. A sub-tolerance correction is
allowed when needed for those bounds. The added pure regression also asserts
that preferences/generation do not change. No other B lifecycle, motion-key,
mailbox or receipt contract was weakened.

## Refresh ownership and timer contract

There is no cross-socket query splice. Native
`Subscription::refresh_windows(epoch, deadline)` sends the query through its
existing bounded command owner. On success that owner normalizes the payload to
a windows observation **at the reply's actual session receive position**, before
completing its waiter. Events before/after it preserve their socket order. This
is explicitly documented normalization, not an invented subscription event,
native revision or atomic windows/workspaces/outputs join. Ordinary requests do
not acquire this installation behavior.

Tracker/Inventory remain the sole typed row owner. ZK's
`Source::windows_refresh()` returns an optional no-queue refresh handle for stock;
`refresh(scope, timeout)` returns only after the corresponding position, or a
later windows census, is installed in the semantic source. Rejections, malformed
payloads, epoch changes, owner closure and timeouts are not successful freshness.
Malformed refreshes follow ordinary Tracker family invalidation. A source
shutdown closes refresh-capable native handles too.

Geometry's first position deadline is **32 ms**. `on_timer` requests exactly one
refresh, then supplies a second bounded timer outcome (`Blocked`) rather than
repeating an identical delta. `next_deadline_ms` exposes the real wakeup; an
invalidated model has no further execution timer. The application must continue
draining observation and the owned command/refresh futures. Do not declare the
query absent before its own future/deadline is classified; after installing its
result reassess current geometry before recording blockage. B's convergence
deadline is a separate upper bound and can end the goal while a receipt drains.

## D call sequence and boundaries

1. Keep A's full live census and joined health. Establish relevant geometry
   quietness before `FrozenGeometry::acquire`; that function validates eligibility
   but does not own the quiet timer or prove an unreported client commit.
2. For current captures, call model `observe(capture, now_ms)`. Project its frozen
   axis, B, starts, **actual committed** extents, model `target_extents` and
   coherence into B's `FeedbackGroup`. Do not omit an own-progress group merely
   because A's fresh candidate axis temporarily disappeared. Use the separately
   validated model, not a globally relaxed group capability.
3. Map native `MotionId { attempt, sequence }` exactly to B's `CommandKey`. On
   compatible observations, install `Feedback::observe` with `Motion::Owned(key)`
   **before** `record_response`. Use the latest capture version, not the goal's
   creation version. Keep independent output/presentation context separate from
   learnable user outer-envelope edits, as B's handoff specifies.
4. On `Drive(goal)`, call `select_edge(goal, latest_target_extents,
   feedback.current())`. `PositionsMatch` means wait for client commits/quietness,
   not native handling or automatic settlement. For `Limited`, call
   `limit_current(..., LimitReason::Executor { reason })` and reconsider; do not
   submit another edge or learn the failed goal as guidance.
5. Compile the selected step against the exact latest capture. Then call B's
   `begin_command` using **compiled.travel** (the capped pixel intent), followed
   by model `begin` with that key, all before starting IPC. Record a genuine
   unsent outcome if local compilation/admission fails after intent admission.
6. Obtain `source.edge_actuator().await` through A's explicit epoch-bound grammar
   verification. The new runner `stock_executor(actuator)` maps
   `Request::Edge(Box<CompiledEdge>)` to qualified transmission outcomes. It is
   deliberately not installed by ordinary startup yet. Own/drain the future and
   record its outcome even on supersession/loss; model progress never fabricates
   a receipt. B still gates a later intent on **both** outcome and response.
7. Arm model deadlines alongside `Feedback::next_reconsideration`. On
   `RefreshWindows`, perform one timed source refresh while observation continues;
   use the latest installed capture, not an older returned snapshot if newer
   captures have already arrived. On `Blocked`, record the same-current-version
   blocked response in B. Its adjacent estimates become selector evidence.
8. Retain the model/cumulative last key after attempt completion for late commits;
   drain B's finite completion mailbox through the existing bounded diagnostics.
   Do not retain an older per-edge key as authority after newer predictions.
   Retire geometry authority on scope/topology/context changes and clear it on
   loss. A handled/no-response edge is limited, never automatically replayed.

**Incompatible geometry needs an explicit decision, not a fallback.** Model errors
revoke further compilation. An independent edit should yield and reacquire after
quietness; an ambiguous overlap can become B's unattributed guidance after its
receipt drains. Coupled geometry seen during a constrained native operation must
instead finish that execution attempt as limited/reassessment, without teaching
the failed result. The native fixture demonstrates that conservative bounded
outcome, not causal detection of an unreported client bound. Rebuilding execution
geometry alone is not an independent input authorizing endless retries or new
preferences. D owns this application-level classification and limitation hold,
along with ordinary routing, recovery, restore/shutdown and diagnostic persistence.

## Verification receipts

- `cargo nextest run --locked -p zooming-kittens -p zk-desktop -p zk-niri -p zk-umbriel -p zk-longcat -p zk-layout`:
  **178 passed**, two opt-in native tests skipped. New pure cases include donor
  chains both ways, heterogeneous three/four-member vectors, blocked-edge
  reopening, recipient maxima, bars/strips, both physical axes, frozen mixed-time
  authority, cumulative late commits, post-response drag, silent timers, bounds
  admission, ordered refresh and a complete geometry-to-B slow-commit regression
  retaining the exact preference vector/generation.
- Same six-package warning-denying all-target Clippy: passed. Warning-denying
  Rustdoc for `zk-longcat`/`zk-umbriel`, explicit owned-file default-style rustfmt
  checks and their doctest commands pass (currently no doctest cases).
- Native `umbriel-client`/`umbriel-tracking`: **55 Nextest tests**, **two doctests**,
  warning-denying all-target Clippy and explicit owned-file rustfmt checks pass.
  The new peer fixture surrounds a refresh reply with windows events and asserts
  exact receive positions; remote rejection installs no replacement snapshot.
- `bash desktop/umbriel/tests/native/check.sh`: **both checks pass** on isolated
  Umbriel `d34b33c7775200de31eb9aa425726af6516523c1`. No native compositor source
  changes. The existing A stock-boundary test still passes. The geometry check
  runs eight real scenarios: positive/negative struts on both physical axes,
  three/four-member mixed-title-bar groups with nonzero outer/inter-member gaps,
  120 ms delayed xdg commits, a fixed middle member, a recipient maximum and a
  donor minimum. Unconstrained cases complete within submission bounds with
  stable B/strips/outer envelope and unchanged seat focus/unrelated anchor.
  The positive-strut three-member fixture asserts **exact final committed Taffy
  vectors** in both chain directions and heterogeneous allocation; other cases
  use two-pixel tolerance while checking the configured floor separately.
- The fixed middle fixture exhausts donor-ready unblocked routes; actual min/max
  fixtures stop on incompatible coupled native geometry. They do not loop or
  pretend to have converged. The local C client supplies real xdg min/max hints
  and delayed commits; no Rust copy of native arrangement equations substitutes
  for those claims. Current native orientation is configured through output
  `workspace_axis`, not the obsolete silently ignored scrolling-direction key.
- Final native geometry check took **12.95 s**, over that harness's advisory
  eight-second budget, but passed. The fixture has an explicit finite per-goal
  bound and eight cases; it is not a production runner timing claim.
- Umbriel-only and niri-only application all-target checks pass. Niri-only still
  emits the inherited unused optional `tokio-stream` manifest warning. D retains
  ownership of the full final feature matrix after composition.

## Acceptance still with D and parent review

This supplies the geometry/compiler/refresh boundary and native corrective
evidence, not an accepted deployed controller. The selected full acceptance still
needs **live stock subscription → Feedback/Taffy → qualified correction → observed
convergence → accepted user adjustment → continued focus behavior** through the
ordinary application route, plus reconnect, shutdown, churn and bounded output.
Native tests here use a fixture-owned corrective loop, not that unfinished route.

[A's admission continuation](implementation-a0.gpt61s.md#continue-with-b-then-cd)
is addressed without weakening unknown-axis acquisition.
[B's public sequence](implementation-b0.gpt61s.md#public-call-sequence-for-cd)
remains authoritative for receipt installation, keys, monotonic clocks and the
completion mailbox, with the small bounds correction documented above.
The [Longcat index](README.md#delivered-related-integration) connects these slices
without promoting implementation checkpoints to product acceptance.
