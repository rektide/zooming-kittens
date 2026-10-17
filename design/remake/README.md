---
type: Index
title: Longcat remake reading map
description: Architecture and independent reuse research for the multi-crate niri and Umbriel remake.
resource: design/remake/README.md
status: draft
generated: { at: 2026-10-14, by: 'model:openai/gpt-6-astra#medium' }
sources:
  - resource: opencode://ses_e9a70a796ffelZmIZrb9Zxlb4f
    title: Connect the Longcat return effort and correct stale descriptions of implemented Taffy and workspace architecture
  - resource: opencode://ses_ec3316eefffejmDBXbZ28spBd7
    title: Commission, implementation integration, Kubeless return and navigation to the Umbriel-side native control requirements
  - resource: kubeless0.gpt61sm.md
    title: Independent reuse assessment integrated into the architecture proposal
---

# Longcat remake

## Implemented consumer

- [Bringing Longcat back](../longcat/README.md): current Umbriel window-sizing
  effort, independent backend features and related activity/entity research.

- [Current architecture](../../ARCHITECTURE.md): delivered domain crates and
  their ownership, distinct from the original proposal below.
- [Application runner](../../app/zk/src/runner/README.md): real native acquisition,
  direct scheduling, Taffy decisions, history-gated command admission and shutdown.
- [Kubeless implementation return](kubeless-asks0.gpt6a.md#implementation-return--integrated-consumer-2026-10-15):
  actual adopted APIs and what the consumer taught us; fairness remains deferred.

## Proposed direction

- [Umbriel-side layout-control needs](file:///home/rektide/src/umbriel-rs/design/layout-control/longcat-needs0.gpt6a.md):
  native topology, targeted/conditional sizing and restoration requirements;
  Rust-library opportunities and alternative compositions with Kubeless. This is
  the continuation home for the Umbriel protocol gap exposed by our consumer.
- [Architecture and landing sequence](architecture0.gpt6a.md): domain-grouped
  crates, observation/policy/execution contracts, Taffy's role, and the split
  between library improvements and compositor protocol work.
- [Kubeless adoption and upstream asks](kubeless-asks0.gpt6a.md): recommended
  first dependencies, fair scheduler work, an executable composition example,
  discovery improvements, and conditional settlement/capacity ideas. Includes
  the full path from native observations through Taffy to observed effects.

## Independent reuse research

- [Kubeless mechanisms](kubeless0.gpt61sm.md), Sol Medium: native inventories,
  before/after impact and timed reconsideration; direct versus transitive
  dependencies; optional scheduler fairness improvement. Selected existing
  mechanism tests: 159 passed, one skipped. The architecture proposal integrates
  the recommendations while keeping the detailed alternatives here.

## Predecessors and current owners

- [Taffy discovery](../../doc/discovery/taffy.md): earlier constraint-solver and
  full-tree proposals. The remake implemented the bounded group solver described
  in the [layout contract](../../layout/README.md).
- [Corrected Umbriel investigation](../umbriel/investigation0.glm53max.md):
  native protocol constraints, restore algebra, focus race and geometry identity.
- [umbriel-rs](file:///home/rektide/src/umbriel-rs/README.md): implemented Rust
  subscriptions, typed actions, state and capture facilities that supersede the
  earlier proposal to start a thin client.
- [Current architecture](../../ARCHITECTURE.md): implemented domain-grouped
  workspace and the remaining native capability boundaries.
