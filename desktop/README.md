---
type: ImplementationContract
title: Desktop observation baseline
description: Planning capture, bounded observation and native source ownership for the Longcat remake.
resource: /desktop/README.md
status: implementing
generated: { at: 2026-10-15, by: 'model:openai/gpt-6.1-sol#xhigh' }
sources:
  - resource: opencode://ses_ec314c407ffeXx45VDjb05vmol
    title: Desktop baseline implementation, API contract and scoped verification
  - resource: /design/remake/architecture0.gpt6a.md
    title: Approved desktop, policy and execution separation
  - resource: /design/remake/kubeless-asks0.gpt6a.md
    title: Selected native memory ownership and before/after impact
  - resource: /design/remake/kubeless0.gpt61sm.md
    title: Native inventory composition and explicit continuity limits
---

# Desktop observation baseline

Integration contract (being implemented in this session):

- `zk_desktop::DesktopCapture`: source/epoch `scope`, local `version`, `windows`,
  `workspaces`, `groups`, `active_window`, per-family `windows_status` and
  `workspaces_status`, `connected`, and declared `capabilities`.
- `WindowKey` / `WorkspaceKey` contain `Scope` plus `NativeId::{Number, Text}`.
  Group keys are positional hints, **not persistent group incarnations**. Ordered
  membership and `Evidence::{Reported, Inferred, Unavailable}` remain explicit.
- Both adapters return `(Source, zk_desktop::Observer)` from async `subscribe`.
  The observer provides `current()` and bounded `recv()` of shared
  `DesktopUpdate { before, after, impact, continuity }`. Lag is an explicit error;
  acquire `current()` and invalidate controller episodes before reconsidering.
- Keep the source owner alive. `Source::shutdown(self).await` joins actual cleanup;
  dropping it requests shutdown. Observation never waits for policy/output work.
- Niri: `subscribe(path, source_id, Options)` and `subscribe_env`;
  public `Reducer` supports pure native-event replay. The actuator is obtained
  separately from `Source::actuator()`, accepts by-ID `HeightRequest`, and reports
  `Unsent`, `Rejected`, `Handled`, or `PossiblySent`, never transaction/settlement.
- Umbriel: `subscribe(endpoint, source_id, Options)` composes existing client and
  `Inventory`; it is read-only. `Reducer` accepts native observations. No exact
  grouping or targeted actuation is invented from geometry.

Sibling development dependencies are explicit relative paths: from each desktop
crate, `../../../umbriel-rs` and `../../../kubeless` resolve under `~/src`.
No upstream files are modified. The app owns policy input conversion and scheduling.
