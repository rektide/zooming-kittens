---
type: Research
title: Umbriel Longcat control and reusable stream tracking
description: Source and live-deployment findings, a concrete conditional scrolling-allocation implementation path, and reusable stream ownership for Longcat and activity logging.
resource: design/longcat/umbriel-control-stream0.gpt6a.md
status: proposed
generated: { at: 2026-10-22, by: 'model:openai/gpt-6-astra#medium' }
sources:
  - resource: opencode://ses_e9a70297dffe6dQ5UJP0up8m7T
    title: Independent source/deployment research followed by authorized native conditional allocation and Rust bindings implementation
  - resource: ../remake/architecture0.gpt6a.md
    title: Earlier three-boundary design and native requirements
  - resource: ../../desktop/umbriel/src/reducer.rs
    title: Current Inventory-backed semantic projection and unavailable grouping
  - resource: ../../desktop/umbriel/src/source.rs
    title: Current independently driven source and bounded observer lifetime
  - resource: ../../app/zk/src/runner/control.rs
    title: Existing Longcat plan and niri-specific per-target execution
  - resource: ../../app/zk/src/commands/systemd.rs
    title: Existing generator omits Longcat subcommand and assumes niri target
  - resource: file:///home/rektide/src/umbriel-rs/design/layout-control/longcat-needs0.gpt6a.md
    title: Conditional native sizing and restoration requirements
  - resource: file:///home/rektide/src/umbriel-rs/design/layout-control/upstream-refresh0.gpt6a.md
    title: October 22 upstream refresh and observation additions
  - resource: file:///home/rektide/src/umbriel-rs/design/layout-control/practical-sizing0.gpt6a.md
    title: Existing focused native fraction operations and their smaller contract
  - resource: file:///home/rektide/src/umbriel-rs/design/handoff/zk-focused-sizing0.gpt6a.md
    title: Earlier useful focused-sizing consumer handoff
  - resource: file:///home/rektide/src/umbriel-rs/crates/umbriel-client/examples/active_window.rs
    title: Private runtime loop with queued-observation precedence over dwell timers
  - resource: file:///home/rektide/src/umbriel-rs/crates/umbriel-client/examples/window_changes.rs
    title: Repeated decode/install/continuity composition around native change receipts
  - resource: file:///home/rektide/src/umbriel-rs/crates/umbriel-state/src/active.rs
    title: Existing pure stable-active reducer and five-second default
  - resource: file:///home/rektide/src/umbriel-rs/design/activity/records0.gpt6a.md
    title: Selected tracing-journald architecture and accepted-plus-metadata behavior
  - resource: file:///home/rektide/src/umbriel-working/CONTRIBUTING.md
    title: Native implementation conventions and isolated headless harness
  - resource: file:///home/rektide/src/umbriel-working/SCOPE.md
    title: Native inspection/control mechanisms in scope; application policy stays outside
  - resource: file:///home/rektide/src/umbriel-longcat-control/docs/user/layout-control.md
    title: Implemented conditional allocation wire contract and restoration semantics
  - resource: file:///home/rektide/src/umbriel-rs/crates/umbriel-protocol/src/model/layout.rs
    title: Implemented reusable basis, native weights, target and committed geometry, request and receipt models
  - resource: https://github.com/noctalia-dev/umbriel/blob/5d657581e27e1a3fe604693779ca1405610f4f21/src/server/ipc_commands.cpp
    title: Current upstream membership, tab and content-geometry serialization
verification:
  method: Initial read-only deployment research, followed by authorized native debug build, pure scrolling tests, isolated headless allocation harness and concurrent stress checks; Rust scoped nextest and strict Clippy.
  limits: No mutation or restart of the running desktop. Initial proposal below is preserved; the implementation addendum records delivered API and checks. Reusable tracker/service work belongs to concurrent agents.
extensions:
  ticket: zk-longcat-return-native-control
  related_work: [zk-longcat-return, umbrs-stream-tracking, umbrs-activity]
---

# Umbriel control and stream tracking

## Implementation addendum

After the research checkpoint, the parent authorized implementation and selected
upstream `5d657581e27e1a3fe604693779ca1405610f4f21` as the base. An isolated named
worktree at `/home/rektide/src/umbriel-longcat-control` preserves the composed local
branch and running desktop. Native first implementation commit: **`14dab546`**;
Rust first implementation commit: **`c318a58f`**. Native refinement **`5432ace1`**
and Rust refinement **`8d20ca95`** add output
identity to the basis, reject pinned groups, and extend decoration/inactive-restore
verification. The current contract is maintained in the native
[layout-control guide](file:///home/rektide/src/umbriel-longcat-control/docs/user/layout-control.md).

The native API now exists: `layout-state` query/event and `layout-allocate` request.
Rust `model::layout_state_request`, `Family::LayoutState`, `LayoutAllocation`,
`LayoutAllocationResult`, `LayoutBasis`, `LayoutWeights`, `LayoutColumn` and
geometry/constraint types are public. Inventory retains the family with independent
health/stamps. Native query support is opt-in; the baseline dump's seven-query
catalog does not start requesting a new unsupported operation on old servers.

Save original `LayoutWeights` separately. Send **last accepted basis** with every
next write/restore. The whole-group receipt returns previous weights and the new
accepted basis/targets; it is not N per-window operations. Optional
`target_content_extents` rejects a vector that the actual native constrained
preview cannot represent, before changing live weights.

`LayoutMember.constraints` are native **tile** constraints; height bounds include
title-bar height. `row_budget` is a tile budget after edge gaps. A vertical-content
solver subtracts member title-bar heights for its budget/bounds and adds them back
for tile weights. Targets distinguish tile/content from committed content. Output
identity participates in the basis even if two outputs have equal geometry.
Current-state comparison includes exact weights, so same-pixel weight edits reject;
the documented ABA limitation remains.

Verification delivered: native debug executable build, pure scrolling allocation
tests (both secondary axes), real-client headless vector allocation/commit, stale
and malformed write rejection without mutation, same-pixel weight-edit rejection,
focus predicate, native-weight restoration, topology invalidation, initial/update
stream delivery, and concurrent isolated stress runs. The strengthened harness
uses forced title bars and restores while another workspace remains active.
Rust protocol/session/client/state scoped run passed **160 tests**; the three
layout protocol tests passed again after adding output identity. Strict scoped
Clippy passes after the concurrent tracker owner resolved its enum warning.
Five scoped Rust doctests and four existing native IPC/state harness checks also
passed. Raw positive subfloor weights are accepted and preserved: tab dissolution
can produce them, while native arrangement still applies its effective `0.05`
floor. Pure/Rust tests cover this restoration case and the final headless check
passed after that correction.
Native `clang-tidy` could not run because the executable is not installed; the
debug build uses native `-Werror=true` and succeeded.

**Delivery still required:** ZK's parent-owned whole-group adapter/controller and
service, then carry the native commits into the selected composed deployment
branch, rebuild that composed binary and schedule a deliberate graphical-session
transition. Installing this upstream-based debug binary as a replacement would
drop the composed branch's independent changes. No live restart, replacement or
mutation is part of this work. Native/Rust work is tracked by `umbrs-layout-control`
and the consumer rendezvous remains `zk-longcat-return-native-control`.
Explicit deployment follow-up: `zk-longcat-return-native-deploy`.

## Recommendation

**Implement a small native scrolling-allocation API, then connect the existing
ZK Taffy plans to it.** Existing focused fraction actions can deliver useful
one-shot resizing immediately, but are not an implementation of the requested
identity-targeted, baseline-preserving Longcat controller. The native work is
tractable: Umbriel already owns row weights, edge-gap weights, member order,
constraints and arrangement; expose and conditionally assign that state.

I can own the C++ native control slice and corresponding Rust protocol bindings
after parent integration. Parent retains ZK controller/model changes. A deliberate
compositor build/session transition is part of delivering this on the real desktop.

## Four distinct source/deployment coordinates

| Location | Observed coordinate | Meaning |
|---|---|---|
| `~/src/umbriel-rs` | parent `72267ad4b628bfb0aa508d19c083478e6ce8dda2`; clean empty working copy | Current Rust library plus research; remote `rektide/umbriel-rs` |
| `~/src/umbriel-working` | parent `1845885e622a6cd923b0155cd1f967fcfe793714`; clean | Local composed compositor branch, eight commits beyond common base |
| `~/archive/noctalia-dev/umbriel-upstream` | `5d657581e27e1a3fe604693779ca1405610f4f21`; clean | Current locally fetched `noctalia-dev/umbriel` upstream; 121 commits beyond common base |
| Running `/proc/12558/exe` | `umbriel 0.1.0 (51e6528ecf3d)` | Actual deployed compositor; not either worktree tip |

The local branch and upstream share base `5731190d142b3e899412962d1220473f38525a90`.
The archive's original checkout remains detached at `7666af0a`; all three C++
directories are Git worktrees of that repository. Do not reset the local branch
to upstream or treat its `origin/main` as its checked-out source. No fetch was
needed for this research; “current” here means the locally fetched October 22
upstream coordinate, not a claim that the remote cannot have moved again.

No nested AGENTS file was found in the two commissioned sibling trees. Native
`CONTRIBUTING.md` and `SCOPE.md` were read. ZK started with unrelated ticket JSONL
changes; those belong to parent/cooperating work and are not this research commit.
`bd onboard` suggested `bd prime`; it was not run.

## Live read-only observations

At `/run/user/1000/umbriel-wayland-0.sock`, one-shot queries returned:

- Windows: **19**, exactly one seat-active; no `column`, `row` or tab fields.
- Workspaces: **3**, all `scrolling`, existing old-model fields.
- `outputs` and `version`: explicit `unknown command` replies.
- Subscription to windows/workspaces: both initial full snapshots arrived.

The running binary's revision was obtained by executing `/proc/12558/exe --version`,
not guessed from a socket filename or source checkout. `git log` resolves it to
`51e6528ecf3d787531d9f8e536df24196edea17d`, “isolate launched applications in
transient scopes.” Query summaries retain no window titles.

`umbriel.service` is active, started October 19 at 17:10:55 UTC, with
`ExecStart=/usr/local/bin/umbriel` and unit file
`/usr/local/lib/systemd/user/umbriel.service`. Its journal contains compositor
diagnostics/application launches. No Umbriel activity service appeared in the
user unit inventory. This establishes a running compositor, **not a deployed
structured active-window activity tracker**. No service was restarted.

## What already works

`umbriel-protocol::action::Action` has primary/secondary set and modify methods,
directional resize methods and cycle actions. `ExtentFraction`/`ExtentDelta`
validate arguments; `Endpoint::request` preserves native reply and transmission
uncertainty. A set-secondary fraction changes the current focused allocation unit,
preserving other units' relative weights. Native targeting happens at handling
time; a preflight focus query cannot make this by-ID.

Current upstream `IpcCommands::windows` lines 633–655 reports column/row and tab
metadata. The local working branch reports tabs but not column/row; the deployed
binary reports neither. Rust `model/inventory.rs` still exposes the old fields.
Unknown-field tolerance lets it consume newer snapshots, but those facts are not
retained in typed `Window` or used by ZK. New optional fields must distinguish
absence on an old server from reported `-1` membership or `false` tab state.

ZK already has the Taffy/episode/runner implementation. Its Umbriel reducer owns
one `Inventory`, projects geometry and focus, but sets `group: None`,
`Evidence::Unavailable`, and read-only capabilities. `Control::request` returns
`zk_niri::HeightRequest`; lifting the initial backend refusal alone cannot work.
The integration needs actual groups/budget/axis and a bulk Umbriel execution path.

## Concrete native implementation slice

### Repository and interface

Develop against a dedicated branch/workspace from the chosen compositor base;
recommend current upstream for an upstreamable mechanism, then deliberately carry
it into the local composed branch if that remains the deployment target. The
parent must choose that integration base before implementation. Local build tools
are present (`meson`, pkg-config `wlroots-0.20 = 0.20.2`); a successful build is
still to be established.

Add a coherent scrolling-layout query/event and structured conditional allocation
request. Proposed names are illustrative, not an existing API:

```text
layout-state -> instance, workspaces[mode, axis, columns[basis, members,
               native weights/gaps, target boxes, allocation span, constraints]]
layout-allocate {target member ID, expected basis, optional expected active ID,
                 replacement row weights and top/bottom gap weights}
  -> accepted new basis and constrained target state, or rejection without writes
```

The minimum useful scope is ordinary untabbed scrolling groups. Explicitly reject
tabbed, fullscreen/maximized or unsupported layout groups in this first operation;
observations should still report them so consumers explain admission. Both axes
can be described natively; ZK can initially admit only its existing vertical
secondary-axis contract. This is a small supported prefix, not geometry guessing.

### Existing native mechanisms to reuse

- `Workspace::scrollingLayout`, `layoutConfig`, tiled usable area and
  `previewArrangedLayout` supply the owner and a fresh target preview without
  client configures. Use the native layout's constraints source, not IPC-side
  duplicate size-hint logic.
- `Column` already retains ordered `views`, `heightWeights`, `topGapWeight`,
  `bottomGapWeight`, width and tab state. `ScrollingLayout` exposes weight getters
  and setters; add one **validate-all then assign-all** method rather than calling
  focused setters or exposing mutable columns to IPC.
- `Workspace::markArrange` schedules native arrangement. Do not call
  `setFocusedHeight`: it clears maximize state and calls `ensureFocusedVisible`.
  An explicit member-targeted workspace method should leave focus/viewport policy
  unchanged and schedule the relevant layout event.
- `Ipc::handleRequest` already dispatches structured requests (`subscribe`, audio,
  screencast). Add parsing there or a dedicated handler rather than forcing a
  two-value operation into one fraction action argument. Current upstream event
  masks are already `uint16_t` with nine families; the older local branch has
  `uint8_t`, so copying only a new event constant across branches is insufficient.

### Conditionality without a sprawling revision retrofit

A concrete first contract can compare an **expected complete basis** with current
native state on the single event loop: compositor instance, workspace/member IDs
and order, supported mode/axis, row/gap weights, relevant tab/state flags, workarea,
gap/decoration and constraints. Reject nonfinite/negative or below-floor weights,
wrong cardinality, missing members and stale basis before assignment. Return the
new complete basis for the next write/restore.

This is compare-current-state semantics. It deliberately does not claim to detect
an intervening edit that returns to exactly the same basis (ABA). If detecting
every intervening mutation is required, add a native control revision at the
central mutation paths and test pointer/config/topology changes; a hash of current
geometry is not such a revision. Do not build an unbounded server token store.
The expected-active predicate is checked only for emphasis, not an inactive-group
restore. Instance binding prevents a restart from reviving stale authority.

### Taffy pixels to native weights

`scrolling.cpp::arrange` subtracts edge padding and inter-unit gaps, then partitions
`stackCross` by weights, rounds each row, and assigns the final row the remainder.
For untabbed feasible integer target extents plus integer edge gaps summing to that
span, choosing those extents as positive weights makes the desired allocation
representable without changing the native solver. Return previewed targets and
verify the round trip rather than promising an untested floating-point identity.

The complications are concrete: native constraints clamp after proportional
allocation, title/tab decoration changes tile-versus-content sizes, and edge gaps
consume part of the same budget. Export these facts and reject infeasible requests
or report constrained targets. Initially preserve edge-gap allocation; ZK's solver
receives the remaining row budget. Save and restore original **native weights and
edge weights**, not reconstructed committed pixel fractions. Restoring those
weights under a changed workarea is not identical pixels; the first basis check
should reject that changed context.

This bulk request fits Taffy better than per-window percentage setters: one native
turn applies a vector, with no focus round trip or intermediate denominators.
ZK must retain a native restore payload alongside its policy baseline and compile
the whole plan, instead of faking N successful niri height operations. Accepted
native targets and later committed geometry are separate evidence.

### Verification and delivery

Add pure layout tests for vector allocation, native-weight restoration, integer
rounding and constraints. Add isolated headless harness checks for wrong expected
members/basis, focus changing before send, inactive workspace restore, both axes,
window removal, client commits and initial/subsequent layout events. Check focus
and active workspace are unchanged. `CONTRIBUTING.md` supplies `just check`,
`just check-stress`, and deterministic `settle`/animation-clock test helpers.
`settle` is test-IPC gated and is not a production capability to assume.

Then add Rust scripted-peer protocol/model tests, ZK bulk-plan/restore replay,
and a scratch-column trial on the selected compatible compositor build. A new
binary on disk does not upgrade PID 12558; plan an explicit session transition
with the user rather than stopping their graphical session from a background task.

## Library-ized stream tracking

Transport is already a reusable library: independent socket driver, bounded
observers, explicit epochs/gaps/lag, cancellation-aware command ownership and
opt-in reconnect. `umbriel-state::Inventory` already owns rows/receipts and
`StableActiveReducer` already provides deterministic identity dwell. Missing
reuse is primarily **composition**:

1. Lift frame classification, typed decode, stamp installation and family-specific
   invalidation from the window-changes example and ZK reducer into a reusable
   optional tracking module. Return captures/change evidence, not formatted text.
   Keep `Inventory` the sole native row owner and retain per-family freshness.
2. Lift `active_window.rs::next_stable` from private example code into reusable
   runtime/example-app support that composes the pure reducer. Preserve bounded
   queue draining before polling a due timer, independent source ownership and
   explicit observer loss. This module depends on client/state; raw client need
   not gain mandatory state, tracing, journal or Polars dependencies.
3. Keep Longcat's immediate focus/layout projection distinct from activity's
   five-second acceptance policy. A journal writer and Longcat may share the
   upstream composition without Longcat consuming delayed journal events.

Two reasonable homes are an optional client tracking feature/module or a small
runtime composition library beside the activity example-app. The latter keeps
transport dependencies unchanged and naturally exposes a library plus binary.
Choose once with the activity owner; do not independently copy the loop into ZK
and a new service. Library API must distinguish `ready`, accepted none, gap,
reacquired state and metadata change.

The current reducer refreshes metadata silently after emitting a stable identity;
it emits no post-acceptance metadata event. The selected activity records design
already requests those updates. Preserve that requirement when extracting: simply
wrapping today's example is not complete activity behavior. Initial-connect retry,
observer reattachment and protocol-error policy also exceed the client's existing
fixed-delay reconnect option.

## Activity and tracking handoff

Native epic `umbrs-activity` already has records, journal, CLI, unit, history,
notify, reconnect and shutdown children. Its selected architecture is tracing
events with `tracing-journald`, not a hand-serialized JSON blob in MESSAGE. The
running compositor's own journal entries do not satisfy those tickets. Append
deployment evidence there; avoid creating a competing ZK-owned activity service.
Broad journal/OSC precedent and optional Kopacetic integration belong to the
other commissioned agents.

Three runnable/deliverable things must stay explicit:

- **Compositor:** running now as `umbriel.service`; it owns the desktop and would
  need a session transition to run the new native allocation API.
- **Rust observation example:** the existing read-only example can be built/run
  now with `UMBRIEL_SOCKET=/run/user/1000/umbriel-wayland-0.sock cargo run -p
  umbriel-client --example active_window` from umbriel-rs. This research did not
  build/run it. It prints tab-separated stable identity records after five seconds;
  putting it under systemd would capture text, not implement selected structured
  journal records or post-acceptance metadata updates. Deliver reusable runtime
  code plus a real library-backed example service, then install/start that service
  independently of native compositor upgrades.
- **ZK sizing service:** depends on a runnable native control path, uses the
  immediate sizing policy rather than the activity dwell, and needs an explicit
  Longcat command. The current `commands/systemd.rs` emits bare executable startup
  with `After/Wants=niri.target`; it is not an Umbriel Longcat unit. Parent owns
  `zk-longcat-return-service` for this correction.

The completed [journal survey](../activity/journald-survey0.gpt6a.md) is the
parent-supplied companion for reader/writer precedents; this research does not
independently re-verify that survey's source claims. Existing activity history
tracking remains the native owner; no duplicate history ticket was created.

Created `umbrs-stream-tracking` for the reusable composition and
`zk-longcat-return-native-control` under `zk-longcat-return` for the native/Rust/ZK
rendezvous. These are implementation opportunities; this research does not close
them. Existing `zk-longcat-user-layout-changes` remains relevant: native basis
rejection must suspend stale plans and restoration rather than silently learn
the controller's own effect as a user baseline.

Connections: the [remake architecture](../remake/architecture0.gpt6a.md) remains
the policy/observation/execution boundary; this document replaces its missing-native-
API stopping point with a bounded implementation path. The upstream
[practical sizing guide](file:///home/rektide/src/umbriel-rs/design/layout-control/practical-sizing0.gpt6a.md)
still supports independent current-focus commands without waiting for this work.
