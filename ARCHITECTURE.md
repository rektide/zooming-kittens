# Architecture

## Current Structure

```
src/
├── main.rs                    # Entry point with CLI parsing and orchestration
├── config.rs                   # Configuration management using figment2
├── commands/                  # Command handlers
│   ├── mod.rs
│   ├── fonts.rs             # Font size commands (inc/dec/set/list)
│   └── systemd.rs           # systemd service generation
├── niri/                       # Niri window manager event handling
│   ├── mod.rs
│   ├── types.rs              # Event types (NiriEvent, WindowInfo)
│   └── registry.rs            # Event stream provider (NiriRegistry)
└── kitty/                       # Kitty terminal operations
    ├── mod.rs
    ├── registry.rs            # Kitty connection management (KittyRegistry)
    ├── types.rs              # Kitty types (KittyConnectionStatus, ZoomingResult)
    ├── util.rs               # Utility functions (password, socket path, process alive)
    ├── process.rs            # Process discovery (PID mapping)
    └── resizer.rs            # Stream consumer (KittyResizer)
```

## Configuration

Configuration is managed by **figment2** and loads from multiple sources in order of precedence:

1. **Default values** - Built into the Config struct
2. **Config file** - `$XDG_CONFIG_HOME/kitty-focus-tracker/config.toml`
3. **Environment variables** - Prefixed with `KFT_` (e.g., `KFT_VERBOSE=true`)
4. **CLI arguments** - Highest priority, override all other sources

### Environment Variables

- `KFT_APP_ID` - Application ID to track
- `KFT_VERBOSE` - Enable verbose logging
- `KFT_SOCKET_TIMEOUT_SECS` - Socket timeout in seconds
- `KFT_MAX_RETRIES` - Maximum connection retry attempts
- `KFT_MAX_CONNECTIONS` - Maximum concurrent connections
- `KFT_IDLE_TIMEOUT_SECS` - Idle connection timeout in seconds
- `KFT_REAP_INTERVAL_SECS` - Connection reaping interval in seconds

### Example Config File

See `config.example.toml` for a sample configuration file.

## Async Stream Architecture

The application uses Rust's async Stream trait to create a composable, event-driven architecture:

### Event Flow

1. **NiriRegistry** connects to niri IPC and creates an event stream
2. Events are filtered to find kitty windows matching the target app_id
3. **KittyResizer** consumes the filtered stream and adjusts font sizes

### Key Components

#### NiriRegistry (`src/niri/registry.rs`)
- **Purpose**: Provide event streams from niri window manager
- **Key Methods**:
  - `new()` - Connect to niri and start event listener
  - `into_events()` - Consume registry and return event stream
  - `windows_matching(predicate)` - Filter events by window properties
  - `focus_events()` - Stream of focus events only
  - `blur_events()` - Stream of blur events only

#### KittyResizer (`src/kitty/resizer.rs`)
- **Purpose**: Consume niri events and adjust kitty font sizes
- **Key Methods**:
  - `new(kitty_registry)` - Create resizer with KittyRegistry
  - `process_events(stream)` - Consume event stream and process Focus/Blur events

#### KittyRegistry (`src/registry.rs`)
- **Purpose**: Manage kitty terminal connections and execute commands
- **Features**:
  - Connection pooling with automatic cleanup
  - PID mapping (shell → kitty master)
  - Retry logic and timeouts
  - Idle connection reaping

### Event Types

```rust
enum NiriEvent {
    Focus { window_id: u64, window: WindowInfo },
    Blur { window_id: u64, window: WindowInfo },
    Create { window_id: u64, window: WindowInfo },
    Destroy { window_id: u64 },
}

struct WindowInfo {
    id: u64,
    app_id: Option<String>,
    pid: Option<i32>,
    title: Option<String>,
}
```

## Benefits of Stream Architecture

1. **Composability**: Use standard stream combinators (filter, map, etc.)
2. **Testability**: Each component can be tested with mock streams
3. **Flexibility**: Easy to add new event consumers or filters
4. **Separation**: Niri events and kitty operations are cleanly separated
5. **Type Safety**: Compile-time guarantees about event types

## Example Usage

```rust
let niri_registry = NiriRegistry::new().await?;
let kitty_registry = KittyRegistry::new(config);
let mut resizer = KittyResizer::new(kitty_registry);

// Filter for kitty windows
let kitty_events = niri_registry.windows_matching(|window| {
    window.app_id.as_deref() == Some("kitty")
});

// Process events
resizer.process_events(kitty_events).await?;
```

## Future Enhancements

- Add debounce support to prevent rapid font adjustments
- Support multiple kitty instances with different configurations
- Add event logging and debugging tools
- Implement plugin system for custom event handlers
