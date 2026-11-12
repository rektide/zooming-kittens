---
type: ImplementationContract
title: Desktop observation and native control boundaries
description: Bounded native observation, stock Umbriel grouping and qualified relative edges, plus preserved conditional whole-group allocation.
resource: /desktop/README.md
status: implemented-baseline
updated: { at: 2026-11-12, by: 'model:openai/gpt-6.1-sol#high' }
generated: { at: 2026-10-15, by: 'model:openai/gpt-6.1-sol#xhigh' }
sources:
  - resource: opencode://ses_e32c4da49ffekNe2490gCdHuLp
    title: Compose ordinary stock source, epoch-bound qualified execution, timed refresh and native-owner recovery in the production runner
  - resource: opencode://ses_e331fea99ffeEcIAeJ9F8pM2aK
    title: Implement stock Tracker projection, independently fresh outputs, inferred axes and epoch-bound qualified internal-edge transmission
  - resource: /design/longcat/feedback0.gpt6a.md
    title: Selected stock feedback contract and implementation assignments
  - resource: /desktop/umbriel/src/edge.rs
    title: Separate grammar verification, local preparation, one owned command and transmission outcomes
  - resource: opencode://ses_e9a70297dffe6dQ5UJP0up8m7T
    title: Integrate coherent Umbriel layout-state, reusable Tracker composition and conditional native allocation
  - resource: /desktop/umbriel/src/layout.rs
    title: Native basis normalization, content constraints, group request compilation and receipt verification
  - resource: opencode://ses_ec314c407ffeXx45VDjb05vmol
    title: Desktop baseline implementation, API contract and scoped verification
  - resource: /design/remake/architecture0.gpt6a.md
    title: Approved desktop, policy and execution separation
  - resource: /design/remake/kubeless-asks0.gpt6a.md
    title: Selected native memory ownership and before/after impact
  - resource: /design/remake/kubeless0.gpt61sm.md
    title: Native inventory composition and explicit continuity limits
  - resource: /desktop/model/src/lib.rs
    title: Scoped captures, semantic impact and bounded observation API
  - resource: /desktop/niri/src/reducer.rs
    title: Native niri event batches over Kubeless view-memory
  - resource: /desktop/niri/src/source.rs
    title: Owned asynchronous source, hard frame bounds and by-ID height requests
  - resource: /desktop/umbriel/src/reducer.rs
    title: Native Umbriel Inventory composition and typed family replacement
  - resource: file:///home/rektide/src/umbriel-rs/crates/umbriel-state/src/inventory.rs
    title: Existing inventory epochs, family health and actual window change receipts
  - resource: file:///home/rektide/src/umbriel-rs/crates/umbriel-client/src/driver.rs
    title: Existing independent native receive ownership and bounded observers
verification:
  method: Scoped nextest, strict all-target clippy and package formatting checks.
  result: Historical conditional integration passed native restoration checks; A/C verified stock boundary/geometry natively. D passes the expanded 200-test desktop/layout/application scope, strict Clippy and owned formatting with ordinary stock composition.
  limits: No mutation or restart of the running desktop. D's integrated native user-guidance acceptance is still open; the lower stock boundary/geometry and preserved conditional checks are separate evidence.
---

# Desktop observation and native control boundaries

Stock observation, pure policy, frozen geometry and ordinary application
composition are implemented across A–D. Native boundary/geometry checks and
scripted production-runner checks are distinct from still-open integrated native
user-guidance acceptance. [D receipt](../design/longcat/implementation-d0.gpt61s.md)
links the checkpoints and exact verification limits.

## Integration contract

- `zk_desktop::DesktopCapture`: source/epoch `scope`, local `version`, `windows`,
  `workspaces`, `groups`, `active_window`, per-family `windows_status` and
  `workspaces_status`, optional `outputs_status`, scoped `outputs`, `connected`,
  and declared `capabilities`. An optional
  `control_status` selects coherent control readiness; `allocations` carries
  conditional native allocation evidence per group.
- `WindowKey` / `WorkspaceKey` contain `Scope` plus `NativeId::{Number, Text}`.
  Group keys are positional hints, **not persistent group incarnations**. Ordered
  membership and `Evidence::{Reported, Inferred, Unavailable}` remain explicit.
  `GroupView.secondary_axis` is optional; unknown geometry never becomes vertical
  by default. `edge_eligibility` separates local topology/presentation readiness
  from qualified server-grammar verification.
- Both adapters return `(Source, zk_desktop::Observer)` from async `subscribe`.
  The observer provides `current()` and bounded `recv()` of shared
  `DesktopUpdate { before, after, impact, continuity }`. Lag is an explicit error;
  acquire `current()` and invalidate controller episodes before reconsidering.
- Keep the source owner alive. `Source::shutdown(self).await` joins actual cleanup;
  dropping it requests shutdown. Observation never waits for policy/output work.
- Niri: `subscribe(path, source_id, Options)` and `subscribe_env`;
  public `Reducer` supports pure native-event replay. The actuator is obtained
  separately from `Source::actuator()`, accepts by-ID `HeightRequest`, and reports
  `Unsent`, `Rejected`, `Handled`, or `PossiblySent`, never transaction/settlement.
- Umbriel stock: `subscribe(endpoint, source_id, Options)` observes windows,
  workspaces and outputs through `umbriel_tracking::Tracker`. It does not probe or
  mutate automatically. Missing older layout fields remain observation-only.
  After current acquisition, `source.edge_actuator().await` separately verifies
  qualified height-down/width-right grammar using an impossible stale ID and its
  specific unknown-window rejection. No focused sizing is a capability test.
- Umbriel bespoke control: `subscribe_layout` uses `umbriel_tracking::Tracker` over the
  single `layout-state` family. Membership, active identity, native basis and
  committed geometry all come from that coherent family. Unrelated windows or
  workspaces events cannot alter control focus/readiness. `layout::execute`
  submits one conditional whole-column request and verifies its accepted receipt.
   It cannot create a stock edge actuator. The application selects it only with
   explicit `--umbriel-control conditional`; extension presence cannot choose it.

Sibling development dependencies are explicit relative paths: from each desktop
crate, `../../../umbriel-rs` and `../../../kubeless` resolve under `~/src`.
The paired native extension and Rust bindings live upstream in their own worktrees.
The app owns policy input conversion and scheduling.

## Source ownership and loss

```rust,ignore
let (source, mut observations) = zk_niri::subscribe_env(
    SourceId::new(Backend::Niri, "niri-instance-1"),
    zk_niri::Options::default(),
).await?;
let actuator = source.actuator(); // does not prolong source ownership
// Consume observations while policy/command futures run independently.
// Observations also offer nonblocking try_recv() and a current() capture.
source.shutdown().await?;
```

`Source::shutdown(self)` consumes the owner, signals cancellation, and joins the
receive task; the result reports a task panic as `tokio::task::JoinError`. Niri
selects cancellation against its asynchronous read even when the compositor is
idle. Umbriel selects against its native observer and then awaits the existing
native owner's shutdown. Both publish a disconnected capture before completion.
Dropping a source requests the same cleanup without waiting. Keep a live Tokio
runtime for that cleanup; explicit shutdown is the join/verification boundary.
An observer or niri actuator alone does not keep a source alive.

Delivery is bounded in **items**, with shared immutable captures; it is not a
historical audit log or an aggregate byte-budget promise. Niri bounds line bytes
before extending its buffer, inventory window/workspace counts, and receive-turn
work. Umbriel retains the client's existing frame, observer-item and observer-byte
limits. An optional semantic consumer cannot block native acquisition. On
`ObserveError::Lagged`, invalidate outstanding episode authority and reacquire
`current()`; do not interpret skipped updates as a continuous transition. On
`Closed`, the final current capture is diagnostic, not fresh control authority.
The niri actuator also refuses a closed publisher, even if a panic prevented the
normal loss capture.

Niri does not reconnect automatically. Reusing a source name for a replacement
subscription requires a strictly higher `Options::initial_epoch`; alternatively
use a unique source-instance name. Umbriel's native reconnect option retains its
epoch announcements; a separately created native subscription restarts at epoch
one and therefore needs a distinct instance name. Retained rows keep their **old
epoch keys** until their own family is replaced. Family stamps are local receive
positions, not a resumable cursor or joint compositor revision.

Niri rejects malformed/inconsistent census or delta input atomically, retains the
old diagnostic rows, marks the source lost and requires a fresh epoch. Umbriel
reuses native validation: a bad complete typed family leaves that family's rows
stale and a later valid complete replacement can reacquire it. Native transport
loss or native observer lag requires a new native epoch; the adapter never
silently re-registers after lag. Independent family readiness travels with every
capture; stock `is_ready()` requires all three families current in the same epoch,
but does not claim source-atomic acquisition. Missing or temporarily inconsistent
workspace/output associations additionally prevent group edge admission until
a later full replacement makes them usable. A bespoke control capture instead
uses only its `control_status`, with no
cross-family join. Invalid control state stops that source; no silent reacquisition.

## Geometry, groups and execution

Niri's reported 1-based scrolling `(column, row)` orders group members. The key's
column number is only a position; compare **ordered member identities** before
carrying a baseline forward. Duplicate row positions or an unlocated tiled sibling
on the same workspace make membership unavailable.
Floating windows never enter these groups. A group's bounding rectangle is a
derived union of observed tile rectangles, potentially including gaps. It is not
a content-height allocation budget. Preserve `geometry.content_size`, `tile_size`,
`tile_position`, and `window_offset_in_tile` as distinct quantities.

Stock Umbriel reports committed content sizes and target content positions, not
target sizes or exact tile rectangles. Column/row membership comes from one full
windows census and is scoped by workspace and source epoch. Rows are ordered by
reported native row, not lexical ID or shared x coordinates. Missing/duplicate
locations withhold complete membership; reported `(-1,-1)` nonmembers do not
silently disqualify supported survivors. Tab, float, pin, minimize, fullscreen and
maximize presentations remain visible but cannot receive these group writes.
Unknown presentation flags also cannot authorize sizing.

Physical axis is inferred only when ordered content rectangles are separated on
one axis and overlap on the other. Ambiguity waits. This is candidate evidence,
not quietness or an allocation basis: target positions and committed dimensions
can describe different stages of a resize. The geometry compiler freezes quiet
constants rather than relearning them from mixed-time frames.
Workspace-local `focused` does not become seat focus. Native Inventory retains
source order; semantic maps iterate by key. Metadata-only window snapshots and
unrelated output edits do not create affected group geometry. Relevant output
changes affect only groups on that output.

`RelativeEdgeRequest { group, target, following, basis, axis, fraction }` is an
adapter-neutral signed cross-end motion, not a pixel assignment or a Taffy share.
`edge::prepare` checks the exact latest local capture, current families, source
scope, group eligibility and actual adjacent members. It refuses outer/last edges,
empty IDs, zero/nonfinite/out-of-range deltas and mismatched axes. Vertical uses
`window-modify-height-down`; horizontal uses `window-modify-width-right`.
The CLI equivalent always has `--window`; IPC uses structured `cmd: action`,
an action name, numeric `delta` and a nonempty `window` field. It does not send
the historical `cmd: msg` text form. Positive and negative travel use the same
target/end edge.

`geometry::FrozenGeometry` implements that compiler: quiet B and measured strips,
separate target estimates/committed extents, both physical axes, cumulative
late-commit corridors and empirical scale. `compile`/`begin` precede native I/O;
`observe` assesses only compatible own progress. `prepare_frozen`/`execute_frozen`
can replace temporary candidate-axis ambiguity only with the private model token.
They preserve all other current scope/context/adjacency checks. Incompatible
geometry revokes the model; the application classifies guidance versus limitation.

`Source::windows_refresh` supplies a no-queue timed stock handle. The existing
subscription owner installs its normalized windows reply at its actual receive
position through Tracker before returning success. It is not an atomic family
snapshot or a cross-socket query splice. Geometry requests one refresh after 32ms
without position progress, then one bounded blockage outcome. The application
waits for the actual refresh future before that second timer. `Reconnect` is
reexported with `SubscriptionOptions`; ordinary stock application startup explicitly
enables the native owner's bounded transport retry policy, while library defaults
remain no-reconnect. Every new epoch still needs new families and qualified probes.

`EdgeActuator` is verified for one epoch, shares one no-queue command permit across
clones, and rechecks local freshness after connection before sending any bytes.
Neither a handle nor an observer prolongs source ownership. A reconnect requires
new grammar verification; an old handle cannot send into the new epoch. Stock
keeps `height_by_id`, atomic allocation, exact restore and native revision flags
false. `relative_edge_by_id` describes the adapter's eligible relative operation,
not server negotiation: only the verified actuator can admit its transmission.
Its `Handled` means native handling only. Rejection never triggers a bare fallback;
loss/uncertain replies remain `PossiblySent`, not permission to replay.

The native extension reports exact scrolling membership and distinct target
tile/content and committed geometry. `ConditionalAllocation` normalizes content
budget, member content bounds, decoration heights, baseline target extents and
rounded edge gaps. Native tile bounds include title bars; vertical content bounds
subtract them. The content solver budget is native `row_budget` minus decorations,
not a sum reconstructed from a transient committed snapshot.

Opaque `basis`, weightless `context`, and `restore_payload` bytes are created and
interpreted only by the Umbriel adapter. The model has no protocol dependency.
The runner compares the complete basis against its last accepted authority;
same-pixel native weight edits are still changes. `GroupAllocationRequest` carries
the expected basis and either an emphasis pixel vector or the saved original raw
weights. Accepted native target contents must match the requested vector. Source
loss, malformed receipts or possibly-sent commands never authorize replay.

`HeightActuator::set_height(HeightRequest { target, basis, height })` checks current
family readiness, exact local basis, source/epoch, target existence and tiled
placement before connect and immediately before sending. It requests niri's
native fixed logical-pixel height by explicit ID, with one in-flight command and
no waiting queue/retry. It does not close a compositor-side race or provide an
atomic multiwindow change. Retain each returned `Transmission`: `Handled` proves
native handling only; `PossiblySent` forbids blind retry. **Do not cancel an
admitted command future** and assume no effect: its lasting receipt/settlement
ledger belongs to the app runner. Continue processing observations while waiting.
Umbriel's group operation supplies a server-side current-state predicate, not a
monotonic revision: ABA remains possible. It never performs focus→resize→focus-back.

## Verification and limitations

Scoped commands (no live compositor mutation):

```sh
cargo nextest run --locked -p zk-desktop -p zk-niri -p zk-umbriel
cargo clippy --locked -p zk-desktop -p zk-niri -p zk-umbriel --all-targets -- -D warnings
cargo fmt --check -p zk-desktop -p zk-niri -p zk-umbriel
```

The fixtures cover unknown versus known-empty state, no focus, opening/closing,
cross-workspace same-column collisions, atomic multirow layout updates,
renumbering, duplicate/malformed snapshots, epoch/loss, native observer byte
loss, semantic lag, scripted socket shutdown, by-ID replies and uncertain
transmission. A waiting command does not pause focus observation. This is a
desktop/native boundary; the [runner contract](/app/zk/src/runner/README.md) describes
integrated Taffy control, native-weight restoration and the headless check recipe.
The actual deployed older compositor still requires the separate native carry and
session-transition work before the historical conditional writes can run on the
user's desktop. The stock boundary has native parser/structured-request oracles,
the existing by-ID headless check, and an opt-in real Rust stock-boundary check:
`bash desktop/umbriel/tests/native/check.sh`. The latter boots an isolated
compositor, observes its actual stock families, verifies both qualified actions,
and checks one edge response without changing unrelated focus/geometry.
Integrated feedback-loop acceptance is still pending B–D.

## Connections

- [Approved remake architecture](/design/remake/architecture0.gpt6a.md#desktop-observations)
  owns the three-way observation/policy/execution boundary.
- [Kubeless adoption](/design/remake/kubeless-asks0.gpt6a.md#adopt-in-the-first-implementation)
  explains why niri uses memory and both adapters project actual before/after
  receipts. Upstream scheduler fairness remains deferred; no upstream edits.
- [Independent assessment](/design/remake/kubeless0.gpt61sm.md#observation-reuse-the-existing-umbriel-owner)
  grounds the choice not to build a second Umbriel inventory.
