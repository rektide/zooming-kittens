# Window Focus Event Streaming Research

## Problem

kitty-focus-tracker currently uses a polling approach (every 1 second) to detect focus changes in niri. This is inefficient and could miss rapid focus changes.

## Research Findings

### 1. niri IPC Event Stream Issue

**Original problem**: The `WindowFocusChanged` event in niri has a bug:

- **GitHub Issue**: #1889 - "Niri doesn't emit `WindowFocusChanged` event if window is spawned under cursor"
- **Symptom**: Event is not emitted when windows are spawned via keybind or launcher
- **Reported**: June 23, 2025
- **Status**: Closed - but issue persists in current version

### 2. Solution in niri v25.11

**Release**: v25.11 (November 29, 2025)

**New Feature**: `WindowFocusTimestampChanged` event was added:

> "niri IPC now exposes window `focus_timestamp` and an event stream `WindowFocusTimestampChanged`"

This new event:
- Reliably tracks focus changes with timestamps
- Uses debounce to avoid intermediate window spamming
- Fixes the bug where `WindowFocusChanged` was not emitted

### 3. Current niri-ipc Version Issue

**Installed**: niri-ipc 25.11.0
**Available**: niri-ipc 25.11.0 (or later versions with WindowFocusTimestampChanged)

The current niri-ipc crate we're using **does not include** the `WindowFocusTimestampChanged` event.

**Verification**:
```bash
$ grep "WindowFocusTimestampChanged" ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/niri-ipc-25.11.0/src/lib.rs
# No matches - event not available in this version
```

### 4. Available niri-ipc Versions

From `cargo search`:

- **niri-ipc 25.11.0** - Current installed version (no WindowFocusTimestampChanged)
- **niri-ipc 0.250501.0** - Available on crates.io (may have the event)
- **multibig-wayland-niri-ipc 0.1.0** - Another niri IPC crate

## Implementation Plan

### Phase 1: Upgrade niri-ipc Dependency

Update `Cargo.toml` to use a version that includes `WindowFocusTimestampChanged`:

```toml
[dependencies]
niri-ipc = "0.250501.0"  # or latest with event
```

### Phase 2: Update Event Handling

In `src/main.rs`, replace polling loop with event stream handling:

```rust
// Current (polling):
loop {
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    let mut socket = Socket::connect()?;
    let reply = socket.send(Request::Windows)?;
    // ... check for focus changes
}

// New (event streaming):
let mut socket = Socket::connect()?;
let reply = socket.send(Request::EventStream)?;
let mut read_event = socket.read_events();

loop {
    match read_event()? {
        niri_ipc::Event::WindowFocusTimestampChanged { id } => {
            // Handle focus change with timestamp
            // Use debounce/delay as needed
        }
        // ... other events
    }
}
```

### Phase 3: Add Timestamp Debouncing

Since `WindowFocusTimestampChanged` uses debounce (mentioned in v25.11 release notes), implement:

```rust
// Quick focus switches should not spam font changes
const FOCUS_DEBOUNCE_MS: u64 = 100; // 100ms debounce

// Track last focus time
let last_focus_time: Option<std::time::Instant> = None;

fn should_handle_focus_change(timestamp: Option<u64>) -> bool {
    match last_focus_time {
        Some(last) => {
            let elapsed = last.elapsed().as_millis();
            elapsed > FOCUS_DEBOUNCE_MS
        }
        None => true,
    }
}
```

### Phase 4: Testing

After upgrading:

1. Test that focus changes are now detected via events (not polling)
2. Verify no `WindowFocusChanged` usage remains
3. Test rapid focus switching to ensure debouncing works correctly
4. Confirm font size adjustments happen only once per focus change

## Benefits of Event Streaming vs Polling

| Aspect | Polling (Current) | Event Streaming (Proposed) |
|---------|---------------------|---------------------------|
| Latency | Up to 1 second | Event-driven (immediate) |
| CPU usage | Continuous requests | On-demand (lower) |
| Focus precision | May miss rapid changes | All changes captured |
| Resource usage | 1 req/sec | Event-based |

## References

- niri GitHub Issue #1889: https://github.com/YaLTeR/niri/issues/1889
- niri v25.11 Release: https://github.com/YaLTeR/niri/releases/tag/v25.11
- niri-ipc crate: https://crates.io/crates/niri-ipc
