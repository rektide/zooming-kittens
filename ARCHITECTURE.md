---
type: Reference
title: Zooming-kittens architecture
description: Domain-grouped Rust workspace, observed desktop ownership, Taffy allocation and native effect evidence.
resource: ARCHITECTURE.md
status: maintained
generated: { at: 2026-10-15, by: 'model:openai/gpt-6-astra#high' }
sources:
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
| `desktop/umbriel` | `zk-umbriel` | Composition over the existing Umbriel client and inventory; read-only semantic observations |
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

Domain baselines are implemented and scoped-tested. Application integration is
being completed against these interfaces; the preimplementation design is not
evidence that every proposed capability is delivered.

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

Umbriel composes `umbriel-client` and `umbriel-state`. Seat activation comes from
`active`, not workspace-local remembered `focused`. Families arrive independently.
The current adapter leaves exact groups unavailable: the wire data does not
provide exact columns, axis/weights or a reliable by-ID extent operation. It
therefore exposes read-only windows, workspaces, activation and health; it does
not perform focus-switching resize workarounds.

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
