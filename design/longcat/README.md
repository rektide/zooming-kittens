---
type: Index
title: Bringing Longcat back
description: Window-sizing work, upstream research and separately tracked activity and entity integrations.
resource: design/longcat/README.md
status: active
updated: { at: 2026-11-11, by: 'model:openai/gpt-6.1-sol#high' }
generated: { at: 2026-10-22, by: 'model:openai/gpt-6-astra#medium' }
sources:
  - resource: opencode://ses_e331fea99ffeEcIAeJ9F8pM2aK
    title: Deliver stock observation and qualified internal-edge slice A and link its B–D continuation
  - resource: opencode://ses_e33541051ffeer2RcnuC2Rn4rj
    title: Add independent implementation-ready Taffy feedback-loop design and distinguish current working-line direction
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

## Deployment decision

The [working-line IPC handoff](handoff0.glm53m.md) supersedes the deployment
assumption in the historical candidate account below: stock IPC plus qualified
sizing is integrated in `working-20261110`, while the bespoke family is preserved
separately. The [feedback-loop design](feedback0.gpt6a.md) selects a concrete
implementation path on that surface, including user-guided preferences and
dispatchable assignments. Its implementation is commissioned: the
[stock boundary slice A](implementation-a0.gpt61s.md) is checkpointed; pure
feedback preferences, corrective execution and application integration (B–D)
remain. This is not yet an accepted or usable end-to-end stock controller.

The [composed deployment candidate](deployment-candidate0.gpt6a.md) is prepared
and verified, **not installed**. Candidate `96b118cf8b11` in
`~/src/umbriel-longcat-deploy` preserves composed base `1845885e` and narrowly ports
native allocation plus committed-size stream publication. Debug/release builds,
native checks and four-instance integrated headless stress passed. The record
includes side-by-side installation, config compatibility and rollback commands.

The composed base itself is 223 commits newer than the running compositor, so
the transition activates that pre-existing upgrade as well as Longcat. Installing
the candidate for the next login and choosing the graphical-session transition
remain user decisions. The activity service is already running independently.

## Current ground

- [Layout](../../layout/README.md) already implements constrained Taffy allocation,
  baseline-relative sibling weights and conserving integer rounding.
- [Runner](../../app/zk/src/runner/README.md) joins native streams, decisions,
  by-ID niri sizing and conditional restoration. Its integrated Umbriel path uses
  coherent native layout state and conditional whole-column allocation; older
  compositors retain read-only dry-run observation.
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

- [Taffy-driven feedback loop](feedback0.gpt6a.md) (2026-11-11): independently
  inspected stock observation and native resize math; selects internal-boundary
  correction, persistent user-guided focus shares and neutral sibling proportions,
  topology reconciliation, bounded constraint handling and four implementation
  assignments. Refines the handoff's target-versus-committed geometry claims and
  deliberately revises the existing permanent-conflict episode policy while
  retaining the Taffy solver and native/application ownership boundaries.
- [Umbriel IPC landscape handoff](handoff0.glm53m.md) (2026-11-10): the
  "port the bespoke family?" question resolved differently — upstream stock IPC
  now carries most of a loose Longcat's needs, and sizing-without-focus landed
  as the small `--window`-qualified `idtarget` extension on the umbriel working
  line. Maps all control surfaces (stock, idtarget, preserved bespoke family),
  the closable stock loop, the adapter evidence question, and client tooling
  for reimplementations.
- [Umbriel control and reusable tracking](umbriel-control-stream0.gpt6a.md): live
  deployment differs from both local composed and fetched upstream sources.
  Recommends a coherent layout snapshot and conditional whole-column weight
  assignment, retaining native baseline weights for restoration. Implementation
  proceeds in an isolated worktree from fetched upstream `5d657581`; carrying it
  into the composed desktop build and transitioning the session remain distinct
  delivery steps. The same research identifies reusable example-loop extraction
  and the missing structured activity service.
- [Recent journald work](../activity/journald-survey0.gpt6a.md): dated workspace
  survey identifies Cyclrs's exact-cursor bounded replay and Phyllited's
  binary-preserving parser/oracle. Neither is a complete live activity follower.
  Umbriel's existing **umbrs-activity-history** owns follow/replay work; the survey
  links that undertaking and distinguishes journal cursors from ZK export counters.

## Separate follow-up

**zk-font-revisit** records the user's wish to recover an earlier font-change
topic. Its precise intent remains to be recovered; current window-sizing work
does not select a font-policy change.

## Delivered related integration

- [Stock observation and qualified command boundary](implementation-a0.gpt61s.md)
  (2026-11-11): upstream typed layout/output fields and independent freshness,
  reported ordered groups, inferred two-axis geometry, presentation admission,
  and an epoch-bound qualified internal-edge actuator. No focus dance or bare
  fallback. Native parser, existing headless by-ID checks and scoped Rust suites
  pass. The receipt names the exact DTOs, commits and remaining B–D work; the
  running desktop, service and default application control policy are unchanged.
- [Reusable Umbriel tracking](file:///home/rektide/src/umbriel-rs/crates/tracking/README.md)
  exposes typed observation installation and Inventory-owned freshness; both
  examples reuse it. Immediate captures remain distinct from delayed activity
  acceptance. The [activity application](file:///home/rektide/src/umbriel-rs/crates/activity/README.md)
  is installed and running as `umbriel-activity.service`, emitting field-first
  journal records. [Deployment evidence](file:///home/rektide/src/umbriel-rs/design/activity/runtime0.gpt6a.md)
  records 78 scoped tests, native journal readback, bounded-output cancellation and
  clean service stop/start without restarting the compositor. History/replay and
  notify readiness retain their existing upstream tickets.
- [Native conditional column allocation](https://github.com/rektide/umbriel/blob/96b118cf/docs/user/layout-control.md)
  is implemented on the isolated upstream-based compositor worktree, with typed
  Rust bindings in `umbriel-rs`. Query/event `layout-state` exposes a coherent
  allocation basis; `layout-allocate` validates and assigns a complete weight
  vector without focus/reveal changes. Native build, pure/headless/stress checks
  and 160 Rust tests plus five doctests passed. ZK consumer commits `fa3ca327` and
  `3e5c9b57` add Taffy/native translation, raw-weight restoration and whole-group
  receipts. 107 scoped tests and real headless integration passed, including four
  concurrent instances. **zk-longcat-return-native-control** is closed;
  **zk-longcat-return-native-deploy** tracks composed-build preparation and the
  separate desktop transition.
- Explicit Longcat service generation is committed in `83f7ade6`; see
  [service usage](../../README.md#running-as-a-systemd-service).
  **zk-longcat-return-service** is closed. Generated font/niri/Umbriel units passed
  systemd verification; focused tests and Clippy passed with isolated dependency
  alignment. No ZK service was installed or started.
- Independent application backend features are committed in `e6df3fb7`; see
  [build examples](../../README.md#select-backends). Eight backend combinations,
  `export-facet` alone and all-features initially passed in an isolated setup.
  Dependency alignment and all ten real-checkout configurations now pass;
  **zk-backend-features** is closed.

- [Optional Kopacetic active cursor](file:///home/rektide/src/umbriel-rs/integration/kopacetic-active/README.md)
  materializes one source-qualified live entity, with immediate activation and
  same-window metadata updates. Loss and shutdown clear active state. Its isolated
  workspace and explicit feature keep it outside normal Umbriel and ZK paths.
  Four tests passed, including scripted reconnect/cancellation; live deployment
  was not performed. Native work items **umbrs-kopacetic-cursor** and
  **kop-umbriel-cursor** are closed.
- [Cursor design and verification](file:///home/rektide/src/umbriel-rs/design/kopacetic/active-cursor0.gpt6a.md)
  records the choice of a stable cursor identity over per-window entities and the
  existing root-workspace Kubeless version mismatch found during verification.
