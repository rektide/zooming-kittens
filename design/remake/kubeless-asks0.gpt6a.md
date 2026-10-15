---
type: Design
title: Kubeless adoption and upstream asks for Longcat
description: Concrete selections, upstream improvements and proof cases for composing Kubeless with Taffy and native compositor clients.
resource: design/remake/kubeless-asks0.gpt6a.md
status: proposed
generated: { at: 2026-10-15, by: 'model:openai/gpt-6-astra#high' }
sources:
  - resource: opencode://ses_ec3316eefffejmDBXbZ28spBd7
    title: Human invitation to improve young sibling libraries, adoption decisions, Taffy composition, and explicit deferral of upstream scheduler fairness during the consumer build
  - resource: kubeless0.gpt61sm.md
    title: Independent source-grounded reuse assessment and prior scoped test evidence
  - resource: architecture0.gpt6a.md
    title: Proposed observation, policy, solver and execution boundaries
  - resource: file:///home/rektide/src/kubeless/work/schedule/src/lib.rs
    title: Rechecked unbudgeted Scheduler stream intake before readiness servicing
  - resource: file:///home/rektide/src/kubeless/view/memory/src/lib.rs
    title: Atomic local batches, captures and commit evidence without native lifecycle policy
  - resource: file:///home/rektide/src/kubeless/control/settlement/src/lib.rs
    title: Acknowledgment-preserving composition and its explicit future-cancellation limitation
verification:
  method: Source inspection, scoped implementation verification reports, independent layout review, and read-only live Umbriel application acquisition.
  limits: Upstream asks remain proposals; aggregate workspace checks are recorded separately when complete. No live mutation is claimed.
---

# Kubeless: what we should use and what we should improve

## Implementation return — integrated consumer, 2026-10-15

The first domain implementations now provide concrete adoption evidence:

| Implemented owner | Actual entrance | Result |
|---|---|---|
| [`zk-niri`](../../desktop/niri/src/reducer.rs) | `Replacement`, `Writer::replace_changes`, `Writer::apply_changes` | Native window census/delta events commit through the memory leaf; malformed native input is rejected and source health invalidated by the adapter. |
| [`zk-umbriel`](../../desktop/umbriel/src/reducer.rs) | Existing `Inventory::install_windows_changes` | Reuses the native state owner and its continuity evidence; no duplicate reflector introduced. |
| Both native reducers | `view-impact::affected_roots_with` | Projects actual before/after receipts into former/current affected workspaces, alongside semantic capture impact. |
| [`zk-layout` and `zk-longcat`](../../layout/README.md) | No Kubeless dependencies | Taffy and clock-injected episode decisions remain independent of state-storage and scheduling mechanics. |
| [Application runner](../../app/zk/src/runner/README.md) | Direct `schedule`, `ScheduleRequest`, `ReadyWork::poll_ready` | One coalescing key for reconsidering the bounded episode set; command receipts, capture changes and convergence deadlines trigger current-state decisions. |

The adapter author reported 25 passing tests with strict Clippy/formatting;
the layout implementation and review correction passed 46 tests with Clippy and
formatting. The application author reported 31 all-feature tests, including
scripted native acquisition, conditional restoration, timer-only reconsideration,
partial effects, configuration precedence and discontinuity catch-up.

Parent live verification ran `zk longcat --backend umbriel --dry-run --limit 1`
against the available native socket: 31 windows, eight workspaces, explicit
unsupported grouping, clean shutdown and no mutations. Startup/unready records
did not consume the ready-decision limit. A separate invocation with write mode
and a nonexistent socket was rejected for unsupported capability before connecting.

What this already teaches us:

- Actual native receipts are a usable reuse boundary. No extra reflection host
  was necessary for either adapter.
- Impact must account for native window changes **and** semantic changes such
  as family readiness and grouping eligibility. A membership-only product would
  not capture the entire planning input.
- Invalid native IDs/rows and unknown siblings remain adapter responsibilities;
  atomic map replacement does not validate a compositor's layout semantics.
- Umbriel's absent exact grouping remains absent in the semantic view. The
  libraries compose successfully without pretending the consumer can now resize
  a topology it cannot observe.

### What the actual runner taught us

**History-gated admission is the most important composition finding.** A latest
healthy capture can be newer than a queued discontinuity. Reading `current()` and
checking one queued event is insufficient: it can carry an old episode across an
unprocessed gap. The runner now catches up at most eight updates per turn,
examines every continuity marker, and yields/rearms if more remain. No policy
admission jumps past that history. Before a write it checks again for queued
updates and current-version mismatch. Native exact-basis validation additionally
narrows, but does not eliminate, the remote race.

This belongs in the proposed upstream composition example. It is not evidence
that Kubeless needs a new universal stream abstraction: native ownership already
exposes the facts, and the controller must decide when continuity matters.

**One direct scheduling key was sufficient.** The first runner serializes native
effects and reevaluates a bounded episode set from current observations. Native
impact remains useful evidence; the app does not yet use it to run independent
per-region queues. Avoid describing this implementation as a parallel keyed
controller. Per-region scheduling can be evaluated if the current scan becomes
material, rather than adopted for symmetry with the primitives.

**Direct scheduling solved the immediate liveness need without upstream work.**
The runner prioritizes command completion/shutdown and bounds history intake.
Episode deadlines fire even after the final event. This is application-composed
bounded intake, not a change to `Scheduler` or a general fairness guarantee.
Scheduler fairness remains explicitly deferred.

**Receipt retention belongs to the episode owner.** The runner has one owned
command future, records each native result and retains uncertain partial effects.
At bounded shutdown it records an outstanding request conservatively as possibly
sent before dropping the waiter. Existing settlement helpers, Executor and the
retained-action host were not required. Episode/member/effect limits stop the
runner rather than silently pruning evidence; see the runner contract for exact
limits. This preserves the conditional settlement-reducer idea as a later fit
test, not an immediate upstream ask.

**The first diagnostic consumer did not need another publication layer.** It uses
a bounded JSONL queue with size/write deadlines. Independent lagging publication
remains a good candidate when there are multiple consumers; it is not installed
between native subscriptions and their inventory owners.

**Taffy integration exposed a domain distinction, not a Kubeless defect.**
Baseline content budgets are retained through partial writes; current per-window
geometry remains observed. Independent review also found the initial fractional
baseline must be compared with its original observation, not an integer restore
target never applied. The core was corrected and regression-tested.

### Updated upstream disposition

1. Fair stream Scheduler: **deferred**, unchanged by these results.
2. Non-resumable composition example: **worth lifting next**, grounded in actual
   receipts, history-gated admission, timer-only progress, and unknown effects.
   Keep compositor geometry/Longcat behavior in this repository.
3. Entrance documentation: **still worthwhile**; the successful low-level path
   demonstrates why consumers need not start with a full controller host.
4. Settlement reducer / keyed capacity: **no new primitive justified yet**.
   Retain the ideas and revisit against a demonstrated consumer pressure.

No Kubeless or umbriel-rs upstream files were changed in this implementation.
The executable consumer and its tests are now the evidence from which to shape
the later lift. Do not turn that recommendation into authorization to implement
the deferred scheduler change.

## Decision

**Yes, use Kubeless.** Its useful contribution is the machinery between a native
observation and a fresh decision: consistent local state, before/after evidence,
affected regions and timed work. Taffy computes allocation. Longcat decides
when to emphasize, learn, suspend and restore. Native clients own their sockets
and protocol facts.

Treat Kubeless as a library we can improve through this consumer, not a frozen
framework whose current limitations dictate our architecture. Equally, adopt
what already works before creating new mechanisms. The strongest first upstream
change identified is scheduler fairness, now explicitly deferred by the human.
The strongest design contribution is a real
non-resumable, observation-driven control example.

This sharpens the earlier assessment's optional choices into the following
recommended build plan; it is not a claim that these dependencies are installed.

## Adopt in the first implementation

| Piece | Owner and purpose | Concrete payoff |
|---|---|---|
| `view-memory` | `desktop/niri`; already beneath `umbriel-state` for Umbriel | Apply a native event batch once and plan from a coherent local capture instead of querying repeatedly during one decision. |
| Native change receipts + `view-impact` | Adapter projection and app runner | A move A→B invalidates both regions; deletion retains the former region; equal full-snapshot rows need not cause more work. |
| Direct `work-schedule` | App runner | Merge dirty reasons by region, service actual timers and reconsider from the latest state. A resize burst ending in silence still gets handled. |

For Umbriel, consume `Inventory::install_windows_changes`; do not install its
same native rows into another reflector. For niri, commit all changes from a
native layout event as one batch. The shared boundary is a planning capture,
not identical native inventory implementations.

Scheduling payloads describe **what needs reconsidering**, not a saved list of
commands. On admission, acquire a current capture and run Longcat again. This
is how focus A→B→C becomes a decision about C rather than delayed writes for A
and B. Pending possibly-sent effects remain recorded separately; superseding a
decision cannot erase them.

Keys have different jobs: native object identity, affected-region identity,
queued-work identity and effect exclusion. Scope native identities by backend
and source epoch. A positional column number is not an incarnation. Initially
serialize mutations per backend session; resize effects include siblings and
can span more than one region.

Use `schedule`'s timing reexports; a separate direct `schedule-policy` dependency
is only needed when directly importing that crate for a clock-driven consumer.
Workspace dependencies should document sibling development paths or pin reviewed
revisions. The shared unpublished `0.1.0` number is not an interoperability promise.

## Adopt when the consumer exists

### Independent status publication: a genuine near-term fit

When the runner has more than one status consumer, use the existing independent
lagging publication policy. A UI, JSON writer or diagnostic client must not
block desktop observation. Publish shared immutable captures/decision records;
slow consumers get explicit loss and reacquire a current view.

Do not insert this beneath Umbriel's already independently driven subscription.
This is publication of **our derived state**, after the native owner. Its item
capacity is not a byte limit; bound retained capture size and consumer queues
at the application boundary. Prefer separate current-state and historical trace
contracts rather than implying a latest-state subscription is an audit log.

### Settlement composition: worth a focused fit test, not mandatory layering

`control-settlement` already preserves a native acknowledgment through ordinary
observation failures, distinguishes native handling from observed settlement,
and supplies `settle_acknowledged` without a submission callback. Those are
directly relevant to resize control.

However, its future owns the acknowledgment; cancelling that future drops the
receipt. Longcat needs a lasting episode record across focus changes, timeouts
and superseded plans. Keep the authoritative command evidence in the runner.
Test the existing helper against that ownership model before adopting it.

The native `ok` reply need not contain the target: a caller's prepared plan can
provide it. Settlement still needs window incarnation, current source health,
membership and a geometry/weight predicate. A terminal mismatch can be ambiguous;
matching geometry proves compatibility, not causality.

### Execution admission: useful when concurrency earns its cost

`work-execute` and `ReadyWork` provide capacity and exclusion-key admission.
Use them if independent backend sessions or proven-independent effect scopes
justify parallel work. One serialized effect owner is sufficient initially.
Per-window concurrency would be incorrect for sibling-redistributing resize.

## Upstream ask 1 — fair stream Scheduler

**Disposition: deferred in Kubeless by human direction, 2026-10-15.**

Build the Longcat consumer using bounded caller intake into direct `Schedule`.
Do not expand this implementation into upstream scheduler work. Preserve the
proposal below for a later Kubeless decision; its difficulty and scope do not
need to be settled to build this application.

`Scheduler::poll_ready` currently loops over input until Pending/EOF, then polls
the schedule. A perpetually ready producer can prevent due work from being
offered. This is a control-loop liveness issue, not just an optimization.

Propose a finite per-poll ingress budget for the stream adapter. At budget
exhaustion, service schedule readiness, arrange another wake as needed and yield
cooperatively. Let the direct `Schedule` remain caller-driven. Consider a finite
default with explicit greedy opt-in; Kubeless is young, so preserving accidental
starvation should not dictate the default. Exact API/default count belongs in
the upstream design, not in a Longcat-specific constant.

Acceptance cases:

1. Permanently ready input and an already-due key: due work is offered after
   bounded input processing, provided the consumer is willing to admit it.
2. More than one key: useful progress continues under repeated updates.
3. Rejected admission retains accepted work and merged payloads; yielding does
   not silently consume or duplicate it.
4. EOF follows the configured close policy; ordinary finite streams preserve
   existing timing and merge semantics.
5. Budget exhaustion has correct wake behavior, without busy spinning when no
   input or deadline can progress.

Until this lands, use bounded caller intake into direct `Schedule`. Executor's
start budget is a different bound and does not repair greedy producer polling.

## Upstream ask 2 — a non-resumable control composition example

**Priority: build from Longcat's first trace, then lift the reusable example.**

Show a fake native source emitting complete snapshots and incremental events,
explicit epochs/gaps, actual committed receipts, before/after impact, timed
reconsideration and one acknowledged write followed by observed settlement.

The example should answer these practical questions in executable code:

- Where is the source owner kept alive while commands wait?
- How is old-region impact retained when a window moves or disappears?
- How is a due decision recomputed from current state instead of replaying an
  obsolete queued plan?
- How does a final resize event start a timer that fires without another event?
- How does an unknown write outcome stop blind retry while fresh observation
  continues?
- How do loss and slow optional subscribers affect readiness differently?

The generic upstream example can use fake resources and effects. Compositor
dependencies, grouping semantics and Longcat proportions stay here. This is
valuable API feedback: awkward joins exposed by a real consumer are better
grounds for library changes than designing another abstract host up front.

## Upstream ask 3 — make existing entrances discoverable

**Priority: small documentation improvement accompanying the example.**

Provide a decision table for direct `Schedule` versus stream `Scheduler`,
blocking versus independent-lagging publication, native state owners versus
reflection, and acknowledgment versus settlement. Document which capacities
bound items, bytes, keys, admitted effects or starts per poll.

In particular, independent lagging publication already exists. Sol found the
root description emphasized shared blocking backpressure; fix the entrance
instead of building a duplicate. Link each recipe to its owning leaf crates
and name the facts that still belong to the application.

## Opportunities to pursue only after a concrete fit test

### Cancellation-independent settlement reducer

If the existing settlement helper forces awkward bookkeeping, consider a small
clock-injected reducer driven by acknowledgment, observation, timeout and loss
facts, with receipts owned outside disposable wait futures. Its contribution
would be reusable state transitions, not new guarantees about remote effects.

First check existing retained-action facilities for this responsibility. Reuse
a fitting leaf; do not create a competing ledger merely because the full host
is larger than Longcat needs. The proof case is: after successful native reply,
a focus change cancels the old wait but the acknowledgment remains inspectable
and cannot trigger accidental resubmission. This is a conditional idea, not a
demonstrated missing API.

### Explicit keyed capacity/refusal

Direct schedule cardinality is unbounded. A desktop runner can initially bound
admission using its live regions and explicit retirement of old-epoch work.
If that creates repeated difficult code, propose a generic new-key capacity
policy that returns refused owned work while permitting merges into existing
keys. Never evict silently or call retained restore obligations disposable.
Measure the real retained work first; no new generic capacity subsystem is
required simply because the primitive is unbounded.

Multi-key effect reservation and the full retained-action host remain later
options. Broad serialization is a clear initial contract, and a Longcat
episode is not automatically a general durable workflow.

## Taffy composition: the whole path

```text
niri events / Umbriel snapshots
  → native state owner, coherent local capture and change receipt
  → layout-relevant before/after impact
  → keyed reconsideration, with explicit timing
  → current capture + Longcat episode + selected emphasis policy
  → Taffy allocation for the affected group
  → semantic plan and backend-specific command preparation
  → native submission evidence + subsequent observed convergence
  → episode update and derived status publication
```

Taffy receives known content extent, ordered members, baseline-relative weights
and explicit constraints. It returns desired allocation. It is neither the
inventory nor a record of what the compositor actually did. Compositor gaps,
content/tile distinctions, integer rounding and infeasible constraints need
explicit adapter/solver contracts. The architecture's `[400,300,200]` to
`[630,162,108]` example is a fixture target, not proof of a particular Taffy
configuration.

This composition is a substantial reuse opportunity. Kubeless eliminates
repeated state/change/timer mechanics; Taffy eliminates bespoke flex allocation;
the code we write can concentrate on Longcat's actual policy and native fidelity.

## Readiness to build

There is no outstanding Kubeless research question that should delay the first
read-only-to-dry-run implementation. Select memory, impact and direct scheduling.
Build the real consumer first; lift the composition example after learning from
it. Scheduler fairness remains deferred. Use the first real trace to decide
publication and settlement adoption.

### Required return after implementation

Update this handoff after the first integrated implementation and verification:
record the exact adopted APIs, awkward integration points, local adaptations,
checks and observed limits. Revise each ask using that evidence. Identify what
can actually be lifted into Kubeless and what is application policy. Keep
scheduler fairness deferred unless the human changes that decision. This is
an implementation closeout obligation, not a claim that the learning already
exists.

The independent blocker to full Umbriel control parity remains the compositor
contract: explicit resize targets and sufficiently exact layout/restore state.
No Kubeless or Taffy improvement can manufacture those native facts.

## Connections

- [Architecture](architecture0.gpt6a.md) owns crate boundaries and the staged
  build; this document sharpens its Kubeless adoption and upstream agenda.
- [Sol's research](kubeless0.gpt61sm.md) preserves API alternatives, source
  references and the earlier 159-passed/one-skipped test account. Those were
  existing mechanism tests, not verification of this proposed composition.
- [Reading map](README.md) connects this agenda to Taffy and Umbriel research.
