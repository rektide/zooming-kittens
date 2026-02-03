# Plan: Async Stream Architecture for Niri Window Events

## Overview

Replace the current hook-based approach with an async stream architecture for handling niri window events. This provides a more Rust-idiomatic, composable, and flexible system for processing window events.

## Goals

1. **Rust-idiomatic**: Use `Stream` trait from futures library
2. **Composable**: Chain operations (filter, map, etc.) on events
3. **Type-safe**: Strongly typed event streams
4. **Testable**: Easy to mock and test event processing
5. **Flexible**: Support multiple consumers of the same event stream

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Niri Event Source                  │
│                  (niri_ipc::Socket)                  │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
              ┌──────────────┐
              │ NiriRegistry │
              │              │
              │ - Stream API │
              └──────┬───────┘
                     │
         ┌───────────┼───────────┐
         │           │           │
         ▼           ▼           ▼
   ┌────────┐  ┌─────────┐  ┌──────────┐
   │ Stream │  │ Stream  │  │  Stream  │
   │  A    │  │  B      │  │    C     │
   └───┬────┘  └────┬────┘  └────┬─────┘
       │              │              │
       ▼              ▼              ▼
   ┌────────┐    ┌──────────┐   ┌──────────┐
   │Kitty   │    │ Logging  │   │Metrics   │
   │Resizer  │    │ Handler  │   │Collector  │
   └────────┘    └──────────┘   └──────────┘
```

## Component Design

### 1. NiriRegistry (src/niri/registry.rs)

**Purpose**: Manages connection to niri and provides event streams

**API:**

```rust
pub struct NiriRegistry {
    // Internal state: socket, event channel, etc.
}

impl NiriRegistry {
    /// Create new registry, connect to niri
    pub async fn new() -> Result<Self>;

    /// Get a stream of all niri events
    pub fn events(&self) -> impl Stream<Item = NiriEvent> + '_;

    /// Get stream filtered by event type
    pub fn focus_events(&self) -> impl Stream<Item = FocusEvent> + '_;
    pub fn blur_events(&self) -> impl Stream<Item = BlurEvent> + '_;

    /// Get stream filtered by window predicate
    pub fn windows_matching<P>(&self, predicate: P) -> impl Stream<Item = WindowEvent> + '_
    where
        P: Fn(&WindowInfo) -> bool + Send + Sync;

    /// Run the event loop (blocks forever)
    pub async fn run(&mut self) -> Result<Infallible>;
}

// Event types
#[derive(Debug, Clone)]
pub enum NiriEvent {
    Focus { window_id: u64, window: WindowInfo },
    Blur { window_id: u64, window: WindowInfo },
    Create { window_id: u64, window: WindowInfo },
    Destroy { window_id: u64 },
}

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub id: u64,
    pub app_id: Option<String>,
    pub pid: Option<i32>,
    pub title: Option<String>,
    // ... other fields from niri IPC
}
```

**Implementation Details:**

```rust
use futures::Stream;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

impl NiriRegistry {
    pub fn events(&self) -> impl Stream<Item = NiriEvent> + '_ {
        // Clone the receiver to create a stream
        ReceiverStream::new(self.event_rx.clone())
    }

    pub fn windows_matching<P>(&self, predicate: P) -> impl Stream<Item = WindowEvent> + '_
    where
        P: Fn(&WindowInfo) -> bool + Send + Sync,
    {
        self.events()
            .filter(move |event| {
                matches!(event,
                    NiriEvent::Focus { window } | NiriEvent::Blur { window })
                    if (predicate)(window)
                )
            })
            .map(|event| match event {
                NiriEvent::Focus { window, .. } => WindowEvent::Focus(window),
                NiriEvent::Blur { window, .. } => WindowEvent::Blur(window),
                _ => unreachable!(),
            })
    }
}
```

### 2. Kitty Resizer (src/kitty/resizer.rs)

**Purpose**: Consumes kitty window events and adjusts font sizes

**API:**

```rust
pub struct KittyResizer {
    kitty_registry: KittyRegistry,
}

impl KittyResizer {
    pub fn new(kitty_registry: KittyRegistry) -> Self;

    /// Consume event stream and process kitty events
    pub async fn process_events(
        &mut self,
        mut events: impl Stream<Item = WindowEvent> + Unpin,
    ) -> Result<()>;
}
```

**Implementation Details:**

```rust
impl KittyResizer {
    pub async fn process_events(
        &mut self,
        mut events: impl Stream<Item = WindowEvent> + Unpin,
    ) -> Result<()> {
        while let Some(event) = events.next().await {
            match event {
                WindowEvent::Focus(window) => {
                    if let Some(pid) = window.pid {
                        self.kitty_registry.increase_font_size(pid).await?;
                    }
                }
                WindowEvent::Blur(window) => {
                    if let Some(pid) = window.pid {
                        self.kitty_registry.decrease_font_size(pid).await?;
                    }
                }
            }
        }
        Ok(())
    }
}
```

### 3. Main Entry Point (src/main.rs)

**Purpose**: Wire up components and start event processing

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Handle subcommands
    if let Some(subcommand) = args.command {
        handle_font_command(subcommand).await?;
        return Ok(());
    }

    // Create niri registry
    let mut niri_registry = NiriRegistry::new().await?;

    // Create kitty registry
    let kitty_registry = KittyRegistry::new(RegistryConfig::default());

    // Create resizer
    let mut resizer = KittyResizer::new(kitty_registry);

    // Get filtered event stream for kitty windows
    let kitty_events = niri_registry
        .windows_matching(|window| window.app_id.as_deref() == Some("kitty"));

    // Process events
    if let Err(e) = resizer.process_events(kitty_events).await {
        eprintln!("Error processing events: {}", e);
    }

    Ok(())
}
```

## Migration Steps

### Phase 1: Create NiriRegistry Foundation

**Tasks:**
1. Create `src/niri/mod.rs` - module exports
2. Create `src/niri/registry.rs` - NiriRegistry implementation
3. Define event types (`NiriEvent`, `WindowInfo`)
4. Implement basic event stream from niri IPC
5. Add `tokio-stream` dependency for stream utilities

**Testing:**
- Unit test event parsing from niri IPC responses
- Unit test event channel broadcasting
- Integration test connecting to real niri instance

### Phase 2: Implement Stream Methods

**Tasks:**
1. Add `events()` method returning `impl Stream<Item = NiriEvent>`
2. Add `focus_events()` and `blur_events()` filtered methods
3. Add `windows_matching<P>()` generic filter method
4. Implement stream composition using `filter`, `map`, etc.

**Testing:**
- Test stream filtering works correctly
- Test predicate functions are called correctly
- Test stream clones for multiple consumers

### Phase 3: Create Kitty Resizer

**Tasks:**
1. Extract resizer logic from current registry.rs
2. Create `src/kitty/resizer.rs`
3. Implement `process_events()` method consuming stream
4. Update imports and dependencies

**Testing:**
- Unit test resizer with mock event stream
- Test focus/blur logic separately
- Integration test with real kitty instance

### Phase 4: Update Main Entry Point

**Tasks:**
1. Create `src/niri/mod.rs` exports
2. Update `src/main.rs` imports
3. Replace current event loop with stream-based approach
4. Wire up NiriRegistry, KittyRegistry, KittyResizer
5. Remove old registry.rs (or keep for backward compat initially)

**Testing:**
- Full integration test with niri and kitty
- Test all subcommands still work
- Test focus tracking functionality

### Phase 5: Cleanup and Documentation

**Tasks:**
1. Remove old `FocusTracker` from registry.rs
2. Update ARCHITECTURE.md to reflect new structure
3. Add inline documentation to public APIs
4. Update this PLAN-stream.md with actual implementation notes

**Testing:**
- Run full integration test suite
- Manual testing with real niri/kitty setup
- Performance profiling (no regressions)

## Dependencies

Add to `Cargo.toml`:

```toml
[dependencies]
# ... existing ...
futures = "0.3"
tokio-stream = "0.1"  # For ReceiverStream and stream utilities
```

## Examples

### Example 1: Simple Event Logging

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let mut niri_registry = NiriRegistry::new().await?;

    niri_registry
        .events()
        .for_each(|event| {
            println!("Event: {:?}", event);
            future::ready(())
        })
        .await;

    Ok(())
}
```

### Example 2: Multi-Consumer Streams

```rust
async fn run() -> Result<()> {
    let niri_registry = NiriRegistry::new().await?;

    // Clone stream for different consumers
    let stream_a = niri_registry.events();
    let stream_b = niri_registry.events();

    // Run multiple consumers in parallel
    let (result_a, result_b) = tokio::join!(
        consume_kitty_events(stream_a),
        log_all_events(stream_b),
    );

    result_a?;
    result_b?;

    Ok(())
}

async fn consume_kitty_events(events: impl Stream<Item = NiriEvent> + Unpin) -> Result<()> {
    events
        .filter(|e| matches!(e, NiriEvent::Focus { window, .. } if window.app_id == Some("kitty".into())))
        .for_each(|event| handle_kitty(event))
        .await
}

async fn log_all_events(events: impl Stream<Item = NiriEvent> + Unpin) {
    events.for_each(|event| println!("{:?}", event)).await
}
```

### Example 3: Composable Operations

```rust
async fn run() -> Result<()> {
    let niri_registry = NiriRegistry::new().await?;

    let kitty_events = niri_registry
        .windows_matching(|w| w.app_id == Some("kitty".into()))
        .map(|event| match event {
            WindowEvent::Focus(w) => (w.pid, FocusOp::In),
            WindowEvent::Blur(w) => (w.pid, FocusOp::Out),
        })
        .filter_map(|(pid, op)| pid.map(|p| (p, op)))
        .buffer_unordered(10)  // Process up to 10 ops concurrently
        .for_each(|(pid, op)| async move {
            match op {
                FocusOp::In => kitty_registry.increase_font_size(pid).await,
                FocusOp::Out => kitty_registry.decrease_font_size(pid).await,
            }
        })
        .await;

    Ok(())
}
```

## Trade-offs and Considerations

### Stream Approach vs Hook Approach

| Aspect | Hook Approach | Stream Approach |
|--------|---------------|-----------------|
| Complexity | Simple callbacks | More async/await syntax |
| Composability | Limited (manual chaining) | Excellent (operators) |
| Rust Idiomatic | Less so | Very idiomatic |
| Testing | Easy (call hook directly) | Easy (mock streams) |
| Multiple Consumers | Hard (need global state) | Easy (clone streams) |
| Dynamic Addition/Removal | Easy (add/remove hook) | Medium (need stream combinators) |
| Error Propagation | Manual | Built into futures |
| Backpressure | Manual | Automatic (channel limits) |

### Why Stream Approach is Better for This Project

1. **Composability**: We need to filter (kitty windows), then map to operations
2. **Buffering**: We may want to batch font operations
3. **Multiple Consumers**: Could add logging, metrics, notifications later
4. **Error Handling**: Stream operators handle propagation cleanly
5. **Type Safety**: Streams are strongly typed throughout

### Performance Considerations

1. **Channel Capacity**: Set reasonable bounds on event channel
   ```rust
   let (tx, rx) = mpsc::channel(100); // Buffer 100 events
   ```

2. **Cloning Streams**: Each clone creates new receiver over same channel
   - Lightweight (just a clone of Arc<Receiver>)
   - Shared events, not duplicated data

3. **Stream Drop**: Clean up when consumers complete
   - Receiver is dropped when stream ends
   - Registry continues running if other consumers exist

## Testing Strategy

### Unit Tests

1. **Event Types**: Test `NiriEvent` enum variants
2. **WindowInfo**: Test struct fields and parsing
3. **Stream Filtering**: Test `windows_matching()` with various predicates
4. **Stream Cloning**: Test multiple independent streams
5. **Resizer Logic**: Test with mock event sequences

### Integration Tests

1. **Niri Connection**: Test connecting to real niri instance
2. **Kitty Operations**: Test font resizing with real kitty
3. **End-to-End**: Test full event flow from niri to kitty

### Mocking Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_stream::wrappers::ReceiverStream;
    use futures::stream;

    #[tokio::test]
    async fn test_resizer() {
        // Create mock event stream
        let (tx, rx) = mpsc::channel(10);
        let events = ReceiverStream::new(rx);

        // Send test events
        tx.send(WindowEvent::Focus(mock_window())).await.unwrap();

        // Create resizer with mock kitty registry
        let mock_kitty = MockKittyRegistry::new();
        let mut resizer = KittyResizer::new(mock_kitty);

        // Process events
        resizer.process_events(events).await.unwrap();

        // Verify operations were called
        assert!(mock_kitty.increase_was_called());
    }
}
```

## File Structure After Migration

```
src/
├── main.rs                      # Entry point
├── niri/                         # Niri event handling
│   ├── mod.rs
│   ├── registry.rs              # Event stream provider
│   └── types.rs                # Event types
├── kitty/                        # Kitty operations
│   ├── mod.rs
│   ├── registry.rs              # Connection pool, PID mapping
│   ├── resizer.rs              # Stream consumer for font ops
│   └── process.rs              # Process discovery
├── commands/                     # CLI commands
│   ├── mod.rs
│   └── fonts.rs
└── registry.rs                   # (deprecated, remove in final phase)
```

## Timeline

- **Phase 1**: 2-3 hours - NiriRegistry foundation
- **Phase 2**: 2-3 hours - Stream methods and filtering
- **Phase 3**: 1-2 hours - Kitty resizer extraction
- **Phase 4**: 2-3 hours - Main entry point updates
- **Phase 5**: 1-2 hours - Cleanup and documentation

**Total Estimate**: 8-13 hours

## Risks and Mitigations

### Risk 1: Stream Complexity
**Issue**: Async stream code can be harder to understand
**Mitigation**:
- Add extensive inline documentation
- Provide simple examples
- Keep stream chains short, use named functions for complex logic

### Risk 2: Testing Challenges
**Issue**: Testing async code can be tricky
**Mitigation**:
- Use `tokio::test` for async tests
- Create good mocking utilities
- Test small components in isolation

### Risk 3: Performance Regression
**Issue**: Stream overhead vs direct callbacks
**Mitigation**:
- Profile before and after changes
- Use efficient channel types (mpsc vs broadcast)
- Benchmark stream operations

## Success Criteria

1. ✅ All existing functionality works (focus tracking, font resizing)
2. ✅ Can run multiple independent event consumers
3. ✅ Type-safe event filtering with predicates
4. ✅ Clean separation between niri and kitty concerns
5. ✅ Comprehensive test coverage
6. ✅ No performance regressions
7. ✅ Documentation for public APIs
