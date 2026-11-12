---
type: Reference
title: Terminal remote control and focus zoom policy
description: Kitty transport ownership, niri and Umbriel focus zoom, bounded acquisition and explicit shutdown restoration.
resource: terminal/README.md
status: implemented
generated: { at: 2026-10-15, by: 'model:zai-coding-plan/glm-5.3#high' }
updated: { at: 2026-11-12, by: 'model:openai/gpt-6.1-sol#high' }
sources:
  - resource: opencode://ses_e310ffa7fffeeG5JUHl4w0WCiE
    title: Implement Umbriel focus zoom, initial epoch admission, targeted PID discovery, SIGINT restoration and feature-matrix regressions
  - resource: opencode://ses_ec3316eefffejmDBXbZ28spBd7
    title: Complete native focus-feed integration and replace the retired legacy bridge reference
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

The [application bridge](../app/zk/src/commands/zoomer/focus.rs) derives focus
transitions from native niri or Umbriel desktop captures. It preserves the original
blur target, handles explicit no-focus and deletion, filters without losing blur,
and reports bounded-delivery/source loss. Default mode still uses niri;
`zoomer --backend umbriel` explicitly selects Umbriel. Both use the same
registry, bridge and font policy, independently of
[Longcat's layout runner](../app/zk/src/runner/README.md).
File/environment/CLI loading stays in
the [app config module](../app/zk/src/config.rs), and manual font command
presentation remains in the app.

## Run alongside Longcat on Umbriel

```sh
cargo build -p zooming-kittens --no-default-features --features umbriel,kitty
target/debug/zk zoomer --backend umbriel --zoom-additive 3 --zoom-step-size 3
```

This explicit initial setting adds 3 points on focus and subtracts 3 on blur,
in one command each. It changes no config file or service and does not install
anything. The CLI defaults remain backend `niri`, app ID `kitty`, step size 1;
without a configured zoom mode, no font commands are sent. See the
[project usage guide](../README.md#zoomer) for the three existing modes.

The source uses Umbriel's stock subscription and native endpoint discovery;
no layout control extension or compositor mutation is needed. Opaque text
window IDs remain diagnostic labels. The reported PID is resolved to a kitty
master, checking the supplied process itself before at most 19 ancestors.
Whitespace/parentheses in Linux process names do not corrupt the parent lookup,
and each command rechecks ancestry rather than trusting a PID cache.
Only that master's `kitty-<pid>.sock` is selected in `XDG_RUNTIME_DIR` (otherwise
the current UID's runtime directory, with `/tmp` only when runtime discovery
is unavailable). No other kitty instance is substituted if the socket is absent.
Kitty remote control and its existing authorization must already be configured;
the password is read from `$XDG_CONFIG_HOME/kitty/rc.password` (normally
`~/.config/kitty/rc.password`). Password contents and raw remote error replies
are not logged.

Acquisition binds the first current windows census, not Umbriel's unseen epoch-0
placeholder. Exactly the pristine initial Umbriel `EpochChanged` marker is
permitted; later discontinuities, stale captures, epoch changes, lag and source
closure stop control. Startup checks queued continuity evidence before using
the latest capture, so a healthy reconnect cannot hide prior loss and synthesize
focus. Native observation remains bounded (64 semantic updates by default),
with a separate 32-slot transition queue; one slot holds at most blur + focus.
Overflow stops the zoomer rather than coalescing or blocking behind kitty I/O.

### Orderly shutdown is not desktop blur

Send **SIGINT** (Ctrl-C), not SIGTERM/SIGKILL, to request restoration. The zoomer
stops admitting focus work, lets the one started command settle, discards its
unsent animation tail and queued focuses, then reverses the net adjustments this
run actually received acknowledgments for. Already-balanced blur effects need
no extra command. Restoration uses relative inverses, not a saved absolute size;
additive zoom therefore does not reset manual font-size changes. Restoration
has a 15-second deadline, followed by bounded source shutdown (3 seconds).

Source loss is still **not** evidence of blur and does not trigger speculative
restoration. A lost reply, refused restore, deadline expiry or abrupt termination
can leave a font adjustment behind; errors do not claim rollback. Inspect and
correct that state before restarting. Existing registry retry/transport ambiguity
is not redesigned here.

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

Verification on 2026-11-12: `cargo nextest run -p zooming-kittens -p zk-kitty
-p zk-zoomer --all-features` passed 119 tests. Fixtures cover opaque Umbriel
IDs/PIDs, initial acquisition before/after feed startup, queued loss/reconnect,
bounded delivery, process/socket targeting, partial/refused commands, shutdown
inverse effects and SIGINT-style stopping during/between commands without
draining queued focus. No live kitty mutation was performed in this session.

Application nextest feature matrix (`--no-default-features`): none **11**,
kitty **17**, niri+kitty **47**, umbriel+kitty **61**, all **78** tests passed.
Scoped `cargo clippy -p zooming-kittens -p zk-kitty -p zk-zoomer --all-targets
--no-default-features [FEATURES] -- -D warnings` passed all five combinations.
`cargo tree` also verified Umbriel+kitty has neither `zk-niri` nor `niri-ipc`,
niri+kitty has no `zk-umbriel`, and kitty-only/empty builds omit desktop adapters.

## Remaining limitations

State is tracked by PID, not kitty OS-window identity: multiple OS windows in one
kitty master are not independently addressed by this bridge. The resizer holds
a per-window state map while awaiting commands; multiple PIDs are handled
sequentially. Absolute zoom retains its existing kitty.conf-baseline semantics,
not a query of each terminal's arbitrary current size. The `Failed`
`ZoomingResult` variant is still never produced by the registry. These are
deliberately out of scope for this backend composition.

## Extraction notes

Verification used scoped `cargo check --all-targets --all-features` and
`cargo nextest run` for the app and the three extracted domain crates. No live
kitty mutation was performed.
