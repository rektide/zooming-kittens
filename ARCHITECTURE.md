---
type: Reference
title: Zooming-kittens architecture
description: Domain-grouped Rust workspace, observed desktop ownership, Taffy allocation and native effect evidence.
resource: ARCHITECTURE.md
status: maintained
generated: { at: 2026-10-15, by: 'model:openai/gpt-6-astra#high' }
sources:
  - resource: opencode://ses_e9a70a796ffelZmIZrb9Zxlb4f
    title: Independent backend composition and application-owned execution boundary
  - resource: opencode://ses_ec3316eefffejmDBXbZ28spBd7
    title: Approved remake, parallel baseline implementation, architectural review and application integration
  - resource: desktop/README.md
    title: Implemented native observation, health, identity and command transmission contracts
  - resource: layout/README.md
    title: Implemented Taffy allocation and pure Longcat episode contracts
  - resource: terminal/README.md
    title: Extracted remote-control and focus-policy ownership
  - resource: design/remake/architecture0.gpt6a.md
    title: Preimplementation architectural reasoning and staged destination
  - resource: design/remake/kubeless-asks0.gpt6a.md
    title: Selected Kubeless mechanisms and deferred upstream agenda
---

# Architecture

## Workspace and ownership

The root is a virtual Cargo workspace. `app/zk` is its default member and builds
the `zk` executable. Library crates live beneath their owning domains:

| Path | Package | Responsibility |
|---|---|---|
| `desktop/model` | `zk-desktop` | Scoped IDs, semantic captures, readiness, geometry evidence, impact and bounded observation |
| `desktop/niri` | `zk-niri` | Native event reduction, asynchronous IPC source and by-ID height commands |
| `desktop/umbriel` | `zk-umbriel` | Reusable tracking, coherent native layout projection and conditional whole-column allocation |
| `layout/solver` | `zk-layout` | Taffy-based constrained secondary-axis allocation and conserving integer rounding |
| `layout/longcat` | `zk-longcat` | Pure baseline/intent/observation episodes, decisions and retained effect evidence |
| `terminal/kitty` | `zk-kitty` | Kitty remote control, connection/process discovery and configuration parsing |
| `terminal/zoomer` | `zk-zoomer` | Font policy over neutral focus/blur inputs |
| `inspection/export` | `zk-export` | Native niri JSON/JSONL records, filters, writers and export command |
| `app/zk` | `zooming-kittens` | CLI/configuration, source/policy bridges, scheduling and execution composition |

The application joins these responsibilities. The solver knows no sockets;
terminal libraries know no niri types. Export remains explicitly native niri
data, rather than silently translating its established format into a neutral
desktop schema. The `export-facet` application feature forwards to the export
crate's existing optional scaffold.

Application features `niri`, `umbriel`, and `kitty` independently select their
optional backend dependencies; all are enabled by default. `export-facet` also
enables `niri`. Font zoom composes niri observation with Kitty control; direct
font commands require only Kitty. The runner owns its execution request/receipt
types and converts them at the native boundary, so Umbriel-only builds do not
depend on the niri adapter. Feature selection applies to the application graph;
`--workspace` checks still select the individual backend library packages.

Development dependencies use the sibling `../umbriel-rs` and `../kubeless`
checkouts. The application does not require the Umbriel Polars packages.

## Three different states

1. **Observed desktop:** immutable captures of native facts, including freshness,
   source epoch and geometry meanings. A coherent local capture is not an atomic
   compositor-wide transaction.
2. **Longcat episode:** the baseline, current intent, native command outcomes and
   subsequent observed compatibility. Baseline state is not rewritten merely
   because our own expansion changed the geometry.
3. **Solver tree:** derived Taffy input/output for an allocation calculation.
   A Taffy node is not a compositor identity or proof of an applied resize.

The intended composed path is:

```text
native event / family snapshot
  → native observed state and before/after capture
  → affected regions and timed reconsideration
  → current facts + Longcat episode
  → Taffy allocation
  → native command preparation and submission
  → retained reply evidence + later geometry observations
```

The application now composes these domains through the
[Longcat runner](app/zk/src/runner/README.md). Default font zoom and `zoomer` use
the new native niri observation source; the legacy focus registry and the old
Longcat implementation have been removed. Compatible Umbriel servers supply
coherent layout observations and conditional whole-column mutation; older servers
retain read-only observation under dry-run.

## Desktop sources

Both native adapters return a source owner and bounded observer. Keep the source
alive while using the observer; explicitly await `shutdown()` to join cleanup.
The native receive task continues while policy, terminal commands or diagnostics
wait. Lag and source termination are explicit, not known-empty desktops.

Identities include backend/source and epoch. Native IDs retain number versus
text representation. Group positions are hints; a control episode additionally
checks ordered membership and context before carrying a baseline forward.

Niri installs complete windows/workspaces and incremental focus, lifecycle and
layout changes. Its asynchronous Unix source replaces the old blocking event
reader. The height actuator sends explicit window IDs and distinguishes unsent,
rejected, handled and possibly-sent outcomes. No automatic action replay or
multiwindow transaction is implied.

Umbriel composes `umbriel-client` and `umbriel-tracking`, retaining Inventory as
the row owner. A read-only capability probe selects the version-1 `layout-state`
extension when available. That coherent family supplies control focus, topology,
constraints, native weights and target/committed geometry; independently arriving
windows/workspaces are not mixed into its authority. Older servers expose legacy
read-only inventory with unavailable grouping. Immediate control is distinct from
the activity service's five-second accepted-identity policy.

The Umbriel executor submits one conditional whole-column request with one native
receipt. Content budgets and bounds subtract titlebar decorations; rounded edge
gaps are preserved. Original raw native weights and native restoration targets
are retained separately from the Taffy baseline allocation. Subsequent writes
compare the last accepted complete native basis, so even same-pixel external
weight changes revoke control. This is compare-current-state, not an ABA-detecting
revision. Lost or uncertain transmission does not authorize retries or rebasing.

See [desktop contracts](desktop/README.md) for source lifetime, frame/delivery
limits, epochs, capabilities and geometry semantics.

## Layout and control

`zk-layout::allocate` uses real zero-basis Taffy Flexbox allocation. Inputs are an
ordered set of baseline weights, bounds, a content span/gap convention and an
optional active share. It handles cascading bounds and rejects infeasible totals.
Its separate rounding pass conserves the integer budget with deterministic ties.
For `[400,300,200]` at 70% first-window emphasis, the result is `[630,162,108]`.

`zk-longcat::Episode` is clock-injected and transport-free. `preview` produces a
dry-run decision without recording imaginary writes. `decide` records an intent;
the runner supplies actual per-command outcomes and fresh observations. Handling
a command is distinct from observing a compatible result. The runner must arm
`next_reconsideration()` even if no further event arrives.

Conditional restoration requires still-valid identities/context and compatible
effects. Source loss, changed topology and conflicting geometry invalidate that
authority. Baseline restoration represents observed pixel allocation, not hidden
native layout weights. Cancelling a waiter cannot undo a possibly-sent request.

See [layout contracts](layout/README.md) for the complete runner call sequence,
deadline behavior, unsent-attempt suppression and shutdown semantics.

## Kubeless responsibilities

- Niri uses `view-memory`; Umbriel retains its existing native inventory owner.
- Actual before/after evidence supports old and new region impact.
- The application selects direct `work-schedule`, with bounded input processing,
  to reconsider current facts rather than replay stale plans.
- Pure solver and episode crates do not depend on Kubeless or Tokio.

Upstream stream-scheduler fairness is **deferred by human direction**. Build and
verify this consumer before lifting a reusable example. The
[Kubeless handoff](design/remake/kubeless-asks0.gpt6a.md) is the record to update
with actual integration findings, not a mandate to adopt the full controller host.

## Configuration and verification

The application owns Figment loading and CLI overrides; domain crates receive
plain policy/configuration values. The existing configuration directory remains
`$XDG_CONFIG_HOME/kitty-focus-tracker` and the environment prefix remains `ZK_`.
Package extraction does not itself migrate persisted configuration.

Build the application with `cargo build -p zooming-kittens`; install with
`cargo install --path app/zk`. Scope checks by package while working on one domain.
The workspace-level test, feature and Clippy checks establish composition after
integration. Scripted compositor peers verify protocol behavior without changing
the live desktop; live convergence is a separate verification claim.

The latest integration passed 107 scoped tests, strict Clippy, all ten application
feature configurations and service-generation tests against real sibling checkouts.
Real ZK/native headless checks cover emphasis, switching, native-weight restoration
and external edits, including concurrent runs with decorations and gaps. The
running desktop still requires the composed native build and session transition;
headless verification is not a claim of deployed desktop sizing.
