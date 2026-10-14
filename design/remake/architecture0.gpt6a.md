---
type: Design
title: Longcat remake — observed desktop, layout policy, compositor execution
description: A domain-grouped Rust workspace and staged niri/Umbriel design, separating existing streaming support from protocol constraints on reliable layout control.
resource: design/remake/architecture0.gpt6a.md
status: proposed
generated: { at: 2026-10-14, by: 'model:openai/gpt-6-astra#medium' }
sources:
  - resource: opencode://ses_ec3316eefffejmDBXbZ28spBd7
    title: Human remake commission, source investigation, abstraction decisions and landing design
  - resource: ../../src/commands/longcat.rs
    title: Existing proportional sizing, restore state, command transport and focus loop
  - resource: ../../src/niri/registry.rs
    title: Existing focus-only projection and blocking socket ownership
  - resource: ../../src/export/stream.rs
    title: Existing native niri lifecycle and layout event vocabulary
  - resource: ../../doc/discovery/taffy.md
    title: Earlier Taffy investigation and full-tree proposal, not implemented adoption
  - resource: ../umbriel/investigation0.glm53max.md
    title: Corrected predecessor research on focus races, restoration and layout identity
  - resource: file:///home/rektide/src/umbriel-rs/README.md
    title: Implemented library, subscription and layout-shape capabilities
  - resource: file:///home/rektide/src/umbriel-rs/crates/umbriel-client/README.md
    title: Source ownership, bounded observers, epochs and transmission evidence
  - resource: file:///home/rektide/src/umbriel-rs/crates/umbriel-client/examples/window_changes.rs
    title: Native subscription to typed snapshot differences and workspace impact
  - resource: file:///home/rektide/src/umbriel-rs/crates/umbriel-protocol/src/model/inventory.rs
    title: Observed geometry and focus semantics
  - resource: file:///home/rektide/src/umbriel-rs/crates/umbriel-protocol/src/action/extent.rs
    title: Current extent builders accept fractions, not window targets
  - resource: file:///home/rektide/archive/noctalia-dev/umbriel/src/server/ipc.cpp
    title: Native subscribe, initial families and broadcast deduplication
  - resource: file:///home/rektide/archive/noctalia-dev/umbriel/src/server/ipc_commands.cpp
    title: Native windows and workspaces payloads
  - resource: file:///home/rektide/archive/noctalia-dev/umbriel/src/server/server_events.cpp
    title: Event-loop idle coalescing of window and workspace notifications
  - resource: file:///home/rektide/archive/noctalia-dev/umbriel/src/workspace/workspace.cpp
    title: Focus-targeted height mutation
  - resource: file:///home/rektide/archive/DioxusLabs/taffy/examples/basic.rs
    title: Taffy computes a supplied tree rather than discovering compositor state
  - resource: kubeless0.gpt61sm.md
    title: Independent Sol assessment, dependency ownership, scheduler fairness and selected test results
  - resource: file:///home/rektide/src/kubeless/work/schedule/src/lib.rs
    title: Parent confirmation of greedy stream intake before readiness servicing
  - resource: file:///home/rektide/src/kubeless/view/impact/src/lib.rs
    title: Parent confirmation of former/current relation traversal semantics
  - resource: file:///home/rektide/src/umbriel-rs/crates/umbriel-state/Cargo.toml
    title: Parent confirmation of the existing memory dependency
  - resource: file:///home/rektide/src/umbriel-rs/crates/umbriel-client/Cargo.toml
    title: Parent confirmation that state and impact are example dev-dependencies
  - resource: file:///home/rektide/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/niri-ipc-26.4.0/src/socket.rs
    title: Blocking helper consumes the event socket without a public shutdown handle
verification:
  method: Source inspection; proposed interfaces and landing checks below are not implemented or tested.
  limits: No live compositor mutation or new stream capture in this session.
---

# Longcat remake

## Direction

Build **one Longcat policy over two compositor adapters**, with a pure layout
calculation boundary and explicitly owned asynchronous sessions. Keep reusable
Umbriel protocol/client/state work in `~/src/umbriel-rs`; Longcat remains a
consumer here. “Port to umbriel-rs” means using that library to control umbrield,
not putting application policy inside the protocol library.

The useful separation is **observation → policy/plan → execution**. A generic
socket/connection trait is unnecessary: both protocols already have clients,
and their command lifetimes differ. Separate observation and actuation roles
within each adapter, sharing one session identity. A layout solver is a third,
pure responsibility; it is not a third transport backend.

**Taffy is not currently used.** Neither Cargo manifest/lock nor Rust source
references it. The [discovery document](../../doc/discovery/taffy.md) proposes
integration; today's `apply_longcat_heights` performs its own arithmetic.

## Streaming is available; control fidelity is the constraint

Inspected umbriel-rs checkout HEAD `74cbff32fd164d18d08417abeed6863a9fdbfee3`
and compositor HEAD `7666af0a43560658e7b966befcbe47c101968e1c`. These are
checkout coordinates, not an assertion that the running daemon has either build.

| Need | Available today | Where a remaining constraint belongs |
|---|---|---|
| Live windows/focus/geometry | `Endpoint::subscribe([Windows, Workspaces], ...)`; independently driven socket, initial snapshots and subsequent frames | No periodic polling needed for these families |
| Add/remove/change and affected workspaces | `Inventory::install_windows_changes`; native example uses Kubeless impact over before/after | Application chooses which changes matter to layout |
| Every intermediate activity | No: server idle-coalesces and byte-deduplicates snapshots | Protocol semantics; polling cannot recover hidden transitions |
| Joint windows/workspaces revision | Families arrive independently | Protocol lacks atomic joint snapshot/revision; library must retain per-family evidence |
| Exact column identity, weights, axis | Not in current wire payload | Compositor protocol; inferred geometry cannot restore missing state |
| Resize a chosen tiled window | Niri has `SetWindowHeight { id, ... }`; current Umbriel extent actions use focused window | Compositor action grammar/implementation, then Rust bindings |
| Know action is visible | Reply proves native handling, not committed geometry convergence | Controller observes later state; target geometry would improve protocol evidence |
| Distinguish manual and self resize | Geometry alone cannot establish attribution | Track expected effects locally; reliable origin/revision evidence needs protocol changes |
| Loss/reconnect | Rust source exposes epoch/gap and observer lag; reconnect is opt-in | Longcat invalidates old control assumptions on any discontinuity |

Evidence: `Ipc::handleRequest` emits initial family snapshots; `broadcastEvent`
deduplicates serialized payloads; `scheduleIpcWindowsEvent` coalesces via an idle
callback. `Window` explicitly models target x/y and committed w/h. The Rust
`window_changes` example already composes subscription, decoding, inventory,
differences and relation impact. A typed convenience feed can improve ergonomics,
but is not a missing transport capability.

“Activity stream” here means observed desktop changes, not raw keyboard/pointer
input or an audit log. Completion deadlines are timers, not polling. An explicit
fresh acquisition after loss is recovery, not a periodic polling architecture.

## Domain-grouped workspace

Recommended destination (crate names provisional; directories are domains):

```text
Cargo.toml                       # workspace and shared dependency versions
desktop/
  model/                        # zk-desktop: IDs, observations, capabilities, plans
  niri/                         # zk-niri: protocol reducer and command executor
  umbriel/                      # zk-umbriel: umbriel-rs composition and mapping
layout/
  longcat/                      # zk-longcat: baseline/intent/effect state machine
  solver/                       # zk-layout: allocation, rounding, Taffy integration
terminal/
  kitty/                        # zk-kitty: existing remote control and PID resolution
  zoomer/                       # zk-zoomer: focus-to-font policy
inspection/
  export/                       # zk-export: native exports and projected diagnostics
app/
  zk/                           # zooming-kittens package, zk binary/config/commands
design/                         # inquiry, including this proposal
doc/                            # maintained user contracts
```

Each leaf above is a crate root, not another flat `crates/` bucket. Domain
READMEs can describe relationships among their children. Do not create empty
placeholder crates: introduce these as responsibilities are extracted.

Dependency direction: adapters depend on `zk-desktop` and their native client;
`zk-layout` and `zk-longcat` consume backend-neutral inputs; `zk-longcat` uses
the solver; the application composes adapters and policies. Kitty operations
need no compositor crate. Zoomer consumes the desktop focus projection, then
uses kitty. Native export can retain native protocol dependencies without
forcing the Longcat model to contain every niri event or every Umbriel field.
No Polars requirement for the daemon or policy tests.

Another plausible grouping is `compositor/{model,niri,umbriel}` plus
`automation/{longcat,zoomer}`, `layout/solver`, `terminal/kitty`,
`inspection/export`, `app/zk`. Prefer the first: desktop facts have consumers
beyond automation, while Longcat's policy is inherently layout-specific.

## Contracts worth introducing

### Desktop observations

Use a small native-independent semantic model, not a union of both whole APIs:

- `SourceId`, `Epoch`, local `ObservationVersion`; `WindowKey` and `WorkspaceKey`
  include source/epoch and an opaque native ID. Niri numeric IDs must not force
  Umbriel string IDs into integers. Local versions are not server revisions.
- `DesktopView` retains windows, workspace facts, active window, family readiness
  and freshness. Unknown/not-yet-seen differs from a known-empty census.
- `GroupView` contains ordered members, secondary axis, geometry and membership
  evidence (`Reported`, `Inferred`, `Unavailable`). Niri column indices are
  current positions, not persistent group IDs; Umbriel shared x is not an ID.
- A control episode binds to source/epoch, workspace, member identities and
  topology evidence. Reordering/merge/split is evaluated explicitly; a matching
  column index never silently transfers a baseline to different windows.
- Distinguish geometry as observed committed content, native target tile, and
  desired output. Keep inferred quantities tagged. Unsupported layouts, floating
  windows, scratchpads and uncertain membership remain representable.

Niri's adapter reduces its initial `WindowsChanged` and subsequent
`WindowOpenedOrChanged`, `WindowClosed`, `WindowFocusChanged`,
`WindowLayoutsChanged`, and workspace events into current state. The current
registry mostly handles focus timestamps and initial focus, and loses blur on
`id: None`; wrapping it unchanged is insufficient. The existing export mapping
shows the larger event vocabulary already available in the dependency.

Umbriel's adapter reduces complete family replacements. Derive seat focus from
`active`, not workspace-local `focused`. Do not fabricate an atomic joined
desktop when the two families happen to have adjacent local positions.

### Plans and execution

Sketch of roles, not committed Rust signatures:

```text
observe() -> { source owner, observation receiver, actuator }
reduce(observation) -> DesktopView + impact + continuity
decide(view, policy_state, clock_input) -> Decision
Decision = Noop | Unsupported(reason) | Suspend(reason) | Plan
prepare(plan, current_view, capabilities) -> PreparedPlan | Rejected
execute(prepared_plan) -> per-action transmission/native-reply evidence
observe subsequent state -> Converged | Conflicting | TimedOut | Lost
```

The source owner lives until explicit shutdown. A dedicated native I/O task
keeps draining while commands and policy work wait. Use bounded delivery with
explicit loss; do not repeat today's blocking `Socket::read_events()` inside
`tokio::spawn` or the unbounded focus channel. Niri's synchronous API can live
on an owned blocking worker with a defined stop mechanism, or a native async
adapter. Inspection of `niri-ipc` 26.4.0's `socket.rs` favors the async adapter:
the helper hides its stream, and `read_events(self)` consumes it into a blocking
closure without a public shutdown handle. Merely aborting a blocking task will
not interrupt its read. Reuse niri's request/event types with an owned Tokio Unix
stream, bounded line framing and explicit shutdown; an upstream socket-access
API would be an alternative if keeping the blocking helper is important.

One controller owns Longcat state; message passing replaces its DashMap and
many RwLocks. Pending effects are state, so awaiting a command must not prevent
observation processing. Keep at most one mutation sequence in flight initially;
discard obsolete unsent plans. Do not replay possibly-sent actions after loss.

A plan names source/epoch, observed basis, member identities, intended focus,
requested allocations and restore conditions. Pre-send local validation reduces
staleness but **cannot** close a server-side focus/topology race. Report this
honestly in capabilities. Execution returns per-command results, including
`Unsent`, native rejection, handled, and possibly-sent/unknown outcomes; a
multi-window niri sequence can partially succeed. Never call it a transaction.

Use a single concrete backend enum at the application composition point if
convenient. Small observation/actuation interfaces support scripted peers and
dry-run; no plugin ABI, universal connection pool, or generic reconnect framework
is needed. Keep policy/solver pure functions until multiple implementations
justify a solver trait.

## Longcat policy and Taffy

Maintain three separate values: **baseline**, **requested effect**, **observed
state**. Today's detector compares current heights with baseline and can mistake
its own expansion for user input. A debounce does not repair that distinction.

Proposed episode states: `Acquiring → Ready → Applying → Settled`, with
`Restoring`, `Conflicted`, and `Lost` exits. A topology change invalidates pending
plans for both the former and new group. A focus change inside a group recomputes
from its baseline rather than compounded enlarged geometry. Leaving the group
restores only while the episode still has sufficient identity and effect evidence.
Loss invalidates restoration authority; keep diagnostics but reacquire before
starting another episode. Shutdown is a bounded conditional restore, not an
unconditional write of historical pixels.

An unexpected geometry change is **unattributed**, not proven manual. Compare
against the requested plan and native constraints; intermediate commits are not
immediate conflicts. After a convergence deadline, suspend the episode and report
the mismatch. Explicit re-baselining is a policy operation; do not silently rewrite
global proportions. Native clamping and manual edits can remain indistinguishable.

Taffy belongs in `layout/solver` as a calculation engine. The observed desktop
and episode ledger remain authoritative; Taffy nodes are derived solver state.
This deliberately revises the earlier idea of making a `TaffyTree` the entire
desktop/state/status owner. A layout engine neither discovers topology nor proves
that its calculated rectangles were applied by a compositor.

Start with a per-affected-group tree; cache later if justified. Inputs include
available secondary-axis content span, per-member baseline weights, explicit
min/max constraints and known gap treatment. Do not count compositor gaps twice
or change compositor gap configuration merely to match a solver.

For baseline `[400,300,200]`, no gaps/bounds, active first and share `0.7`,
desired heights are `[630,162,108]`. A flex formulation needs zero bases and
grow shares `[0.7,0.18,0.12]` with deliberate shrink/min handling; grow ratios
on top of nonzero original bases do not mean the same thing. Establish fixture
contracts before choosing exact Taffy styles. Infeasible minima produce an
explicit infeasible outcome, not negative sizes or unexplained overflow.
Round deterministically while conserving the feasible integer allocation.

The solver emits desired allocations; backend compilation decides how to express
them. Niri can request fixed heights but still has native constraints and
sequential effects. Umbriel has proportional native layout, so a compositor-native
share strategy may express focus emphasis better than translating every computed
pixel back into an approximate fraction. Declare strategy/capability differences;
do not promise pixel-identical output across both compositors.

### Current defects that should become replay cases

These are source-level findings, not reproduced live failures:

- `handle_focus_lost` tests a focused window's column index without also testing
  its workspace. Equal positional indices on different workspaces can look like
  staying in the same column. Replay a cross-workspace focus transition with
  both columns numbered zero.
- The registry clears its remembered focus on `WindowFocusChanged { id: None }`
  without emitting blur. Replay window-to-layer/empty focus and require an
  explicit active-window transition to none.
- `_handle_window_layout_changed` is not wired into the Longcat loop, which
  ignores lifecycle/layout events. Replay a moved/closed member while an apply
  is pending; neither the baseline nor the pending plan may follow an old index.
- Original height totals include retained entries, while current windows are
  fetched again for applying. Replay deletion between baseline and allocation;
  removed members must not donate fictional space to the remaining group.
- The current minimum-height pass does one redistribution, which can leave a
  previously unconstrained member below minimum or exhaust a group's budget.
  Test cascading bounds and infeasible sums, not only the happy-path 70/30 split.

These justify replacing the control state model rather than merely moving the
current `Longcat` struct behind a backend trait.

## Improve the owners as we go

### Kubeless: selected mechanisms, with native state ownership retained

The follow-up [adoption and upstream agenda](kubeless-asks0.gpt6a.md) sharpens
these choices: use memory for niri (reuse Umbriel's existing owner), direct
impact and direct scheduling in the first implementation. Improve stream
scheduler fairness alongside the consumer. Add independent derived-status
publication when it gains consumers; fit-test settlement against retained
episode ownership before adopting its future-based helper. The agenda records
the human invitation to improve Kubeless while it is still young and connects
those changes to the Taffy-driven control path.

The [independent assessment](kubeless0.gpt61sm.md) agrees with the three-way
boundary and adds concrete dependency and timing choices. Parent inspection
confirmed the impact API, manifests, and the stream Scheduler's intake loop.

| Mechanism | Selected placement |
|---|---|
| `kubeless-view-memory` / change receipts | Already beneath `umbriel-state`; optionally direct in `desktop/niri` for its reducer. No second Umbriel inventory. |
| `kubeless-view-impact` | Direct dependency for runner impact projection: both former and current groups/workspaces on move/delete. Umbriel's example uses it only as a dev-dependency. |
| `kubeless-work-schedule` | Direct dependency in an app-owned runner module for keyed reconsideration and real timer-driven batching. Use direct `Schedule` with bounded intake. |
| Executor / ReadyWork | Later option if keyed concurrent effects become worthwhile; initial serialization already provides the needed exclusion. |
| Session / reflection / publication / retained-action host | Not baseline dependencies. Native subscriptions already own delivery and loss; introduce these only for a concrete additional consumer contract. |

Keep the runner as an `app/zk` module initially; extract `control/runner` if
shared runtime consumers justify it. Scheduling state is neither layout policy
nor socket state. Merge dirty reasons, then calculate from the latest capture
when work is admitted; don't queue obsolete resize plans for replay.

Filter native receipts by layout-relevant changes: full replacements can include
equal rows, and equal membership can still hide important geometry/focus changes.
Preserve before/after captures when deriving group impact; a moved window's old
group cannot be recovered from current state alone. Group identity remains our
adapter's responsibility, not a fact supplied by `affected_roots`.

Focus response remains immediate initially, matching present intent. Resize
quiet batching is separate; a maximum batch delay is not proof that a drag ended
and must not authorize baseline learning. An actual timer must fire even if no
further window event arrives. Clock-injected policy tests cover that case.

**Upstream opportunity:** bound `Scheduler::poll_ready` stream-intake turns so a
permanently-ready input cannot starve due work. Its current loop drains until
Pending/EOF before polling the schedule. Direct caller-budgeted `Schedule`
already avoids this, so no Kubeless upstream change blocks the first port.
Also improve entrance documentation for existing independent lagging publication
and add a non-resumable desktop composition example, as Sol recommends.

Sol ran selected existing Kubeless tests: **159 passed, one skipped** across two
nextest runs. These support mechanism reuse, not the unbuilt Longcat integration.
The assessment records commands and the inspected sibling revision; use explicit
development paths or pinned reviewed revisions rather than assuming unpublished
`0.1.0` packages are interchangeable.

### umbriel-rs: useful library improvements

1. Lift the example's typed observation/install composition into a reusable,
   optional state-feed API if the Longcat adapter proves duplication. Preserve
   epoch/gap/lag/invalid-snapshot semantics and source ownership; keep raw client
   delivery independently usable.
2. Expose layout-shape inference as a small reusable module or leaf if needed by
   both capture and Longcat diagnostics. It currently lives in desktop-dump tool
   composition. Carry its inferred/unresolved labels; it is not exact columns.
3. Add typed models/builders for newly agreed compositor capabilities. Keep
   Longcat proportions, baseline retention and conflict policy here in zk.

### umbrield: the actual control-contract work

The current Rust extent builder accepts only `ExtentFraction`, matching the
server's focused-window `Workspace::setFocusedHeight`. A library-only overload
cannot implement reliable by-ID sizing. Propose a structured IPC operation with
window ID, secondary extent and optional expected layout revision; implement
selection and validation together on the server event loop. Do not synthesize
focus → resize → focus-back.

Also propose native layout inspection: group identity/lifetime, ordered members,
axis, row/edge weights or exact restore representation, target boxes, and a
layout revision. Publish changes to that state, not just a one-shot query.
Capability discovery should identify support; do not infer it from the Rust
crate version. For exact reversible episodes, consider compare-and-set group
weights or a compositor-owned restore token: merely exposing a fraction and
setting rows sequentially does not guarantee arbitrary group restoration.

These are protocol proposals, not existing APIs. A first by-ID operation unlocks
race-free target selection; exact restoration and atomic preconditions are
separate improvements. Existing geometry permits read-only planning now, but
does not justify claiming production-equivalent Umbriel restoration today.

## Landing sequence and assessment

| Stage | Coherent delivered capability | Evidence needed at that boundary |
|---|---|---|
| 1. Workspace extraction | Keep `zk` executable behavior; move app and existing native domains into buildable crates | Workspace builds; existing export/config/font tests and CLI smoke checks; update install/run paths |
| 2. Desktop observation | Both adapters publish current semantic views, focus and explicit loss; read-only Umbriel consumer | Scripted initial state, no-focus, move, closure, malformed census, lag and reconnect; native read-only capture |
| 3. Pure policy and solver | Replayable baseline/intent/effect machine, Taffy-backed allocation, dry-run plans | Allocation feasibility/rounding fixtures; self-effect vs conflict; move invalidates old/new groups; no writes in dry-run |
| 4. Niri execution | Existing Longcat capability through new contracts; bounded pending effects and conditional restore | Partial command failure, stale plan, loss during apply, shutdown and live scratch-column convergence |
| 5. Umbriel control rendezvous | Server contract + Rust bindings + adapter execution for declared capabilities | Focus changes between observation/send target the intended ID; restore fidelity under hints, moves, clamp and external edits |
| 6. Consumer consolidation | Zoomer uses common focus; native export and semantic status are distinct views | Kitty blur targets original terminal; old export formats explicitly preserved or migrated; bounded slow-output behavior |

Stages 2 and protocol design can proceed independently after initial extraction;
stage 3 can run from fixtures before native mutation is possible. Stage 5 lands
only against a named compatible compositor/library revision pair. Each prefix
must compile and tell the truth about available capabilities. Avoid a giant
foundation merge that postpones the first real consumer until the end.

Retain `font`, `conf-size`, systemd generation and kitty process/connection
behavior during extraction. Existing native JSON/JSONL export and optional Facet
scaffolding move with their owner; they do not become neutral merely by moving.
Config has app loading plus domain policy values: preserve precedence and names
until an explicit migration. Presets, OSD, Vicinae, D-Bus and further terminal
backends have homes in this architecture but are separate implementation work.

## Connections and next decisions

- [Remake reading map](README.md) groups this proposal and the independent
  [Kubeless assessment](kubeless0.gpt61sm.md), whose selected mechanisms are
  integrated above. The assessment preserves the wider alternatives and test evidence.
- [Earlier Umbriel investigation](../umbriel/investigation0.glm53max.md) supplies
  restoration algebra and focus-race reasoning. Its “no Rust library” and
  “build a thin client next” statements describe its earlier date; umbriel-rs now
  supplies a much richer client and state implementation.
- [Taffy discovery](../../doc/discovery/taffy.md) motivates constraint solving.
  This design retains that goal but separates solver state from desktop truth.
- Existing `zk-longcat-dry-run`, `zk-longcat-user-layout-changes`,
  `zk-multi-col-height`, and `zk-status-json` identify related demand. This
  proposal does not mark their implementation complete.

The principal decision is whether to pursue the umbrield control extensions
alongside the read-only adapter. Recommendation: **yes**. Start the observation
and dry-run path now while designing exact targeting/restoration with the native
owner; otherwise the “port” will inherit irreducible races behind a nicer trait.
