# Architecture Design: Niri Registry with Hooks

## Problem Statement

Current `registry.rs` mixes too many concerns:
- `FocusTracker` - tracks currently focused window
- `KittyRegistry` - manages kitty connections and PID mapping
- Both tightly coupled to niri IPC

## Proposed Architecture: Hook-Based System

### Core Concepts

1. **Event Stream**: Async stream of window events from niri
2. **Registry**: Manages event stream and hooks
3. **Predicate**: Filter function for matching events
4. **Hook**: Callback attached to registry with optional predicate
5. **Handler**: Business logic for specific concerns (kitty resizing)

## Structure

```
src/
├── niri/
│   ├── mod.rs
│   ├── registry.rs              # Event stream + hook system
│   └── types.rs                # Event types (WindowInfo, FocusEvent, etc.)
├── kitty/
│   ├── mod.rs
│   ├── registry.rs              # Kitty PID mapping, key lookup
│   ├── resizer.rs              # Hook handler for font resizing
│   └── client.rs               # Kitty connection wrapper
└── main.rs                     # Wire everything together
```

## Niri Registry Design

### Event Types

```rust
pub enum NiriEvent {
    Focus { window_id: u64, window: WindowInfo },
    Blur { window_id: u64, window: WindowInfo },
    Create { window_id: u64, window: WindowInfo },
    Destroy { window_id: u64 },
}

pub struct WindowInfo {
    pub id: u64,
    pub app_id: Option<String>,
    pub pid: Option<i32>,
    // ... other fields from niri IPC
}
```

### Hook System

```rust
pub struct Hook<F, P> {
    predicate: P,
    handler: F,
}

impl<F, P> Hook<F, P>
where
    F: Fn(&NiriEvent) + Send + Sync,
    P: Fn(&WindowInfo) -> bool + Send + Sync,
{
    pub fn new(predicate: P, handler: F) -> Self {
        Self { predicate, handler }
    }

    pub fn matches(&self, event: &NiriEvent) -> bool {
        match event {
            NiriEvent::Focus { window, .. } => (self.predicate)(window),
            NiriEvent::Blur { window, .. } => (self.predicate)(window),
            _ => false,
        }
    }

    pub fn invoke(&self, event: &NiriEvent) {
        (self.handler)(event);
    }
}
```

### Registry API

```rust
pub struct NiriRegistry {
    hooks: Vec<Box<dyn DynHook>>,
}

impl NiriRegistry {
    pub fn new() -> Result<Self> {
        // Connect to niri IPC
        // Start event stream
    }

    pub fn add_hook<H, P>(&mut self, hook: Hook<H, P>)
    where
        H: Fn(&NiriEvent) + Send + Sync + 'static,
        P: Fn(&WindowInfo) -> bool + Send + Sync + 'static,
    {
        self.hooks.push(Box::new(hook));
    }

    pub async fn run(&mut self) -> Result<()> {
        while let Some(event) = self.next_event().await? {
            for hook in &self.hooks {
                if hook.matches(&event) {
                    hook.invoke(&event);
                }
            }
        }
    }
}
```

## Kitty Resizer Hook

```rust
pub struct KittyResizer {
    kitty_registry: KittyRegistry,
}

impl KittyResizer {
    pub fn new(kitty_registry: KittyRegistry) -> Self {
        Self { kitty_registry }
    }

    pub fn as_hook(&self) -> Hook<impl Fn(&NiriEvent), impl Fn(&WindowInfo) -> bool> {
        Hook::new(
            // Predicate: match kitty windows
            |window| window.app_id.as_deref() == Some("kitty"),
            // Handler: resize on focus/blur
            |event| {
                match event {
                    NiriEvent::Focus { window, .. } => {
                        if let Some(pid) = window.pid {
                            self.kitty_registry.increase_font_size(pid);
                        }
                    }
                    NiriEvent::Blur { window, .. } => {
                        if let Some(pid) = window.pid {
                            self.kitty_registry.decrease_font_size(pid);
                        }
                    }
                    _ => {}
                }
            }
        )
    }
}
```

## Usage in main.rs

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let mut niri_registry = NiriRegistry::new()?;
    let kitty_registry = KittyRegistry::new();

    // Create resizer and add as hook
    let resizer = KittyResizer::new(kitty_registry);
    niri_registry.add_hook(resizer.as_hook());

    // Could add more hooks for other purposes
    // e.g., logging hooks, monitoring hooks, etc.

    // Run the event loop
    niri_registry.run().await?;
}
```

## Benefits

1. **Separation of Concerns**
   - niri/ only knows about niri events
   - kitty/ only knows about kitty resizing
   - Main just wires them together

2. **Composability**
   - Easy to add new hooks without modifying core
   - Hooks can be added/removed at runtime
   - Predicates allow fine-grained filtering

3. **Testability**
   - Mock NiriRegistry for testing handlers
   - Test predicates independently
   - Test KittyRegistry with fake events

4. **Extensibility**
   - Add logging hook: `Hook::new(|_| true, log_event)`
   - Add stats hook: `Hook::new(|_| true, update_stats)`
   - Add notification hook with custom predicate

## Alternate: Async Stream Architecture

Instead of hook-based, use Rust streams:

```rust
pub struct NiriRegistry {
    event_stream: BoxStream<'static, NiriEvent>,
}

impl NiriRegistry {
    pub fn events(&self) -> impl Stream<Item = NiriEvent> + '_ {
        self.event_stream.clone()
    }

    pub fn filter<P>(stream: impl Stream<Item = NiriEvent>, predicate: P) -> impl Stream<Item = NiriEvent>
    where
        P: Fn(&NiriEvent) -> bool,
    {
        // Filter stream based on predicate
    }
}

// Usage
async fn run() -> Result<()> {
    let registry = NiriRegistry::new()?;

    let kitty_events = registry
        .events()
        .filter(|event| matches!(event, NiriEvent::Focus { window, .. } if window.app_id.as_deref() == Some("kitty")));

    tokio::pin!(tokio_stream::for_each(kitty_events, |event| {
        match event {
            NiriEvent::Focus { window, .. } => handle_focus(window.pid),
            NiriEvent::Blur { window, .. } => handle_blur(window.pid),
            _ => {}
        }
    })).await;

    Ok(())
}
```

### Stream Approach Tradeoffs

**Pros:**
- Very Rust-idiomatic (futures, streams)
- Composable with stream operators
- No callback hell

**Cons:**
- Requires cloning streams for multiple consumers
- More complex async/await syntax
- Harder to dynamically add/remove consumers

## Recommendation

**Start with hook-based system** for these reasons:

1. **Simpler to understand**: Hooks are more straightforward than streams
2. **Easier to manage**: Add/remove hooks dynamically is simple
3. **Clear semantics**: `add_hook(hook)` is obvious
4. **Good for our use case**: We have a few discrete handlers (resize, maybe logging)

**Future enhancement**: If we need more complex event processing, could refactor to streams later.
