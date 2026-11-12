---
type: Project
title: zooming-kittens
description: Compositor-driven window sizing and terminal font automation.
resource: README.md
status: maintained
sources:
  - resource: opencode://ses_e32c4da49ffekNe2490gCdHuLp
    title: Deliver ordinary stock Feedback application composition, explicit conditional routing, configuration and scripted acceptance
  - resource: opencode://ses_e9a70a796ffelZmIZrb9Zxlb4f
    title: Independent backend feature implementation and renewed Umbriel Longcat work
  - resource: opencode://ses_ec3316eefffejmDBXbZ28spBd7
    title: Longcat remake design, implementation integration and workspace usage updates
---

# zooming-kittens

> Compositor-driven window sizing and terminal focus automation

Zooming-kittens uses native desktop streams for focus-driven window sizing and
[kitty](https://sw.kovidgoyal.net/kitty/) font control. Longcat supports
[niri](https://github.com/YaLTeR/niri) by-ID height changes and stock Umbriel
feedback through the sibling `umbriel-rs` libraries. Bespoke conditional Umbriel
allocation remains an explicit alternative.

## Features

### Zoomer

Automatically adjusts terminal font size when a window gains or loses focus. Three zoom strategies:

| Mode | Flag | Behavior |
|------|------|----------|
| **Additive** | `--zoom-additive 6` | +6pt on focus, -6pt on blur |
| **Multiplicative** | `--zoom-multiplicative 1.5` | 1.5x on focus, /1.5 on blur |
| **Absolute** | `--zoom-absolute 18` | Set to 18pt on focus, restore on blur |

Use `--zoom-step-size` to break large adjustments into smaller increments for smoother transitions.

```bash
# Additive zoom: +6pt on focus, -6pt on blur
zk --zoom-additive 6

# Absolute: always use 18pt for the focused terminal
zk zoomer --zoom-absolute 18

# Multiplicative with stepped transitions
zk zoomer --zoom-multiplicative 1.5 --zoom-step-size 2
```

### Longcat

Uses Taffy to allocate content space within reported compositor groups. The focused
window gets an emphasized share; siblings retain neutral relative weights.
Stock Umbriel supports both secondary axes, yields during movement and accepts
quiet user resize guidance. Blur/shutdown solve current neutral preferences.
Niri and explicitly selected conditional Umbriel keep immutable-baseline Episode
semantics; conflicting changes suspend those routes rather than learning.

```bash
zk longcat
zk longcat --active-height-proportion 0.7 --other-height-proportion 0.3
zk longcat --dry-run --limit 1
zk longcat --backend umbriel --dry-run --limit 1
zk longcat --backend umbriel --minimum-width 60
# Only for the separately preserved bespoke extension:
zk longcat --backend umbriel --umbriel-control conditional
```

`--socket` selects an explicit native endpoint. `--limit` counts ready decisions,
so startup waits and final shutdown records may add JSONL rows. Weights normalize
as `active / (active + other)`; explicit CLI values override file/environment
configuration. `--convergence-timeout-ms` and `--shutdown-timeout-ms` govern the
effect and conditional-shutdown deadlines.

Ordinary Umbriel control needs the working-line stock layout/output fields and
`--window`-qualified sizing, not the bespoke extension. It verifies that grammar
without targeting a live window. Unsupported/tabbed/maximized/ambiguous groups
wait; no focused fallback is used. `resize_debounce_ms` defaults to 500ms and is
the trailing quiet interval, separate from convergence. Height and width minima
remain distinct, both defaulting to 60. Native constraints can end a bounded
attempt as limited, rather than looping or teaching the failed target.

The stock runner is implemented and script-tested; its full integrated native
user-guidance acceptance remains open. Earlier conditional and stock geometry
headless checks are separate evidence. No installed desktop/service change is
claimed; see the [delivery record](design/longcat/implementation-d0.gpt61s.md).

See the [runner contract](app/zk/src/runner/README.md) for admission, retention
limits, partial effects, loss handling and diagnostics. Use dry-run to inspect
the current native input before enabling changes.

### Export

Exports raw niri state and event records as JSON or JSON Lines. By default, `export` prints a snapshot first, then continues with the niri event stream.

```bash
zk export
zk export --snapshot --format json
zk export --stream --filter type=window_layouts_changed
zk export --filter output --current-mode
```

Records use a flat `type` field, for example `window`, `workspace`, `output`, `cast`, or `window_layouts_changed`. Use `--filter` to select record kinds, IDs, or event types:

```bash
zk export --filter window=4,workspace=9,output=DP-3
zk export --stream --filter type=cast_started_or_changed
```

See [`doc/export.md`](doc/export.md) for record shapes, filter rules, stream IDs, and output format details.

### Font control

Direct font size manipulation for running kitty instances:

```bash
zk font inc          # increase font size
zk font dec --count 3  # decrease by 3 steps
zk font set 14       # set absolute size
zk font list         # show running kitty instances
```

### Utilities

- **conf-size** -- read the configured font size from `kitty.conf`
- **export** -- print raw niri snapshot and stream records
- **generate-systemd** -- produce a systemd user service file for auto-startup
- **cleanup** -- reap dead connections from the connection pool

## Install

### From source

```bash
cargo install --path app/zk
```

The checkout uses a domain-grouped Cargo workspace. `cargo run -- --help` and
`cargo build --release` at the root select the `zk` application; use
`cargo nextest run --workspace` to check all workspace packages.

Desktop packages currently use sibling development checkouts of `../umbriel-rs`
and `../kubeless`. Their relative path dependencies are part of the development
workspace setup.

### Select backends

The application enables `niri`, `umbriel`, and `kitty` by default. Select only
the backends needed for an installation:

```sh
cargo build -p zooming-kittens --no-default-features --features umbriel
cargo build -p zooming-kittens --no-default-features --features niri
cargo build -p zooming-kittens --no-default-features --features niri,kitty
```

`kitty` enables terminal font commands; automatic font zoom additionally requires
`niri`. `niri` enables native export. `export-facet` enables niri/export as well as
its optional serialization scaffold. Disabled commands explain the required
features before opening sockets. Umbriel-only builds support Longcat control on
compatible compositors; see the [Longcat return effort](design/longcat/README.md)
for deployment progress.

## Configuration

Configuration is loaded via [figment2](https://docs.rs/figment2) from multiple sources, highest priority wins:

1. **CLI arguments**
2. **Environment variables** (prefixed `ZK_`, e.g. `ZK_APP_ID`, `ZK_ZOOM__ADDITIVE`)
3. **Config file** -- `$XDG_CONFIG_HOME/kitty-focus-tracker/config.toml`
4. **Defaults**

See [`config.example.toml`](config.example.toml) for a full annotated sample.

## Running as a systemd service

```bash
# Generate a service file
zk generate-systemd --output > ~/.config/systemd/user/kitty-focus-tracker.service

# Or use the helper script
./run.sh
./journal.sh   # tail logs
```

Generate a separate window-sizing unit explicitly:

```sh
zk generate-systemd --output longcat --backend umbriel --dry-run \
  --active-height-proportion 3 --other-height-proportion 2 \
  --minimum-height 80 > longcat.service
```

Add `--socket /path/to/umbriel.sock` to fix the endpoint; otherwise discovery uses
the service's environment. All Longcat options are supported. The generated unit
requires an already-running compositor, follows its session lifecycle, and sends
SIGINT with a stop budget allowing conditional restoration. Generation only prints
the unit; it does not install or start it. Bare `generate-systemd --output` retains
the font-tracker service. Remove `--dry-run` only after checking the intended
compositor's actual qualified stock capabilities and the acceptance record;
explicit conditional mode still requires its native allocation extension.

## Architecture

### Umbriel activity

The sibling Umbriel project now supplies a
[reusable tracking library](file:///home/rektide/src/umbriel-rs/crates/tracking/README.md)
and a [structured activity service](file:///home/rektide/src/umbriel-rs/crates/activity/README.md).
The dedicated `umbriel-activity.service` is installed on this workspace's desktop.
Follow its structured records with:

```sh
journalctl --user _SYSTEMD_USER_UNIT=umbriel-activity.service \
  UMBRIEL_SCHEMA_VERSION=1 TARGET=umbriel::activity --all -o json --follow
```

Activity acceptance defaults to five seconds; Longcat consumes immediate native
state. Journal history/replay is tracked separately upstream. The
[Longcat effort map](design/longcat/README.md) links implementation and deployment
evidence, journald precedents and the optional Kopacetic integration.

### Workspace domains

The workspace separates native observation (`desktop/`), Taffy allocation and
Longcat policy (`layout/`), font control (`terminal/`), native export
(`inspection/`) and runtime composition (`app/`). Kubeless supplies selected
observed-state, before/after impact and direct scheduling mechanisms.

See [`ARCHITECTURE.md`](ARCHITECTURE.md) for details.

The [remake reading map](design/remake/README.md) preserves the design rationale
and [Kubeless handoff](design/remake/kubeless-asks0.gpt6a.md), including the
implementation findings and deferred upstream work.

## Verification

The latest Umbriel integration passed 107 scoped tests, strict Clippy, all ten
application feature configurations and service-generation tests. Workspace checks:

```sh
cargo nextest run --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

Scripted peers cover failure and continuity handling. Real ZK/native headless tests
verify emphasis, focus switching, raw-weight restoration and external-edit
rejection, including four concurrent instances with titlebars and nonzero gaps.
The running desktop has not yet been upgraded or used for a sizing trial.

## License

SEE LICENSE IN LICENSE
