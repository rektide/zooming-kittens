---
type: Reference
title: Terminal remote control and focus zoom policy
description: Current crate ownership and the neutral focus input boundary after application extraction.
resource: terminal/README.md
status: implemented
generated: { at: 2026-10-15, by: 'model:openai/gpt-6.1-sol#medium' }
sources:
  - resource: opencode://ses_ec310f020ffe5d6kIV42ehdO2z
    title: Bounded application and terminal extraction, neutral focus bridge and scoped verification
  - resource: ../design/remake/architecture0.gpt6a.md#domain-grouped-workspace
    title: Approved domain grouping and dependency direction
---

# Terminal domains

- [`kitty`](kitty/src/lib.rs), package `zk-kitty`, owns remote control,
  process/socket/password discovery helpers, connection configuration and
  kitty.conf font parsing. `RegistryConfig` is a plain value type; the
  application converts its loaded configuration into it. The single baseline
  font-size `OnceLock` remains in `kitty::conf_parser`.
- [`zoomer`](zoomer/src/lib.rs), package `zk-zoomer`, owns `ZoomConfig`,
  `ZoomType` and `KittyResizer`. Its input is a stream of `FocusEvent::Focus`
  or `FocusEvent::Blur`, each carrying a `FocusTarget { id: String,
  pid: Option<i32> }`. IDs are opaque diagnostic labels, not niri IDs.
  Blur carries the original terminal target. Neither crate depends on niri.

The [application bridge](../app/zk/src/niri/mod.rs) translates the legacy niri
adapter's events; it discards create/destroy events. The new desktop adapters
are not wired through this extraction. File/environment/CLI loading stays in
the [app config module](../app/zk/src/config.rs), and manual font command
presentation remains in the app.

## Preserved limitations

This is extraction, not a zoom algorithm rewrite. Fractional `step_size` is
still truncated through the existing integer increment APIs. Multiplicative
steps that do not progress (including the default step `1`) can leave the
existing loop nonterminating. Command failures are still ignored by policy,
and state is tracked by PID. These behaviors need a separately tested policy
correction; moving them does not establish their correctness.

Verification used scoped `cargo check --all-targets --all-features` and
`cargo nextest run` for the app and the three extracted domain crates. No live
kitty mutation was performed.
