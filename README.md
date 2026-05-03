# zooming-kittens

> Niri window manager toolkit for dynamic terminal sizing and focus-driven automation

Zooming-kittens connects to the [niri](https://github.com/YaLTeR/niri) scrollable-tiling Wayland compositor via IPC and reacts to window focus changes. Currently it ships integrations for [kitty](https://sw.kovidgoyal.net/kitty/) terminal; support for additional terminals is planned.

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
kitty-focus-tracker --zoom-additive 6

# Absolute: always use 18pt for the focused terminal
kitty-focus-tracker zoomer --zoom-absolute 18

# Multiplicative with stepped transitions
kitty-focus-tracker zoomer --zoom-multiplicative 1.5 --zoom-step-size 2
```

### Longcat

Manages window heights within niri columns. When a window in a multi-window column gains focus, the active window expands and others contract. Original heights are restored when focus leaves the column. Manual resizes are detected and adapted to after a configurable debounce period.

```bash
kitty-focus-tracker longcat
kitty-focus-tracker longcat --active-height-proportion 0.7 --other-height-proportion 0.3
```

### Export

Exports raw niri state and event records as JSON or JSON Lines. By default, `export` prints a snapshot first, then continues with the niri event stream.

```bash
kitty-focus-tracker export
kitty-focus-tracker export --snapshot --format json
kitty-focus-tracker export --stream --filter type=window_layouts_changed
kitty-focus-tracker export --filter output --current-mode
```

Records use a flat `type` field, for example `window`, `workspace`, `output`, `cast`, or `window_layouts_changed`. Use `--filter` to select record kinds, IDs, or event types:

```bash
kitty-focus-tracker export --filter window=4,workspace=9,output=DP-3
kitty-focus-tracker export --stream --filter type=cast_started_or_changed
```

See [`doc/export.md`](doc/export.md) for record shapes, filter rules, stream IDs, and output format details.

### Font control

Direct font size manipulation for running kitty instances:

```bash
kitty-focus-tracker font inc          # increase font size
kitty-focus-tracker font dec --count 3  # decrease by 3 steps
kitty-focus-tracker font set 14       # set absolute size
kitty-focus-tracker font list         # show running kitty instances
```

### Utilities

- **conf-size** -- read the configured font size from `kitty.conf`
- **export** -- print raw niri snapshot and stream records
- **generate-systemd** -- produce a systemd user service file for auto-startup
- **cleanup** -- reap dead connections from the connection pool

## Install

### From source

```bash
cargo install --path .
```

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
kitty-focus-tracker generate-systemd --output > ~/.config/systemd/user/kitty-focus-tracker.service

# Or use the helper script
./run.sh
./journal.sh   # tail logs
```

## Architecture

The application uses an async stream architecture built on tokio:

- **NiriRegistry** connects to niri IPC and produces event streams (focus, blur, create, destroy)
- **KittyResizer** consumes filtered streams and adjusts font sizes through kitty's remote control socket
- **KittyRegistry** manages pooled connections with retry logic, idle reaping, and PID-based socket discovery

See [`ARCHITECTURE.md`](ARCHITECTURE.md) for details.

## License

SEE LICENSE IN LICENSE
