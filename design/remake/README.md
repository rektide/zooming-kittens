---
type: Index
title: Longcat remake reading map
description: Architecture and independent reuse research for the multi-crate niri and Umbriel remake.
resource: design/remake/README.md
status: draft
generated: { at: 2026-10-14, by: 'model:openai/gpt-6-astra#medium' }
sources:
  - resource: opencode://ses_ec3316eefffejmDBXbZ28spBd7
    title: Commission, source-grounded architecture and research integration
  - resource: kubeless0.gpt61sm.md
    title: Independent reuse assessment integrated into the architecture proposal
---

# Longcat remake

## Proposed direction

- [Architecture and landing sequence](architecture0.gpt6a.md): domain-grouped
  crates, observation/policy/execution contracts, Taffy's role, and the split
  between library improvements and compositor protocol work.

## Independent reuse research

- [Kubeless mechanisms](kubeless0.gpt61sm.md), Sol Medium: native inventories,
  before/after impact and timed reconsideration; direct versus transitive
  dependencies; optional scheduler fairness improvement. Selected existing
  mechanism tests: 159 passed, one skipped. The architecture proposal integrates
  the recommendations while keeping the detailed alternatives here.

## Predecessors and current owners

- [Taffy discovery](../../doc/discovery/taffy.md): earlier constraint-solver and
  full-tree proposals. Taffy is not integrated in the current executable.
- [Corrected Umbriel investigation](../umbriel/investigation0.glm53max.md):
  native protocol constraints, restore algebra, focus race and geometry identity.
- [umbriel-rs](file:///home/rektide/src/umbriel-rs/README.md): implemented Rust
  subscriptions, typed actions, state and capture facilities that supersede the
  earlier proposal to start a thin client.
- [Existing architecture](../../ARCHITECTURE.md): historical single-package
  organization; current source and the remake proposal distinguish implemented
  behavior from intended stream capabilities.
