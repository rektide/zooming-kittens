---
type: ImplementationContract
title: Desktop observation and conditional allocation
description: Bounded native observation, coherent Umbriel layout authority and conditional whole-group allocation.
resource: /desktop/README.md
status: implemented-baseline
generated: { at: 2026-10-15, by: 'model:openai/gpt-6.1-sol#xhigh' }
sources:
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
  result: Initial desktop baseline had 25 tests; integrated consumer scope now passes 107 tests and strict Clippy, plus isolated native headless sizing/restoration.
  limits: No mutation or restart of the running desktop; headless verification uses the separately built native extension.
---

# Desktop observation and conditional allocation

## Integration contract

- `zk_desktop::DesktopCapture`: source/epoch `scope`, local `version`, `windows`,
  `workspaces`, `groups`, `active_window`, per-family `windows_status` and
  `workspaces_status`, `connected`, and declared `capabilities`. An optional
  `control_status` selects coherent control readiness; `allocations` carries
  conditional native allocation evidence per group.
- `WindowKey` / `WorkspaceKey` contain `Scope` plus `NativeId::{Number, Text}`.
  Group keys are positional hints, **not persistent group incarnations**. Ordered
  membership and `Evidence::{Reported, Inferred, Unavailable}` remain explicit.
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
- Umbriel legacy: `subscribe(endpoint, source_id, Options)` remains read-only
  windows/workspaces observation with unavailable grouping.
- Umbriel control: `subscribe_layout` uses `umbriel_tracking::Tracker` over the
  single `layout-state` family. Membership, active identity, native basis and
  committed geometry all come from that coherent family. Unrelated windows or
  workspaces events cannot alter control focus/readiness. `layout::execute`
  submits one conditional whole-column request and verifies its accepted receipt.

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
capture; legacy `is_ready()` does not claim source-atomic windows/workspaces
acquisition. A control capture instead uses only its `control_status`, with no
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

Legacy Umbriel reports committed content sizes and target positions, not target
tile sizes. Its read-only baseline leaves `groups` empty and workspace `grouping`
unavailable, rather than inventing exact columns from shared x coordinates. It
still reports windows, workspaces, seat-global activation, scratchpads and changes
for read-only diagnostics and future protocol work. Workspace-local `focused`
does not become seat focus. Native source order is retained by Umbriel's owner;
semantic map iteration is key order, not compositor order.

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
session-transition work before these writes can run on the user's desktop.

## Connections

- [Approved remake architecture](/design/remake/architecture0.gpt6a.md#desktop-observations)
  owns the three-way observation/policy/execution boundary.
- [Kubeless adoption](/design/remake/kubeless-asks0.gpt6a.md#adopt-in-the-first-implementation)
  explains why niri uses memory and both adapters project actual before/after
  receipts. Upstream scheduler fairness remains deferred; no upstream edits.
- [Independent assessment](/design/remake/kubeless0.gpt61sm.md#observation-reuse-the-existing-umbriel-owner)
  grounds the choice not to build a second Umbriel inventory.
