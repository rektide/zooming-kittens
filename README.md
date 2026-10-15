---
type: Project
title: zooming-kittens
description: Compositor-driven window sizing and terminal font automation.
resource: README.md
sources:
  - resource: opencode://ses_ec3316eefffejmDBXbZ28spBd7
    title: Longcat remake design, implementation integration and workspace usage updates
---

# zooming-kittens

> Compositor-driven window sizing and terminal focus automation

Zooming-kittens uses native desktop streams for focus-driven window sizing and
[kitty](https://sw.kovidgoyal.net/kitty/) font control. Longcat supports
[niri](https://github.com/YaLTeR/niri) by-ID height changes and read-only Umbriel
observation through the sibling `umbriel-rs` libraries.

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

Uses Taffy to allocate heights within reported niri columns. The focused window
gets an emphasized share; siblings retain their baseline-relative weights within
the configured constraints. Leaving a column conditionally restores the baseline
when identity, context and observed effects still agree. Conflicting changes
suspend the episode rather than silently learning new proportions.

```bash
zk longcat
zk longcat --active-height-proportion 0.7 --other-height-proportion 0.3
zk longcat --dry-run --limit 1
zk longcat --backend umbriel --dry-run --limit 1
```

`--socket` selects an explicit native endpoint. `--limit` counts ready decisions,
so startup waits and final shutdown records may add JSONL rows. Weights normalize
as `active / (active + other)`; explicit CLI values override file/environment
configuration. `--convergence-timeout-ms` and `--shutdown-timeout-ms` govern the
effect and conditional-shutdown deadlines.

Umbriel currently reports windows, workspaces, activation and health, but lacks
the exact grouping and targeted resize contract needed here. Its dry-run reports
that limitation explicitly; mutation is rejected before connecting. No polling
or focus-switching workaround is used. `resize_debounce_ms` is retained for
configuration compatibility; automatic resize learning is disabled.

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

## Architecture

The workspace separates native observation (`desktop/`), Taffy allocation and
Longcat policy (`layout/`), font control (`terminal/`), native export
(`inspection/`) and runtime composition (`app/`). Kubeless supplies selected
observed-state, before/after impact and direct scheduling mechanisms.

See [`ARCHITECTURE.md`](ARCHITECTURE.md) for details.

The [remake reading map](design/remake/README.md) preserves the design rationale
and [Kubeless handoff](design/remake/kubeless-asks0.gpt6a.md), including the
implementation findings and deferred upstream work.

## Verification

The integrated remake passes **154 tests**, strict Clippy and formatting:

```sh
cargo nextest run --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

Native mutation/convergence tests use scripted compositor peers. A live read-only
Umbriel run acquired 31 windows and eight workspaces and exited cleanly; live
niri resizing and exact Umbriel control parity are not claimed.

## License

SEE LICENSE IN LICENSE
