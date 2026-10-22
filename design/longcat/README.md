---
type: Index
title: Bringing Longcat back
description: Window-sizing work, upstream research and separately tracked activity and entity integrations.
resource: design/longcat/README.md
status: active
generated: { at: 2026-10-22, by: 'model:openai/gpt-6-astra#medium' }
sources:
  - resource: opencode://ses_e9a70a796ffelZmIZrb9Zxlb4f
    title: Commission Longcat return, independent backend features, journald research and optional upstream Kopacetic integration
  - resource: ../../layout/README.md
    title: Existing Taffy allocation and pure Longcat episodes
  - resource: ../../app/zk/src/runner/README.md
    title: Existing execution, observation and restoration contract
  - resource: ../remake/README.md
    title: Previous implementation and upstream control requirements
---

# Bringing Longcat back

Work anchor: **zk-longcat-return**. The immediate outcome is usable focus-driven
window sizing on the Umbriel desktop. The existing Taffy solver and Longcat
episodes are the starting implementation.

## Current ground

- [Layout](../../layout/README.md) already implements constrained Taffy allocation,
  baseline-relative sibling weights and conserving integer rounding.
- [Runner](../../app/zk/src/runner/README.md) joins native streams, decisions,
  by-ID niri sizing and conditional restoration. Its Umbriel path is currently
  read-only; exact grouping and targeted sizing need upstream investigation.
- [Remake map](../remake/README.md) preserves the earlier research and links to
  Umbriel's native layout-control requirements. That work supplies the baseline
  for checking current upstream capabilities.

## Commissioned directions

1. **Independent backends:** Cargo features for `umbriel`, `niri` and `kitty`,
   allowing a window-sizing installation without the terminal-font backend.
2. **Umbriel library and service composition:** reuse library-owned stream state
   and investigate reusable example code; establish the running service's role
   and the native topology/control contract needed by Longcat.
3. **Journald activity:** search the last four months of workspace work for useful
   implementations and interests. Preserve meaningful link-outs even when reuse
   is not worthwhile. Journal activity and desktop focus need distinct semantics.
4. **Optional upstream Kopacetic integration:** investigate materialized entities
   versus one live active-object cursor in Umbriel. Keep this opt-in and outside
   primary runtime paths; use the findings to inform later ZK entity modeling.

Independent research precedes integration choices. Track upstream changes with
their native owners and connect those tickets to this effort.

## Research returns

- [Recent journald work](../activity/journald-survey0.gpt6a.md): dated workspace
  survey identifies Cyclrs's exact-cursor bounded replay and Phyllited's
  binary-preserving parser/oracle. Neither is a complete live activity follower.
  Umbriel's existing **umbrs-activity-history** owns follow/replay work; the survey
  links that undertaking and distinguishes journal cursors from ZK export counters.

## Separate follow-up

**zk-font-revisit** records the user's wish to recover an earlier font-change
topic. Its precise intent remains to be recovered; current window-sizing work
does not select a font-policy change.
