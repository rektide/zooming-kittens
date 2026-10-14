---
type: DesignResearch
title: Kubeless mechanisms for the Longcat remake
description: Source-grounded reuse, limits, and small upstream opportunities for backend-independent observed layout control.
resource: /design/remake/kubeless0.gpt61sm.md
status: independent-proposal
generated:
  by: model:openai/gpt-6.1-sol#medium
  at: 2026-10-14
sources:
  - resource: opencode://ses_ec331458affeezVLOh5qkIN8Ag
    title: Independent Kubeless investigation, targeted tests, and integration clarification of transitive versus direct dependencies and optional upstream lifts
  - resource: /src/commands/longcat.rs
    title: Current state, manual resize inference, action loop, and restoration
  - resource: /src/niri/registry.rs
    title: Current lossy focus translation and socket ownership
  - resource: /src/export/stream.rs
    title: Existing raw niri event coverage
  - resource: /doc/discovery/taffy.md
    title: Prior inquiry, raw-export update and refinement questions, especially observation and tree ownership
  - resource: file:///home/rektide/src/kubeless/README.md
    title: Capability inventory, native-mechanism boundaries and dependency policy
  - resource: file:///home/rektide/src/kubeless/observation/session/src/session.rs
    title: Pull-driven sessions, snapshot and supplied-cursor constructors, cooperative polling
  - resource: file:///home/rektide/src/kubeless/observation/session/src/protocol.rs
    title: Native recovery and snapshot/change protocol
  - resource: file:///home/rektide/src/kubeless/observation/reflection/src/state.rs
    title: Candidate replacement, imperative facts and selected commit evidence
  - resource: file:///home/rektide/src/kubeless/observation/reflection/src/driver.rs
    title: Snapshot-first driver lifecycle and ownership
  - resource: file:///home/rektide/src/kubeless/observation/model/src/lib.rs
    title: Scoped identity and explicit unsupported/missing/known facts
  - resource: file:///home/rektide/src/kubeless/view/memory/src/lib.rs
    title: Atomic map replacement, checked candidates, coherent captures and local generations
  - resource: file:///home/rektide/src/kubeless/view/impact/src/lib.rs
    title: Former/current relation traversal and affected-root deduplication
  - resource: file:///home/rektide/src/kubeless/observation/relation/src/lib.rs
    title: Compact relation product that intentionally omits geometry
  - resource: file:///home/rektide/src/kubeless/view/publication/src/lib.rs
    title: Blocking, independent lagging, and seeded publication policies
  - resource: file:///home/rektide/src/kubeless/view/publication/src/lagging.rs
    title: Actual independent subscriber and drain-then-lag behavior
  - resource: file:///home/rektide/src/kubeless/work/schedule/src/lib.rs
    title: Direct timed admission, merge, close policy and stream-ingress starvation
  - resource: file:///home/rektide/src/kubeless/work/schedule-policy/src/lib.rs
    title: Latched quiet/cap batching and earliest reconsideration semantics
  - resource: file:///home/rektide/src/kubeless/work/ready/src/lib.rs
    title: Retention until exact consumer admission
  - resource: file:///home/rektide/src/kubeless/work/execute/src/lib.rs
    title: Capacity, exclusion-key permits, bounded starts and cancellation
  - resource: file:///home/rektide/src/kubeless/operation/src/conditional.rs
    title: Conditional attempts and conflict-only recomputation, not invented remote CAS
  - resource: file:///home/rektide/src/kubeless/control/settlement/src/lib.rs
    title: Distinct native acknowledgment and observed settlement
  - resource: file:///home/rektide/src/kubeless/control/retained-actions/README.md
    title: Optional retained responsibility and its substantial integration contract
  - resource: file:///home/rektide/src/umbriel-rs/crates/umbriel-state/README.md
    title: Existing native inventory, continuity, change receipts and Kubeless integration
  - resource: file:///home/rektide/src/umbriel-rs/crates/umbriel-state/src/inventory.rs
    title: Native stamps, health, immutable capture interface and actual WindowChanges type
  - resource: file:///home/rektide/src/umbriel-rs/crates/umbriel-state/Cargo.toml
    title: Actual normal dependency on the narrow Kubeless memory leaf
  - resource: file:///home/rektide/src/umbriel-rs/crates/umbriel-client/Cargo.toml
    title: Impact and state are example dev-dependencies, not normal client dependencies
  - resource: file:///home/rektide/src/kubeless/view/memory/Cargo.toml
    title: Transitive view-change dependency beneath the memory leaf
  - resource: file:///home/rektide/src/umbriel-rs/README.md
    title: Current capture and inferred-layout capabilities and explicit missing protocol facts
verification:
  method: Source inspection and two scoped cargo nextest runs in the sibling Kubeless checkout.
  result: 159 tests passed; one test skipped across the two runs.
  limits: No Longcat implementation, integration compilation, live compositor mutations, performance measurements, or same-wave draft comparison.
---

# Kubeless mechanisms for the Longcat remake

## Recommendation

Use Kubeless as **selected mechanisms underneath native compositor adapters and Longcat policy**, not as the model of a desktop or a compulsory controller framework. The strongest immediate fits are:

1. Atomic observed inventories and rich before/after receipts.
2. Former/current relation impact when windows move or disappear.
3. Keyed reconsideration with genuine timer-driven debounce.
4. Capacity and effect-scope exclusion, if concurrent application becomes useful.

Start smaller than the whole controller stack. Umbriel already has a native Kubeless-backed inventory; reuse it rather than wrapping it in a second reflector. Longcat must still own its baseline, geometry interpretation, policy, command dependencies, and uncertainty about effects.

Both sibling projects are changeable. The most valuable Kubeless change exposed by this use case is **bounded ingress turns in the stream Scheduler**; this is a concrete gap, unlike independent lagging publication, which already exists.

**No Kubeless upstream change identified here is a prerequisite for the remake.** The existing direct APIs support observation → pure policy/solver → native execution. There is no proposed generic transport framework. Scheduler fairness is an optional convenience lift because bounded caller-driven intake into direct `Schedule` already avoids its greedy stream adapter.

### Dependency ownership: what arrives and what zk selects

The package manifests establish a narrower boundary than “Umbriel uses Kubeless” might suggest:

| Piece | Already arrives through Umbriel? | Recommended zk ownership |
| --- | --- | --- |
| `kubeless-view-memory`, with `kubeless-view-change` beneath it | **Yes, through `umbriel-state`**, not through the normal `umbriel-client` dependency graph alone. State privately owns its Reader/Writer and returns native `WindowChanges` receipts. | Consume `Inventory::install_windows_changes` and the native receipt on the Umbriel path. Add a direct memory dependency to the **niri observation crate** if it selects the same map/receipt mechanics; do not duplicate Umbriel state or add memory to the pure solver just for symmetry. |
| `kubeless-view-impact` | **No.** It is only an `umbriel-client` dev-dependency for examples, including `window_changes`; it is not a normal `umbriel-state` dependency. | Direct dependency in zk's impact/planning assembly where it projects native before/after receipts into both former and current affected regions. Reuse the demonstrated API, not an imagined built-in subscription-to-work pipeline. |
| `kubeless-work-schedule` | No. | Direct dependency in zk's app-owned control runner if selecting keyed timed admission. Drive direct `Schedule` with bounded ingress; its debounce/coalescing policy stays outside transport and the pure allocation solver. |
| `kubeless-work-schedule-policy` | No through Umbriel; schedule reexports its public timing types/functions. | A direct dependency only for a standalone clock-injected policy consumer. If the runner already uses schedule's reexports, no extra direct dependency is needed solely for `BatchWait`/`RequestTiming`. It is an alternative lower entrance, not a second timer system. |
| `kubeless-work-execute` / `kubeless-work-ready` | No. | Optional direct runner dependencies when keyed execution or a custom ReadyWork producer is actually useful. An initially serialized native execution owner need not adopt them. Declare ready directly if zk imports its crate path; Rust transitivity does not authorize direct imports. |
| Session, reflection, compact relation, publication, settlement, retained-action host | No. | **Not baseline dependencies.** Their conditional opportunities are described below; adopt individually only for an actual requirement. Native `Endpoint::subscribe` already supplies Umbriel transport/delivery ownership. |

Thus a small selected composition is native `umbriel-client` + `umbriel-state` in the Umbriel adapter, direct memory in the niri adapter if useful, and direct impact + schedule in app/control assembly. Even these direct mechanism choices are design recommendations, not requirements to wrap every stage in Kubeless. In particular, no new Kubeless publication layer is needed between `Endpoint::subscribe` and `Inventory`.

## Situation and current failure pressures

The current [Longcat implementation](/src/commands/longcat.rs) combines transport, observation queries, grouping, baseline retention, resize inference, allocation, actions, logging and shutdown in one type. Its async shell wraps synchronous `Socket::send`; adding Kubeless cannot make those calls nonblocking.

Concrete pressures for the remake:

- `handle_focus_gained`, manual-resize inspection, baseline capture and height application each perform their own queries. Their inputs can describe different compositor moments. A coherent local capture would at least make one planning pass internally consistent.
- `ColumnState` is keyed by `(workspace_id, column_idx)`. An ordinal is a position, not stable column identity: insertion, removal, moves and reconnect can invalidate its association with saved heights.
- `check_manual_resize_and_update_proportions` compares current geometry to **original**, not last commanded geometry. Ordinary Longcat resizing can therefore look like user intervention. Its first differing observation sets a timestamp; later differing observations do not restart it. This is not a trailing quiet-period debounce, and no timer invokes the method when input stops.
- `run_longcat` handles only Focus/Blur. `_handle_window_layout_changed` is unused. The [registry](/src/niri/registry.rs) ignores most native lifecycle/layout events, and turns `WindowFocusChanged { id: None }` into private state loss without a Blur event. It cannot be the remake's authoritative desktop feed.
- `handle_focus_lost` checks focused column index without checking the candidate window's workspace. Cross-workspace ordinal collisions can suppress restoration.
- Multiwindow writes stop at the first failure, leaving a possible partial prefix with no explicit retained execution record. `Response::Handled` is treated as enough; no readback establishes the resulting allocation.

These are source observations, not live reproductions. They support a remake around coherent observations and explicit effect records, rather than a mechanical extraction of the current methods into traits.

## Concrete reuse map

| Longcat need | Current Kubeless entrance | What it contributes / what remains native |
| --- | --- | --- |
| Install one complete window family | `kubeless-view-memory::{Replacement::try_from_entries, Writer::replace_changes}` | Duplicate checking and atomic local replacement; identity validation, order and source health remain adapter policy. |
| Maintain staged initial acquisition | `kubeless-observation-reflection::Reflection::apply_retaining` | Snapshot candidate and whole-map commit; useful only if native acquisition actually needs staging. |
| Recover snapshot/change sessions | `kubeless-observation-session::{streaming, Recovery}` | Pull-driven protocol and deferred classified recovery; no invented source resume, no reconnect backoff. |
| Recompute old and new layout regions | `kubeless-view-impact::{visit_relations, affected_roots}` | Both sides of move/delete/replacement; native projection chooses which region those windows affect. |
| Debounce resize adaptation | `kubeless-work-schedule::{schedule_with_merge, ScheduleRequest}` | Separate identity/payload, quiet batching, optional maximum delay and retained refused work. |
| Deterministically test timing | `kubeless-work-schedule-policy::{BatchWait, RequestTiming}` | Caller-clock facts independent of Tokio; application selects batching episodes and hard gates. |
| Serialize effects while keeping input alive | `kubeless-work-execute::Executor`, `ReadyWork` | Capacity and projected-key exclusion, not dependency planning or atomic remote batches. |
| Expose status to optional consumers | `kubeless-view-publication::lagging::bounded` or seeded equivalent | Slow consumer loses continuity explicitly without blocking producer; not byte accounting or automatic resync. |
| Explain command acceptance vs actual geometry | `kubeless-control-settlement::{acknowledge_then_settle, settle_acknowledged}` | Retains acknowledgment through ordinary observation failures; application supplies the real geometry predicate and timeout. |

### Observation: reuse the existing Umbriel owner

[`umbriel_state::inventory::Inventory`](file:///home/rektide/src/umbriel-rs/crates/umbriel-state/src/inventory.rs) already owns source order, family readiness and local stamps over `kubeless-view-memory`. `install_windows_changes` returns actual replacement evidence; `WindowChanges::is_continuous` distinguishes first/stale replacement from continuous comparisons. Use that API in the Umbriel adapter. Do not materialize the same native windows in a second Kubeless view merely to match a niri code path.

For niri, build a native observation owner from full `WindowsChanged` plus window open/change, close, focus and layout changes. [Raw export](/src/export/stream.rs) already demonstrates these event variants; it is a discovery precedent, not yet a reducer. Install all rows of a layout-change event as one local batch before planning. Preserve the distinction between source array order, niri scrolling coordinates and arbitrary map iteration.

Expose a normalized **planning capture**, not necessarily a shared implementation of the two native inventories. Its useful contents are backend/session scope, source interval/health, current focus or activation semantics, windows, layout-relevant geometry and provenance, and required family stamps. Local atomicity must not be advertised as a globally atomic compositor snapshot.

The memory leaf's `Generation` is process-local commit order, not a compositor revision. `SubjectRef`/`Evidence` from observation-model are optional useful vocabulary: scope IDs by backend and session; distinguish unsupported layout facts from missing facts and known absence. Do not force every field into a generic resource object.

### Sessions: conditional fit, not automatic adoption

`session::streaming` has a real initial-snapshot/steady-change contract; `Frame::SnapshotComplete` establishes its transition. Its cursor means volatile resume position. Niri and Umbriel should not turn a receive counter into a claim of resumability. Umbriel inventory explicitly says its epoch/position are **not** source resume tokens.

Prefer native subscription/reconnect ownership already supplied by umbriel-rs. For niri, use Kubeless Session only if it simplifies a faithful adapter: reconnect must reacquire a complete fresh state and emit an explicit continuity break. Otherwise a native feed plus direct memory replacement is enough. `Reflection` also permits imperative changes before snapshots; its convenience `Driver`, however, starts expecting SnapshotBegin, so replacing the driver and immediately feeding old deltas is not equivalent to retaining the same live driver.

Readiness must include current health, not just “a snapshot was once installed.” `Reflection::mark_ready` is explicit and independent of source health; retaining old maps across reconnect does not make them safe for mutation.

### Relations: old and new region impact, not a second geometry database

Given an actual window commit, project each before/after handle to the region in which a layout change matters. A moved window A→B requires both A and B. Removal requires the former region even if querying current state can no longer find the window. Full replacement naturally includes omitted windows as former-side evidence.

The compact `kubeless-observation-relation` product retains only child→root rows. It is useful for a membership-only observer, but is **not sufficient as Longcat's canonical state** because allocation needs geometry, baseline weights and other window facts. Start with full native receipts and `view-impact`; a second compact relation product is not justified yet.

Filter actual change evidence by **layout-relevant equality**, not merely membership equality. Geometry or focus changes within an unchanged group still matter. Conversely, a complete replacement receipt includes equal touched rows; unfiltered receipts would trigger work for every census and encourage feedback loops. Keep an unfiltered record only where diagnostic provenance needs it.

### Timed work: recompute from current state, do not replay old plans

Use a payload such as `{ region, reasons, observed_generation, policy_generation }`; merge dirty reasons and keep useful latest evidence. At execution, reacquire the current planning capture and produce a fresh plan. Do not queue a sequence of historical “focus A, focus B” resize batches and replay it after input has advanced.

Two kinds of time must not be conflated:

- **Resize quiet batching:** `ScheduleOptions.debounce` plus a selected `max_delay` can bound batching under churn. Once elapsed, the batch latches ready; later notices cannot indefinitely restart it.
- **Continuous focus dwell or a hard user-intervention holdoff:** identity changes reset/cancel a native episode; a hard holdoff is a separate gate. `RequestTiming` selects the **earliest** reconsideration request and does not enforce a retry embargo. A capped resize batch is also not proof that the user stopped dragging.

Baseline learning requires the native policy to compare observed geometry with last planned/acknowledged/settled geometry, account for rounding and compositor constraints, and classify ambiguous changes. A timer cannot prove causality. Keep learned preferences separate from immutable configured defaults, unlike today's global configuration mutation.

Use direct `Schedule` with bounded ingress turns first. `Scheduler::poll_ready` greedily drains ready input before polling timers; an endless ready stream can starve emission. Executor's bounded **start** policy does not fix starvation inside that producer poll. `CheckedIngress::poll_into` can already transfer at most one request per call if its refusal contract is useful.

### Effect scope and cancellation

Keep distinct keys for observation identity, debounce coalescing and execution exclusion. Begin conservatively with a serialized backend/session effect scope, or a workspace scope only once native effects are known not to cross it. A per-window key is too weak when resizing one tile redistributes siblings. A per-column key is also unsafe when indices shift or an action batch touches two regions.

`Executor` supports one active future per projected key, not arbitrary overlapping sets of keys. Use a broader key for now. Its readiness gate is a start gate, not automatic cancellation/revalidation of already admitted work. Revalidate native identities, membership and policy before submission; after an await, inspect whether the plan was superseded before sending subsequent actions. Canceling a local future is not remote rollback.

Do not put synchronous niri socket work on the async polling thread. Select a native async adapter or a dedicated blocking owner with bounded command transfer; keep event acquisition independently driven so command latency cannot hide focus/layout changes.

## What Kubeless cannot establish

**Remote compare-and-swap.** `view-memory` anchors fence local map writes only. `operation::conditional_replace` retries after genuine native conflict evidence; it cannot transform “query geometry, then unconditional resize” into a server-atomic conditional write. `Outcome::Unknown` is terminal to that generic retry recipe. Preserve uncertainty instead of labelling a transport timeout as safe rejection.

**Atomic multiwindow application.** A batch remains ordered native effects unless the backend provides a transaction. Retain each action's target, result and observed settlement; explicitly report partial completion. No automatic replay or rollback follows from using Executor.

**Manual-resize attribution.** Matching later geometry to a target establishes observed compatibility, not who changed it. Mismatches can be user action, constraints, animation, rounding, stale input, or another controller. This is Longcat policy and backend evidence, not a generic Kubeless field.

**Exact layout identity.** Umbriel's current desktop shape is geometry-inferred and explicitly not an exact tree; full native column membership/split ratios are absent according to its README. Kubeless cannot repair missing protocol semantics. A native exact-topology API may be worth adding to Umbriel; that decision belongs to the parent investigation.

**Durable restoration.** All these mechanisms are volatile. Preserve baseline state across disposable local passes, but do not blindly restore old heights after session loss or changed topology. A baseline should retain window/session identity and the context in which it was captured; restoration is an explicit plan over still-valid targets, not garbage collection.

## Worthwhile changes to Kubeless

All proposals in this section are **optional lifts**, not blockers for the native observation/policy/execution architecture. The bounded direct Schedule workaround is sufficient now; the consumer probe and documentation changes improve reuse evidence and discoverability. Neither a multi-key executor nor retained-action host work is required to land a serialized first implementation.

### 1. Bounded Scheduler ingress — strongest concrete upstream request

Add a selectable ingress-turn budget to the **stream adapter**, retaining current greedy behavior as an explicit policy if compatibility needs it. After the budget, service existing timers/readiness, self-wake if further input may remain, and yield. Do not add another queue or alter Schedule's deadline truth.

Acceptance proof: with a permanently-ready source, a due key still reaches the consumer within bounded ingress turns; two keys continue making progress; accepted payloads remain retained on consumer rejection; EOF retains the selected close policy. Test separately from Executor admission budgets, since they bound different layers.

Existing Session has `COOPERATIVE_POLL_BUDGET`, and Executor has Count/Policy: these are design precedents, not already equivalent Scheduler coverage.

### 2. A real non-resumable desktop consumer recipe

Add a small generic probe using complete native snapshots, explicit loss/epochs, actual replacement receipts, before/after impact and timed keyed reconsideration. Demonstrate an unknown write followed by observation rather than replay. This would exercise the low-level composition without Kubernetes or compulsory retained-action bookkeeping.

Keep Longcat behavior and compositor dependencies in this repository; upstream only the reusable trace and recipe. Umbriel already provides native inventory evidence, so collaborate rather than write a competing snapshot adapter.

### 3. Clarify and test separate limits, before adding new mechanisms

`Schedule` has unbounded cardinality; publication capacity counts items, not bytes; rich replacement evidence can retain whole before/after pictures. For a desktop, normally small key counts may be sufficient. First measure/limit native frame bytes, queued commands, retained baseline windows and observer payloads at application boundaries. Add generic capacity/refusal support only if a concrete admission path needs it, returning rejected owned work rather than silently pruning.

The root Kubeless README currently describes publication primarily as shared blocking backpressure, while the actual module exposes independent lagging policy too. Improve entrance documentation so consumers do not propose or reimplement an already supplied capability. The latter's `Publisher::publish` has no payload-Clone requirement; `Subscriber<T>` requires Clone when delivering, so use shared handles for expensive records.

### Not first: multi-key exclusion or the full retained-action host

These are possible later investments, not required for the first remake. Multi-key exclusion needs an atomic reservation law and a demonstrated workload that cannot tolerate broad serialization. The retained-action host is useful if duties/results must survive driver replacement and support explicit check-only continuation, but asks/plans/targets/acceptance/history are a substantial contract for this small controller. Start with one native owner and explicit action records. Do not recreate its responsibility ledger in Longcat if that stronger requirement later becomes real.

## Domain-grouped layout implications

Two plausible shapes, both with crates under meaningful root domains:

```text
# Concern-first: recommended while topology and connection are separate questions
desktop/model/          # planning captures, scope/health, IDs and geometry evidence
desktop/niri/           # native observation reducer and niri transport integration
desktop/umbriel/        # adapt umbriel-rs, do not fork its inventory/client
layout/longcat/         # baseline/policy/planning, optional solver module
control/runner/         # selected Kubeless timing/execution and native effect records
app/cli/               # existing commands and configuration assembly

# Backend-first: useful if each backend grows several substantial native concerns
backend/api/            # narrow planning-facing contracts
backend/niri/           # connection + state modules in one crate initially
backend/umbriel/        # existing native library adapter
layout/model/           # normalized region/constraint interpretation
layout/longcat/          # allocation and restoration policy
app/cli/                # app-owned orchestration, including Kubeless composition
```

Neither requires one crate per module or copying Kubeless's large package inventory. In the first shape, `control/runner` can begin as an app module until independently useful. The important dependency boundary is pure Longcat planning separated from native observation/effects. Connection abstractions should not pretend to provide layout capabilities; topology adapters should not own socket reconnection. Native layout modes can reject unsupported policy rather than guessing a shared column model.

The earlier [Taffy refinement questions](/doc/discovery/taffy.md#appendix-a--refinement-prompt) are complementary: Taffy can compute target allocation, while Kubeless maintains input/change/work mechanics. Neither should be made the sole owner of observed state, saved baseline and desired output simultaneously. A pure solver over a planning capture is easier to test than treating solver-node IDs as compositor identities.

## Suggested first coherent proof and open choices

Build a read-only trace first: backend observation → native receipt → normalized capture → affected regions → timer-driven current-state Longcat plan. Print observed/baseline/desired separately; fake both backends in deterministic tests. Then enable native mutations with retained partial results and readback. This is a proposed landing sequence, not implementation authorization.

Required traces include focus A→B in one group, A→another workspace with the same column ordinal, a moved/deleted window, column renumbering, self-induced geometry, a user resize burst ending without another event, impossible minimum-height budgets, source loss during pending work, a partial action prefix, and an unknown result followed by fresh observation. Slow optional status subscribers must not pause core input.

Open decisions for the architecture synthesis:

1. Is focus response immediate, stable-active dwell-based, or selectable? Do not let resize debounce accidentally choose it.
2. What is the native effect conflict scope per backend/action, and can actions target exact window identity without focus switching?
3. Which layout modes provide exact enough grouping for restoration, and where should inferred geometry make the controller refuse to act?
4. After manual intervention, learn per-region weights, per-window baselines, or global preference? These have different persistence and ownership meanings.
5. On source loss, preserve an owed restoration for explicit later reassessment or abandon it with a clear report? Never silently restore stale identities.
6. Are status and replay diagnostics part of this remake, or only a read-only planning proof? A current capture is not a historical event log.

## Verification account

Inspected the sibling checkout reached as `~/src/kubeless` (build paths resolve to `~/src/kube-kubeless`). Its observed working-copy commit was `854058d8`, above `d5587443`; this names the local investigation point, not an immutable published dependency contract. Do not infer reproducible adoption from the shared `0.1.0` package version. Pin a reviewed Git revision when a remote is available, or explicitly document the selected sibling development dependency.

Executed:

```sh
cargo nextest run -p kubeless-work-schedule -p kubeless-work-execute \
  -p kubeless-view-impact -p kubeless-observation-reflection
# 73 passed

cargo nextest run -p kubeless-view-memory -p kubeless-view-publication \
  -p kubeless-observation-session -p kubeless-work-schedule-policy
# 86 passed, 1 skipped
```

Both emitted pre-existing manifest lint-inheritance warnings for e2e/examples. No live compositor was used. This verifies the inspected mechanisms' selected existing tests, not the proposed Longcat composition or all Kubeless feature profiles. Same-wave other-author drafts were not read.
