---
type: Reference
title: Layout calculation and Longcat episodes
description: Pure solver and episode APIs, separated from native desktop observations and command transport.
resource: layout/README.md
status: implemented
generated:
  at: 2026-10-15
  by: model:openai/gpt-6.1-sol#xhigh
sources:
  - resource: opencode://ses_ec2fec1aaffe5IDnmC0nIJUQ04
    title: Correct fractional baseline attribution before effects and verify integer restoration tracking
  - resource: opencode://ses_ec3149872ffe9daEmFbCXAaWBp
    title: Pure Taffy allocation and baseline-relative Longcat episode implementation
  - resource: ../design/remake/architecture0.gpt6a.md
    title: Approved observation, policy, solver and native execution boundaries
  - resource: ../design/remake/kubeless-asks0.gpt6a.md
    title: Consumer-owned scheduling and retained native execution evidence
  - resource: ../doc/discovery/taffy.md
    title: Earlier Taffy inquiry, superseded in tree/state ownership by the remake
  - resource: file:///home/rektide/archive/DioxusLabs/taffy/Cargo.toml
    title: Taffy 0.10.1 feature selection for real Flexbox calculation
verification:
  method: Scoped nextest, clippy with warnings denied, and rustfmt checks.
  result: 46 tests passed across zk-layout and zk-longcat; clippy and formatting passed.
  limits: Pure fixtures only; no live compositor writes or adapter/runner integration checks.
---

# Layout

`solver/` (`zk-layout`) calculates a bounded integer secondary-axis allocation
from an ordered member list, baseline weights, optional emphasis and explicit
gap treatment. Taffy is derived calculation state, not desktop truth.

`longcat/` (`zk-longcat`) owns one group episode: immutable baseline, requested
effect, latest observation and per-command execution evidence. The application
bridges its native capture into pure inputs and serializes native mutations.
There are no sockets, timers, scheduler tasks or Kubeless dependencies here.

Native source and epoch, workspace, topology and ordered member identity must
survive the bridge; a positional column index alone must never serve as an
incarnation. The adapters remain responsible for deciding whether membership
evidence and native execution capabilities justify writing at all.

## Solver API

`zk_layout::allocate(Request { extent, gaps, members, emphasis })` returns an
`Allocation` or an explicit `Error`, including `Error::Infeasible`. Supply
positive finite baseline weights and integer `minimum`/optional `maximum` bounds.
An emphasis names the active member's **index in input order** and a share
strictly between zero and one. Siblings divide the remainder by baseline weight.
Without emphasis, all members divide by baseline weight.

- `[400,300,200]`, budget `900`, active first, share `.7` → `[630,162,108]`.
- `Gaps::AlreadyExcluded`: the span is already the sum of usable member space.
- `Gaps::Between { pixels: 10 }`: the span includes one gap between each pair;
  a span of `920` for three members has content budget `900`. Edge gaps and
  decorations must already be excluded. Never subtract the same gap twice.
- The complete span is floored once; output plus integer gaps conserves it.
  Largest remainders receive pixels first; exact ties use input order.
- Taffy performs actual zero-basis Flexbox allocation and cascading bound
  saturation with built-in rounding disabled. Grow factors are all at least one
  so saturation cannot trigger CSS's sub-unit-grow underfill behavior.
- Invalid/nonfinite/negative spans, invalid weights/bounds and impossible summed
  minima/maxima return no allocation. A zero content budget is valid only when
  all minima permit it. Spans above `2^24` and weight ratios unrepresentable by
  the f32 engine are rejected rather than pretending exact integer geometry.

## Episode API and observation bridge

Construct `zk_longcat::Episode::new(initial_observation, Policy)` once per
topology-bound group. `Observation` includes:

- `GroupKey { source: Source { id, epoch }, workspace, topology, axis }`;
- a monotonic local capture `version` and explicit available span/gap treatment;
- ordered `ObservedMember { id, extent, minimum, maximum }` rows;
- `Focus::Window(WindowKey { source, workspace, id })`, explicit `Focus::None`,
  or not-yet-known `Focus::Unknown`. Workspace and source prevent a similarly
  numbered column/window elsewhere from being mistaken for this group.

Use **observed** extents, never last requested targets, in each observation.
The episode separately exposes `baseline()`, `observed()`, `requested()`,
`effects()`, `phase()` and `suspension()`. The whole episode, decisions, plans,
observations and execution evidence implement `serde::Serialize`; input values
also support deserialization for replay. The baseline is immutable. Restoration
uses its conserving integer allocation, not a claim to recover hidden native
weights or exact topology that the compositor did not report.

Before any applied effect (including after a completely unsent initial attempt),
geometry attribution compares with the original observed extents, retaining their
fractional precision. Once an effect settles or restoration converges, attribution
compares with that plan's requested integer targets instead. Integer restoration
is a conserving representation, not pixel-exact recovery of fractional inputs.
The declared available span determines the allocation budget; it is not inferred
from member extents. To represent the original content span, the bridge must
supply their initial sum with the declared gap treatment, not a larger free span
that would proportionally reweight restoration.

### Dry-run call sequence

1. Bridge a fresh native group capture and construct an episode.
2. For each later capture call `episode.preview(observation, now_ms)` and print
   the returned serializable `Decision` beside `episode.baseline()` and the actual
   observation. `preview` clones for calculation and does **not** mutate the
   episode, record a request, claim command handling, or arm a deadline.
3. Continue previewing from that same baseline when focus moves inside the group.
   Ordinary unchanged observed geometry is not compared to the previous preview.
4. Discard/reacquire explicitly after a topology/source/context change. Preview
   returns the invalidation decision but, being non-mutating, does not itself
   install that invalidation. Do not acquire while a previous real episode may
   still own effects. Native capabilities can allow preview while refusing write.

Do not call `record_execution` with imaginary `Handled` replies to advance a
dry-run. A preview plan's IDs/deadlines are hypothetical, not executor work IDs.

### Actual execution and timer call sequence

1. Call `episode.decide(observation, now_ms)`. `Decision::Plan(plan)` records an
   unexecuted intent and enters `Applying` or `Restoring`; `Unsent` evidence is
   not a claimed compositor effect. Keep only one native sequence in flight.
2. Revalidate against a fresh capture with `plan.validate(&current)` before its
   first command. Between sequential commands use `validate_identity` (the old
   geometry can legitimately change after earlier commands). These are local
   admission checks, not remote compare-and-swap or atomic transactions.
3. Submit in target order; record each result using
   `record_command(plan.id, index, CommandOutcome)`. On focus/topology change,
   stop the unsent remainder. `Handled` is a native acknowledgment;
   `PossiblySent { message }` preserves uncertainty; `Rejected { message }`
   preserves native refusal; an unsent tail stays `Unsent`.
4. When the executor can no longer submit more commands, call
   `finish_execution(plan.id)`. Alternatively install a complete same-length
   report with `record_execution(plan.id, outcomes)`. Reports are checked before
   changing anything; final definitely-unsent tails cannot later become sent.
5. Continue observations independently during execution. Call `decide` after
   each relevant observation **and after completion evidence**, using a fresh
   current capture. No new plan is generated while a sequence remains unfinished.
   Matching later geometry establishes `Convergence::Compatible`, not causality.
6. Arm a real one-shot timer for `episode.next_reconsideration()`, in the same
   caller-owned monotonic millisecond clock used for `now_ms`. On expiry call
   `decide(latest_capture, now_ms)` even when the final event was long ago.
   Command handling alone does not clear the convergence deadline.

Focus changes while an effect is pending do not erase it. Once it converges,
reconsider the latest focus using the original baseline. A completely unsent
replacement can be reconsidered safely; it does not erase a prior settled
effect's restore duty. An identical capture after a completely unsent attempt
returns `Noop::NotSubmitted` instead of spinning on completion-driven retries;
a fresh observation or changed focus can prompt a new decision. Rejection,
deadline mismatch or settled unexpected geometry
conflicts the episode without automatic replay, learning or restoration.
`effects()` retains every sequence's evidence through focus changes and loss;
the runner bounds episode lifetime/diagnostic retention rather than this core
silently pruning it.

### Leaving, shutdown and loss

Known focus outside this source/workspace/member set, including explicit
`Focus::None`, requests baseline restoration **only** after the previous effect
has completed and observed geometry still matches it. Unknown focus suspends
ordinary decisions instead of being treated as blur. Changed member order,
membership, topology token, workspace, source/epoch, axis, available span, gaps or
bounds invalidates the episode; none inherits historical restore authority.

For shutdown call `request_restore(current_observation, now_ms)`, then drive the
same native evidence/observations/timer protocol. This latches restoration intent
so later callbacks do not start new emphasis. The application supplies its own
bounded shutdown deadline; it may stop without restoration rather than force a
stale write. `lose(reason)` immediately revokes restore authority and retains
evidence, even if late command results arrive. Recovery requires an explicit new
episode over fresh trustworthy state. No automatic rebaseline, resize learning,
inactive-drift repair, socket handling or scheduler host is implemented here.

## Checks

```sh
cargo nextest run -p zk-layout -p zk-longcat
cargo clippy -p zk-layout -p zk-longcat --all-targets -- -D warnings
cargo fmt -p zk-layout -p zk-longcat --check
```

The tests cover bounded allocation/rounding, baseline-relative focus changes,
known-none versus unknown focus, source/workspace identity, topology changes,
partial/uncertain/late execution evidence, silent-source timer expiry and
conditional shutdown restoration. This evidence does not establish native
compositor convergence, restore fidelity or transport cancellation behavior.

## Connections

- [Remake architecture](../design/remake/architecture0.gpt6a.md#longcat-policy-and-taffy)
  defines the baseline/requested/observed distinction implemented here.
- [Desktop domain](../desktop/README.md) supplies native observations and execution; it
  does not delegate topology discovery to Taffy.
- [Kubeless adoption](../design/remake/kubeless-asks0.gpt6a.md#taffy-composition-the-whole-path)
  places scheduling in the application, outside these pure cores. Upstream
  scheduler fairness is deferred; this implementation does not change Kubeless.
