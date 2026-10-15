---
type: Reference
title: Terminal remote control and focus zoom policy
description: Current crate ownership, the neutral focus input boundary, and the corrected zoom transition semantics.
resource: terminal/README.md
status: implemented
generated: { at: 2026-10-15, by: 'model:zai-coding-plan/glm-5.3#high' }
sources:
  - resource: opencode://ses_ec310f020ffe5d6kIV42ehdO2z
    title: Bounded application and terminal extraction, neutral focus bridge and scoped verification
  - resource: opencode://ses_ec3144533ffeIEijLxykcDnn0V
    title: Corrected zoom stepping, failure honesty and amount validation in the terminal crates
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
  `ZoomType` and the resizer policy over any `FontPort` transport. Its
  input is a stream of `FocusEvent::Focus` or `FocusEvent::Blur`, each
  carrying a `FocusTarget { id: String, pid: Option<i32> }`. IDs are opaque
  diagnostic labels, not niri IDs. Blur carries the original terminal target.
  Neither crate depends on niri.

The [application bridge](../app/zk/src/niri/mod.rs) translates the legacy niri
adapter's events; it discards create/destroy events. The new desktop adapters
are not wired through this extraction. File/environment/CLI loading stays in
the [app config module](../app/zk/src/config.rs), and manual font command
presentation remains in the app.

## Zoom transition semantics

Kitty's `set-font-size` remote command accepts an increment operator (`+`,
`-`, `*`, `/`) and a positive float size and applies `old <op> size`
atomically, so every adjustment — fractional, additive or multiplicative — is
one remote command. [`zoomer::plan`](zoomer/src/plan.rs) is the pure planner
that decides those commands; the resizer executes them and commits state.

- **Steps split changes, they do not define totals.** `step_size` is the
  maximum magnitude of an intermediate command, used to smooth visible
  transitions. The final command carries the exact remainder, so the applied
  sum (additive/absolute) or product (multiplicative) always equals the
  intended change. The inherited `ceil`-based code overshot targets, and its
  `u32` cast silently dropped fractional steps — `step_size 0.5` sent zero
  commands while recording the full adjustment.
- **Non-progressing steps collapse to one exact command.** A multiplicative
  step `<= 1` — including the historical default `1.0` — cannot move the
  factor toward its target; the inherited loop multiplied by `*1.0` forever.
  Plans that would need more than `MAX_PLAN_STEPS` (128) commands also
  collapse. Every plan is finite by construction.
- **Failed commands do not advance state falsely.** Commands are applied in
  order; the first result that is not `Success` stops the plan. The tracked
  size advances only by the acknowledged portion, so a later event replans
  from what kitty actually did. The inherited code ignored results entirely.
- **Amounts are validated before commands.** `ZoomConfig::validate` rejects
  nonfinite values, non-positive `step_size`, non-positive `absolute` targets
  and non-positive `multiplicative` factors; the planner independently
  returns `Invalid` (no commands) for unusable state, and `zk-kitty` rejects
  nonfinite or non-positive amounts and unknown increment operators before
  building a command. A nonfinite tracked size is treated as untracked and
  replanned from the baseline rather than poisoning later transitions.
- **Focus/blur intent is preserved.** Focus applies the configured zoom;
  blur reverses it: absolute returns to the kitty.conf baseline, additive
  subtracts the same amount, multiplicative returns to factor 1.0. An
  untracked size starts from the configured baseline with the historical
  `12.0` fallback.

Verification: `cargo nextest run -p zk-kitty -p zk-zoomer` (32 tests: pure
planning fixtures plus a scripted command sink exercising full, partial and
refused transitions with no live kitty mutation) and `cargo clippy -p zk-kitty
-p zk-zoomer --all-targets`. The application package still compiles and its
tests pass against the unchanged public `KittyResizer` API.

## Remaining limitations

State is tracked by PID, and the resizer holds a per-window state map while
awaiting commands; multiple PIDs are handled sequentially. The `Failed`
`ZoomingResult` variant is still never produced by the registry. These are
deliberately out of scope for the correctness correction above.

## Extraction notes

Verification used scoped `cargo check --all-targets --all-features` and
`cargo nextest run` for the app and the three extracted domain crates. No live
kitty mutation was performed.
