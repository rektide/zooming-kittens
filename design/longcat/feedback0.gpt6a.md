---
type: Design
title: Longcat as a Taffy-driven desktop feedback loop
description: Selected stock-IPC controller, explicit user guidance, boundary actuation, and dispatchable implementation assignments grounded in the existing Rust and native sources.
resource: design/longcat/feedback0.gpt6a.md
status: proposed
generated: { at: 2026-11-11, by: 'model:openai/gpt-6-astra#high' }
sources:
  - resource: opencode://ses_e33541051ffeer2RcnuC2Rn4rj
    title: Independent source assessment and selected implementation-ready Longcat feedback design
  - resource: handoff0.glm53m.md
    title: Working-line idtarget availability and preserved bespoke control family
  - resource: ../../ARCHITECTURE.md
    title: Existing domain ownership and implemented integration
  - resource: ../../desktop/README.md
    title: Native observation, identity, health and transmission contracts
  - resource: ../../layout/README.md
    title: Taffy allocation and current immutable episode contract
  - resource: ../../app/zk/src/runner/README.md
    title: Current acquisition, scheduling, native authority and retention behavior
  - resource: ../../layout/solver/src/lib.rs
    title: Actual zero-basis Taffy allocation and conserving integer rounding
  - resource: ../../layout/longcat/src/lib.rs
    title: Current terminal conflict states, immutable baseline and execution evidence
  - resource: ../../desktop/model/src/lib.rs
    title: Semantic geometry, group evidence and capability representation
  - resource: ../../desktop/umbriel/src/reducer.rs
    title: Current stock projection drops grouping and declares read-only capability
  - resource: ../../desktop/umbriel/src/source.rs
    title: Independent subscription owner and stock versus layout-state family selection
  - resource: ../../app/zk/src/runner/control.rs
    title: Native basis checks, retained-effect admission and stopping bounds
  - resource: ../../app/zk/src/runner/bridge.rs
    title: Vertical-only observation bridge and frozen content budgets
  - resource: ../../app/zk/src/runner/native.rs
    title: Bespoke capability gate and application-owned execution requests
  - resource: ../../app/zk/src/config.rs
    title: Existing active share, minimum and 500 ms resize debounce defaults
  - resource: umbriel-control-stream0.gpt6a.md
    title: Earlier strict conditional-control implementation and evidence retained as precedent
  - resource: ../remake/architecture0.gpt6a.md
    title: Existing observation-policy-execution boundary and Taffy ownership
  - resource: file:///home/rektide/src/umbriel-working/src/server/ipc_commands.cpp
    title: Stock windows, workspaces and outputs payloads at d34b33c7775200de31eb9aa425726af6516523c1
  - resource: file:///home/rektide/src/umbriel-working/src/server/ipc.cpp
    title: Full-snapshot event construction and byte deduplication
  - resource: file:///home/rektide/src/umbriel-working/src/server/event_pacer.cpp
    title: Paced coalescing and final snapshot delivery
  - resource: file:///home/rektide/src/umbriel-working/src/workspace/workspace.cpp
    title: Tiled area, content target positions and focus-free qualified sizing
  - resource: file:///home/rektide/src/umbriel-working/src/layout/scrolling.cpp
    title: Arrange, fraction setting, internal boundary transfer and resize constraints
  - resource: file:///home/rektide/src/umbriel-working/src/layout/layout.cpp
    title: Strut-adjusted tiled area and edge padding
  - resource: file:///home/rektide/src/umbriel-working/src/config/fields_layout.cpp
    title: Layout struts permit both positive and negative offsets
  - resource: file:///home/rektide/src/umbriel-working/src/scene/ssd_frame_geometry.cpp
    title: Content origin and extent after title-bar subtraction
  - resource: file:///home/rektide/src/umbriel-working/tests/harness/checks/layout/id_resize_actions.sh
    title: Existing by-ID sizing, stale-ID and focus-isolation native checks
  - resource: file:///home/rektide/src/umbriel-working/tests/unit/keybind_parse.cpp
    title: Qualified sizing grammar and rejected malformed qualifiers
  - resource: file:///home/rektide/src/umbriel-rs/crates/umbriel-protocol/src/model/inventory.rs
    title: Typed stock model omissions at 38f4be7fd59c13b61c237029ca06b3074711028c
  - resource: file:///home/rektide/src/umbriel-rs/crates/umbriel-protocol/src/action/extent.rs
    title: Existing fraction/delta types and unqualified sizing constructors
  - resource: file:///home/rektide/src/umbriel-rs/crates/umbriel-protocol/src/action/mod.rs
    title: Action request encoding and raw escape hatch
  - resource: file:///home/rektide/src/umbriel-rs/crates/tracking/src/lib.rs
    title: Reusable Tracker with Inventory-owned typed installation and family health
verification:
  method: Read-only source inspection and repository document checks; no native sizing experiment in this wave.
  result: Rekon check --okf passed for this design and the Longcat index, including local destinations, anchors and session provenance.
  limits: Actuation convergence and guidance behavior below are specified implementation tests, not claimed runtime results. Native checkout revisions identify inspected ground, not the running desktop.
extensions:
  ticket: zk-longcat-return
  related_work: [zk-longcat-user-layout-changes]
---

# Longcat as a Taffy-driven desktop feedback loop

## Selected destination

**Build an ongoing Taffy controller over Umbriel's stock subscriptions and
`--window`-qualified internal-edge actions.** User rearrangement and resizing
change its inputs. Taffy remains the one allocation engine; the native adapter
turns its answer into bounded corrective steps and reports what happened.

This is a selected design offered for implementation, not an accepted change or
a new feasibility program. The existing implementation supplies most of the
ownership structure. The significant change is replacing its permanent
conflict/restoration episode with a revisable preference state and short-lived
execution attempts. The assignments below can start directly after parent
selection; no additional design wave is a prerequisite.

The product behavior is:

- Focus emphasizes the selected member using Taffy and the group's neutral
  proportions. Focus switching does not compound earlier emphasis.
- A user can resize while Longcat runs. Motion yields control; a quiet result
  becomes guidance. Resizing the emphasized member teaches its desired focus
  share, while changing the other members teaches their relative proportions.
- New, removed, moved and reordered members enter the next calculation. Old
  effects do not permanently disqualify surviving windows.
- Leaving a group removes emphasis using **current** neutral preferences.
  Shutdown does the same when observations remain usable. It does not undo
  accepted user adjustments by restoring the process-start layout.
- Native constraints can limit achievement. Longcat reports a limited result
  and stops that attempt instead of spinning or inventing a manual-resize event.

The ordinary target is untabbed scrolling groups with at least two members.
Tabbed, floating, pinned, minimized, fullscreen and maximized presentations are
represented but ineligible for these writes; returning to an eligible topology
automatically resumes normal acquisition. This is an explicit geometry scope,
not a permanent conflict state.

## Verified hook and semantic map

Inspected ground: ZK parent `b11c885f`, Umbriel working checkout
`d34b33c7775200de31eb9aa425726af6516523c1`, and umbriel-rs
`38f4be7fd59c13b61c237029ca06b3074711028c`. These facts refine the
[handoff](handoff0.glm53m.md); they do not identify the live compositor version.

| Hook / source symbol | Verified meaning | Consequence for this design |
|---|---|---|
| `IpcCommands::windows`, `windowEntry` (`ipc_commands.cpp:627–736`) | A full mapped-window census. Reports workspace, output, active identity, scrolling column/row, tabs and presentation flags. | Group by native placement, never shared x coordinates. `active` is seat activation; workspace `focused` is remembered focus. |
| `windowEntry:690–721` | `x/y` are presented **content target positions**, including preview of a pending arrange. `w/h` are committed content dimensions. | There is **no separately reported target size vector**. A mixed-time position/size rectangle is not a complete allocation basis. |
| `IpcCommands::workspaces:810–845` | Output and layout mode are reported; scrolling direction is not in this payload. | Join independently current families; infer the secondary physical axis from stable ordered positions and label that inference. |
| `IpcCommands::outputs:901–981` | Output logical size, position and `usable_area` are reported. | Observe output changes and obtain a starting scale for edge commands; do not call workarea the exact layout budget. |
| `Ipc::snapshotPacer`, `broadcastEvent`; `EventPacer` | Paced, deduplicated full snapshots, with a trailing emission after a burst. | Process state, not event counts. A quiet layout need not emit a second identical snapshot. Use timers for quiet/deadline decisions. |
| `Workspace::setFocusedHeight:2561`, `ScrollingLayout::setHeightFraction:1451` | One row's fraction changes its weight against all other weights, including edge gaps. Siblings and minima affect the result. | Setting each member to its final fraction in succession is not vector assignment. Do not use that as the main actuator. |
| `Workspace::resizeFocusedEdge:2625–2669` | Qualified action selects a view, opens native `ResizeGrab`, applies one total delta, and marks arrange. Delta is a fraction of **tiledArea**, with top/left sign reversal. Qualified calls omit focus reevaluation. | Use by-ID edges; never focus-dance. A reply means handling, not committed convergence. |
| `ScrollingResizeGrab::applyDelta:1719–1789` | An internal secondary boundary transfers weight between adjacent units; edge boundaries transfer to/from edge-gap weights. | Control only internal boundaries. Keep the outer gaps and column width out of ordinary emphasis. |
| `ScrollingLayout::arrange:1174–1247` | Uses edge padding, total gap, weighted edge gaps, rounding and per-view constraints; the last row gets a residual. Constraints can break simple proportionality. | The linear transfer derivation below is exact only in its stated idealization. Feedback is necessary. |
| `Workspace::tiledArea:845`; `applyLayoutStruts` | Tiled area applies workspace layout struts to output workarea. Struts may be negative. | `delta / output.usable_height` is a scale estimate, not a universally exact or conservative conversion. |
| `beginResize:1844–1847`, `resizeFocusedEdge:2664` | Resize may clear full-width saved state and maximized state. | Focus-free is not state-free. Exclude reported maximize modes; do not promise preservation of hidden full-width-toggle history. |
| Native `id_resize_actions.sh` | Existing harness checks qualified targets, stale IDs, floating targets and focus isolation. | Extend its test idioms for the controller rather than inventing a desktop automation harness. |
| umbriel-rs `model/inventory.rs`, `action/extent.rs` | Current typed windows omit placement/tab/output/presentation additions; no typed outputs model there; sizing constructors are unqualified. | Extend the upstream protocol/state owner first. Raw JSON retained elsewhere does not make these fields available to ZK's typed reducer. |
| `Tracker::observe` | Reusable decoding/install, per-family status and native windows change receipts. | Use this owner for the stock path as well as the existing coherent path. |

No inspected stock payload supplies a user-action tag, grab begin/end, exact
native weights, title-bar height, client min/max vector, or a causal request ID.
The controller therefore uses modest geometric attribution. It must call this
**accepted guidance**, not proof that a human caused a specific frame.

## State and ownership

Use three explicitly different records:

1. **Preferences:** source-epoch-qualified member identities, neutral weights,
   optional learned active share, and a preference generation. These are mutable
   policy inputs, never automatically replaced with our achieved emphasis.
2. **Desired allocation:** Taffy's integer vector for current membership, content
   budget, bounds, focus and preference generation. Derived and replaceable.
3. **Observed / execution state:** native facts, a quiet geometry reference,
   measured geometry constants, the current attempted goal, one pending native
   command, and its reply/position/commit observations. These establish what can
   be attempted next, not what the user prefers.

Keep `zk-layout` pure. Put preference updates and guidance decisions in
`zk-longcat`; native edge preparation, axis/geometry projection and transmission
remain `zk-umbriel` responsibilities. The application owns clocks, scheduling,
request futures and diagnostics. A new pure `Feedback` controller beside the
existing `Episode` is preferable to adding more mode flags to its irreversible
state machine. Wire Umbriel to `Feedback`; the niri episode contract can continue
through its existing execution path. This is a backend semantic distinction,
not two competing allocation engines.

Suggested controller states are `Acquiring`, `Ready`, `Driving`, `Yielding`,
`Limited`, and `Unavailable`. `Limited` holds for the current input generation;
focus, preferences, topology or independent context changes can make a new attempt
useful. `Unavailable` waits for eligibility/freshness. Neither means the window
has been permanently abandoned.

### Read → assess → write → reassess

1. Install each full native family into Inventory. Preserve source epoch and
   health; project one latest semantic capture. All three required families
   (`windows`, `workspaces`, `outputs`) must be current in the same native epoch.
   This is a local join, not an atomic server snapshot. Transient inconsistent
   membership/output associations wait for the next replacement.
2. Use a full windows snapshot for the whole group's membership and activation.
   On geometry/topology/context change, assess before issuing the next write.
   Title changes and unrelated output updates do not reset geometry quietness.
3. If geometry is unexplained, yield immediately. Stop the unsent tail; allow the
   owned command future to finish and preserve its outcome. Keep ingesting input.
4. Once relevant geometry has been quiet for `resize_debounce_ms` (existing
   default 500 ms), and own pending effects have settled or been classified,
   accept eligible guidance and recompute with Taffy. Acquisition also uses this
   quiet rule. A timer over a still-current capture is sufficient; dedup means
   two equal events cannot be required.
5. Admit one corrective edge from current facts. Record the command before
   awaiting it. Reassess its reply, target-position response and later committed
   geometry before another correction. Never queue the remainder as irrevocable
   IPC work.
6. On quiet convergence, retain achieved geometry as the **expected observation**,
   not as new preferences. On limitation, retain desired-versus-achieved residual
   and wait for a meaningful new input. Stable nonzero residual alone cannot
   schedule an infinite retry.

Reuse the runner's independent source drain, bounded input turns, one owned
mutation future, command-completion priority, monotonic timers and bounded
shutdown. The debounce and convergence timeout remain distinct.

## Canonical Taffy model and user guidance

### Allocation

One affected native group maps to the existing one-dimensional Taffy flex tree.
Each eligible member supplies a positive neutral weight and configured content
minimum. Native bounds are unknown on stock; do not fabricate them. Any learned
execution limitation is separately labeled empirical and scoped to its current
geometry/context, not promoted to a reported client constraint.

Acquire `B = sum(committed secondary content extents)` from a quiet eligible
group, then hold B through our internal-edge corrections. Pass
`Gaps::AlreadyExcluded`; the budget already excludes decorations and gaps.
An output/presentation/outer-envelope change reacquires B after quietness.
Intermediate client commits cannot change B. Use actual observed extents in
feedback even when their transient sum differs from B.

Neutral allocation is `allocate(B, weights, no emphasis)`. Emphasis is the same
solver with the focused member's learned share, or the configured default `.7`.
Keep zero bases, cascading bounds, deterministic conserving rounding, and
explicit infeasibility. Solver tree dimensions can remain scalar/internal
"height" while the adapter projects either physical secondary axis. Taffy owns
the requested distribution; actuator corrections do not replace it with a native
fraction policy.

### Teach the parameter the user actually changed

The chosen policy distinguishes neutral proportions from focus emphasis. A
resize during emphasis does not identify a complete neutral layout: the active
member's neutral weight was deliberately hidden by its focus share. Pretending
otherwise is the route to amplified baselines.

For an accepted quiet vector `g`, with `B = sum(g)` and active member `a`:

- If its content share changed beyond tolerance relative to the last settled
  expected observation, save `active_share[a] = g[a] / B`. Keep it for that
  member in this workspace/source epoch, including later focus returns. This
  design deliberately chooses a remembered preference over a focus-tenure-only
  override that would surprise the user on the next return.
- Preserve `weight[a]`. For siblings, preserve their previous total neutral
  weight `W_other` and set `weight[i] = W_other * g[i] / sum(g[other])`.
  Thus the visible sibling ratios teach the available neutral information.
- If no member is emphasized, update all neutral weights proportionally to the
  accepted vector, preserving their prior total. Preserve learned active shares.
- Do not learn from a matching owned response, from a mere focus transition, or
  from partial client convergence. Changes within tolerance are no input.

Positive extents and at least two members make these shares valid. If a user's
accepted geometry is below the configured minimum, lower that member's effective
policy floor to the accepted extent for this context: the configured default is
a default for automatic sizing, not a reason to bounce a manual resize back.
The compositor still enforces its real constraints. Reset this empirical/manual
floor allowance when the member's presentation/context changes.

Example: neutral `[400,300,200]`, focused A, B=900 produces
`[630,162,108]`. The user chooses `[540,216,144]`. Learn A's share `.6`; sibling
weights stay `300:200`, and A's neutral weight remains 400. Taffy reproduces the
chosen geometry; blur gives `[400,300,200]`; later focus on A again gives
`[540,216,144]`. Reobserving any of these owned results makes **no preference
change**. Resizing only the sibling boundary to `[630,200,70]` instead teaches
the sibling ratio `200:70` and leaves A's share unchanged.

This resolves the apparent choice between ignoring the user and copying our
own emphasis into the baseline. It also states a real semantic limit: editing
an emphasized member teaches its *focused* size, not its hidden neutral size.
Neutral size can be learned when the group is un-emphasized. These choices are
concrete defaults for the implementation, not questions blocking dispatch.

### Attribution that is useful without claiming causality

- While settled and idle, a change in relevant geometry starts `Yielding` and
  the quiet timer. Repeated movement keeps writes stopped. Focus changes do not
  bypass an active yield timer.
- During our edge operation, movement of its predicted internal boundary toward
  the requested position, with compatible adjacent size changes, is owned
  progress. Client sizes catching up to already observed target positions are
  also owned progress, even after the command reply.
- An unrelated boundary movement, reversal, movement past the requested result,
  or changed outer envelope cancels further correction and starts yielding.
  Native clamps can also cause unexpected geometry: classify the final outcome
  with the attempted command before deciding whether it is guidance.
- A handled command with no position progress is initially execution limitation,
  not a new user preference. After its observation deadline, mark the attempt
  limited. A later independent change can become guidance normally.
- If command and external motion overlap ambiguously, stop writing, drain the
  command, wait for quiet, and accept the final coherent user-visible vector as
  **unattributed guidance**. Record that classification. A small false attribution
  is preferable here to repeatedly fighting a drag or permanently disabling the
  group. Ordinary delayed commits matching the frozen prediction are excluded.

There is no stock grab-ended signal. A long pause inside a drag is
indistinguishable from its end. The 500 ms quiet interval and immediate yielding
on renewed motion are the chosen interaction policy, not a race-proof guarantee.
Tests must demonstrate no repeated correction throughout continuous drag input.

## Practical internal-boundary actuator

### Geometry that does not need unreported decoration sizes

Let `p[i]` be the target content start on the secondary axis and `h[i]` its
quiet committed content extent. Capture:

```text
gap_strip[i] = p[i+1] - p[i] - h[i]     for i = 0 .. n-2
B           = sum(h)
start       = p[0]
end         = p[n-1] + h[n-1]
```

For vertical secondary layout, `gap_strip[i]` includes total inter-row gap and
the **next** member's title bar: `contentBoxInTile` moves the content origin down
by its own bar. For horizontal secondary layout, the title bar is off-axis.
Keep this measured strip intact; do not label it a reported native gap or
subtract a guessed decoration height. Stable mixed bars are supported naturally.

With unchanged presentation and topology, an internal edge changes adjacent tile
heights by opposite amounts. Fixed decoration offsets cancel, so the same deltas
apply to content heights. Edge-gap weights remain untouched. From Taffy's desired
content vector `d`, compute desired internal positions:

```text
p_desired[0]   = start
p_desired[i+1] = p_desired[i] + d[i] + gap_strip[i]
```

The column's primary extent, outer gaps and its outer secondary content envelope
are preserved in the unclamped case. The stable captured B is the available
content allocation, not the whole output workarea. A user's outer-edge edit
changes the envelope and B; accept it after quietness rather than expanding the
column back to an assumed full-screen budget.

Target positions allow early progress checks before clients redraw. While B and
the strips remain valid, infer target extents for control as
`t[i] = p[i+1] - p[i] - gap_strip[i]`, with the last extent `B - sum(t[0..n-2])`.
These are **model-derived target estimates**, not reported sizes. Keep committed
sizes alongside them and require eventual agreement. Negative extents, changed
strips after committed settlement, or an inconsistent final endpoint invalidate
the model and start reassessment. Do not recalibrate strips on mixed-time frames.

Axis is inferred from the ordered multirow group's stable separated content
rectangles: rows advance on exactly one axis and overlap on the other. Report
membership as `Reported` and axis/geometry inference separately. Ambiguous or
overlapping arrangements wait rather than accidentally invoking a primary-column
resize. Horizontal strips use `width-right`; vertical stacks use `height-down`.
Both operate the cross-end edge of row i, always with an actual following row.
Never issue the last member's outer-edge action for this algorithm.

### Why cumulative error is useful, and why command order matters

In the unconstrained native math, `deltaWeight = travel / stackCross * totalWeight`.
Arrangement multiplies by `stackCross / totalWeight`: the changed row extent is
therefore `travel`. Pair weight total is conserved. Internal boundary i needs

```text
R[i] = sum(d[j] - t[j], j = 0..i)
```

Moving it by R[i] corrects that prefix; with invariant strips this equals
`p_desired[i+1] - p[i+1]`. A fixed top-to-bottom sweep is wrong under minima:
`[100,100,500] → [500,100,100]` first needs to transfer bottom space into the
middle, then into the top. The reverse sweep is wrong for the reverse problem.

Use **donor-ready boundary selection**:

1. Recompute residuals from current target estimates after every native response.
2. For positive R[i], donor is i+1; for negative R[i], donor is i. Cap the desired
   step by the donor's extent above its effective policy minimum and any known
   recipient capacity. Choose a boundary with nonzero transferable slack; prefer
   the largest realizable correction, tie by row index. This moves available
   space through the chain rather than asking an already-minimal donor to shrink.
3. Submit one qualified cross-end action for member i. Positive means down/right;
   negative means up/left *travel of that same end edge*. Do not switch to a
   different member's start-edge command to express negative residuals.
4. Reobserve before choosing the next boundary. Complete when positions and
   committed content match the vector within tolerance; expose each separately.

On a feasible unconstrained chain, each transfer changes only its corresponding
prefix residual and reduces its absolute value. This explains the algorithm,
not a convergence proof under unreported native bounds. A member fixed at both
bounds can prevent moving space through it with pairwise actions even when a
whole-vector assignment could jump to the final distribution. That is an honest
reachability limit of the selected actuator.

### Fraction scale, clamps and termination

Qualified action example (action grammar, not a shell-interpolated command):

```text
window-modify-height-down <signed-fraction> --window <upper-member-id>
```

Native fraction multiplies the strut-adjusted tiled extent `T`, not B and not
`stackCross`. Start with output usable extent U as a scale estimate. The first
correction for a new context is an actual useful move capped at 16 logical pixels
in that estimate, not a separate probe mode. Observe its target boundary travel.
For an isolated positive-gain pair response, estimate `T_eff = travel / fraction`
and use that gain for the next residual correction. Record it as empirical.
Discard it on context changes; never infer a scale from absent, wrong-direction
or clearly coupled/clamped responses. Limit each fraction to the protocol's
finite nonzero delta range, and cap large residual moves using current donor
slack. Negative struts can make the first move overshoot: feedback corrects it;
the output denominator is not advertised as an exact or conservative guarantee.

Use two-pixel position/content tolerance initially, matching the existing policy
scale. A reply plus target-position response can permit the next edge without
waiting for every client redraw; retain the original B/strips and the final
committed target vector until clients catch up. Allow at least two pacing
intervals (32 ms) for a position response before considering it absent, and use
the separate convergence deadline for final committed geometry. No repeated
identical delta on a silent stream: the timer can issue one fresh windows query
to disambiguate missing publication, then classify a non-response as limited.

Native constraints remain authoritative:

- For isolated partial pair progress, continue from the measured result. If a
  later attempt makes no progress, mark that boundary/direction blocked for this
  goal and its current adjacent geometry. Try another donor-ready edge; a change
  in either adjacent extent makes that blocked result eligible for reassessment.
  Do not assume a universal client minimum from one clamped operation.
- For systematic overshoot, reduce step size and invalidate the gain estimate.
  For changed suffixes/total content span, stop the chain and reacquire stable
  geometry; do not keep applying the linear model to incompatible observations.
  Reacquiring execution geometry does not itself authorize changing preferences.
- Bound a goal to `8 * (n-1) + 2` edge submissions and the configured convergence
  timeout. Stop sooner on no-progress across available boundaries. Emit
  `Limited { desired, achieved, blocked_edges, reason }`; do not learn our failed
  goal as a new active share or declare it restored.
- A limited result is an achieved reference for detecting subsequent edits.
  Retain the Taffy goal for diagnostics. A future focus/context/preference change
  gets a fresh attempt; mere stable mismatch does not.

Those numeric defaults are implementation parameters with specified tests, not
new user decisions or an adaptive-control research project. The priority tests
are ordinary stacks, heterogeneous neutral ratios, bars/gaps/edge gaps, client
minima, slow client commits and struts. Failure has an explicit bounded outcome.

## Rearrangement, recovery and restoration

**Topology:** membership/order comes from one windows census. A change stops the
unsent sequence and retires its old geometry model. Retain neutral member
preferences by scoped identity; reorder them with native rows. When a member
moves between managed groups, carry its neutral weight rather than its currently
emphasized observed extent. Initialize a genuinely new member from its quiet
observed content extent. Neutral weights are in acquired pixel-like units, so
merging groups has a useful starting scale. Reacquire the new group's B and
strips, then solve the latest topology and focus. Positional column renumbering
alone must not discard member preferences.

Retain a learned focus share across reordering and group membership edits within
the same workspace: its meaning is a share of the *current* group. A workspace
move seeds the member's neutral weight but resets its learned focus share, since
the working context changed. No snapshot of a user's new topology is replaced
with the old order. Transient unsupported tab/maximize states park the group and
resume on eligible quiet geometry; supported survivors can form new groups.

**Loss:** cancel unsent work immediately. A possibly-sent delta is never replayed.
Drain its future and retain the receipt classification. Reconnect/resubscribe
through the source owner, obtain current families in a fresh epoch, then acquire
from a quiet current layout. Old goals/restoration rights do not cross epochs.
Fresh acquisition can contain a previously applied emphasis; hidden neutral
weights cannot be recovered from stock pixels after lost continuity. Report
that acquisition honestly. Source loss need not force process exit, but recovery
must clear the old pending authority before new sizing.

**Blur/shutdown:** solve the current neutral preferences and drive them with the
same internal-boundary executor only while the group remains eligible and quiet.
An active user edit wins over restoration. A bounded shutdown can leave the
achieved layout in place. This is visible preference restoration, not bit-exact
restoration of native raw weights or hidden maximize history.

**Lifetime:** keep live preferences for observed identities, one current attempt,
its outstanding receipt and the last achieved reference. Finalize attempts into
the existing diagnostic stream and cumulative counters; historical attempts
must not be an admission lock. Explicit retirement on topology/epoch transition
replaces the current 32-episode/64-effect process lifetime trap. Preserve the
output queue limits and failure semantics; this design does not require an
unbounded in-memory audit trail or silently discarding unfinished evidence.

## Reuse/change map

| Owner | Reuse | Required change |
|---|---|---|
| `umbriel-rs/crates/umbriel-protocol` | IDs, request codec, typed events, fraction validation | Retain stock column/row/tab/output/presentation fields; type outputs/usable area; add qualified sizing constructors using the current native grammar and proper token encoding. Missing fields must mean unavailable, never invented column zero. |
| `umbriel-rs/crates/umbriel-state`, `crates/tracking` | Inventory as sole row owner, family health/epochs, change receipts, Tracker | Install typed outputs with its own freshness; extend window comparison/evidence for the new layout facts. |
| `desktop/model` | Scoped identity, semantic capture, before/after impact, observer | Represent output context, flags, inferred axis and a relative-edge capability honestly. Do not mark `height_by_id` true if that implies fixed-height assignment. Keep exact/atomic flags false for stock. |
| `desktop/umbriel` | Source lifetime, independent receive task, native library composition | Upgrade stock reducer through Tracker; project reported membership; subscribe to outputs; add by-ID edge preparation/transmission and stable geometry model. Keep raw native details here. |
| `layout/solver` | Entire Taffy allocator and rounding | No substitute solver. Feed new preference inputs and either-axis scalar extents; add fixtures only for newly required contracts. |
| `layout/longcat` | Pure policy ownership, clock injection, plans/outcomes, serializable diagnostics | Add feedback state, explicit preference acceptance and attempt completion; do not force mutable guidance into terminal-conflict Episode semantics. |
| `runner/bridge.rs` | ID qualification, latest observation conversion | Provide stock feedback inputs, current output context and explicit inferred geometry; stop requiring reported vertical-only group geometry for this path. Preserve legacy niri bridge semantics. |
| `runner/control.rs` | Central policy ownership, admission, diagnostics | Compose feedback controller for stock Umbriel; remove its overlap/retained-history veto, immutable-context loss and 64-effect halt. One attempt owns current execution; preferences survive normal rearrangement. |
| `runner/native.rs` | App-owned request/receipt boundary | Add edge request/outcome; make stock plus idtarget the ordinary Umbriel route. Existing bespoke detection cannot remain the mandatory mutation gate. |
| `runner/mod.rs`, `output.rs` | Scheduling, live observation during commands, shutdown, bounded diagnostics | Add quiet/progress/recovery deadlines and feedback result records; retain source and I/O isolation. |

Preserve the bespoke implementation and its evidence; it remains useful as an
explicit conditional-control path, but selecting its presence automatically must
not select the old permanent-conflict user policy. The stock path is the delivery
target here, without a compositor port prerequisite. Maintain niri and feature
isolation while changing this path. Height-named external configuration already
exists: keep it for vertical behavior; add `minimum_width` with the same default
60 for horizontal secondary sizing, rather than silently redefining
`minimum_height` as a width. Internally the policy uses `minimum_extent`; the
existing active/other proportion pair supplies the default share on either axis.

Capability admission must verify that **qualified** sizing is understood. Use a
documented non-mutating parser/server capability check if available; otherwise
the known-stale ID form can verify the recognized qualified grammar by its
specific `unknown window` rejection. Unknown/unsupported replies do not permit
bare-action fallback. Keep that check separate from live targets and never use
a focused resize as a capability test.

## Implementation assignments and success tests

These are implementation slices, not additional research stages. A and B can
proceed together once the small shared DTO shapes are agreed; C joins their
outputs. D owns the running composition and native acceptance evidence.

### A — stock observation and qualified command boundary

**Own:** umbriel-rs protocol/state/tracking changes in that repository, then the
ZK desktop model/adapter projection. A native compositor change is not required
by the selected contract.

**Deliver:** typed current stock snapshots, layout-impact receipts, independently
fresh outputs, reported ordered groups, inferred axis/presentation eligibility,
and a typed by-ID signed edge request. No shell interpolation and no focus dance.
Use source parser tests to verify the qualified grammar; the current Rust
constructor's colon-form history is not enough evidence for a newly formatted
qualified string.

**Pass:** current native fixtures retain every layout field; older missing-field
fixtures stay observation-only; two workspaces with column zero do not collide;
rows reorder without identity loss; tab/flag/axis transitions update eligibility;
family skew waits and then resumes; output loss prevents writes; exact action
encoding handles representative IDs and rejects empty IDs; stale ID cannot fall
back to focus. Scoped upstream tests plus desktop tests and Clippy.

### B — pure feedback preferences and attempt lifecycle

**Own:** `layout/longcat` feedback module and its public input/decision types.
Reuse `zk-layout`; accept adapter-neutral membership, axis, content estimates,
quiet/progress facts and native outcomes. No socket/task dependency.

**Deliver:** selected guidance formulas, preference generations, topology
reconciliation, explicit neutral restore, yield/quiet timers, limited outcomes
and finite current-attempt evidence. A preview produces hypothetical output
without imaginary native handling.

**Pass:** the numerical examples above; 100 focus switches produce no neutral
drift; own progress and late commits never teach a share; user continuous motion
produces zero new writes after yielding; quiet resize changes the selected
preferences and survives focus return; sibling-only resize preserves active
share; manual smaller-than-default extent is respected; opening, closing,
reordering, cross-group move and tab/un-tab resume automatically; loss cannot
revive an old command; more than 64 focus changes remain usable.

### C — geometry compiler and corrective edge execution

**Depends on:** A's real geometry/command contract and B's goal/attempt types.
**Own:** stock geometry model in `desktop/umbriel`, pure donor-ready step
selection in the control domain, and `runner/native.rs` edge request mapping.

**Deliver:** fixed content budget/strip extraction, target-position estimation,
two-axis internal-edge translation, empirical scale, bounded progress and
blocked-boundary outcomes. One pending command and latest-state recomputation.

**Pass:** `[100,100,500] → [500,100,100]` and reverse use available donors;
three/four-member heterogeneous ratios converge; mixed title bars and nonzero
outer gaps remain stable; physical horizontal/vertical secondary axes choose
the correct action; positive and negative struts do not create repeated
oscillation; configured minima hold in unconstrained fixtures; native clamps,
maxima and a fixed middle member terminate limited rather than loop; partial
commits do not change B; slow clients do not teach preferences; no-change/dedup
paths reach a timer outcome. Use real native headless cases for the claims that
depend on native arrangement, not only a Rust copy of its equations.

### D — application integration and usable-service acceptance

**Depends on:** A–C. **Own:** runner composition/config/diagnostics and native
headless controller test; update maintained architecture/desktop/layout/runner
contracts to describe the behavior actually implemented.

**Deliver:** ordinary `zk longcat --backend umbriel` uses the working-line stock
surface; JSONL records contain preferences, Taffy goal, inferred versus reported
geometry, pending edge, achieved result, guidance reason and limitation. Preserve
independent backend features and the existing service-generation path. Report
the actual compositor identity/capability when invoking an installed service.

**Pass:** scripted stream/command peers verify receipt ordering, no stale tail
after topology/focus change, reconnect/reacquisition, pending command on loss,
quiet timers without duplicate events, output changes and bounded shutdown.
The real headless compositor verifies focus emphasis, blur, focus return after
user resize, drag/yield, member moves, title bars/edge gaps, stale IDs, unchanged
seat focus and unrelated column geometry. Long-running focus/membership churn
does not hit the old episode lifetime bounds. Run the existing feature matrix,
scoped nextest, Clippy and formatting checks after integration.

The end-to-end acceptance is **live subscription → Taffy goal → qualified resize
→ observed convergence → accepted user adjustment → continued focus behavior**.
A pixel-perfect unconstrained fixture is necessary; constrained fixture success
can be a truthful bounded limited result. Mere action replies, dry-run plans,
or another architecture document do not meet this acceptance.

## Connections and disposition

- [Earlier control research](umbriel-control-stream0.gpt6a.md) is implementation
  evidence for the strict native path. This design deliberately changes its
  exact-basis/restoration policy to ongoing visible guidance on the working line;
  it does not claim the old implementation was unnecessary or unavailable.
- [Remake architecture](../remake/architecture0.gpt6a.md#longcat-policy-and-taffy)
  supplies the sound separation of observed, policy and derived Taffy state.
  Its permanent suspension after unattributed geometry is the specific policy
  revised here.
- [Current layout contract](../../layout/README.md#leaving-shutdown-and-loss) and
  [runner retention contract](../../app/zk/src/runner/README.md#preview-effects-and-timers)
  identify the code behavior that must change, rather than suggesting a fresh
  application around an unrelated solver.
- [The IPC handoff](handoff0.glm53m.md#open-threads) explains why the old native
  deployment ticket is not the gate for this delivery. `zk-longcat-return` is
  the work anchor; `zk-longcat-user-layout-changes` covers the topology/guidance
  outcome. Update their pointers with implementation receipts when work lands.

No unresolved human choice blocks these assignments. The significant choices
made here are persistent per-window focus-share guidance, current-preference
restoration, inferred geometry with bounded corrective actuation, and automatic
resumption after normal topology edits. Parent review owns whether to select
them; ordinary implementation and native tests own tuning and disproving the
stated geometric assumptions.
