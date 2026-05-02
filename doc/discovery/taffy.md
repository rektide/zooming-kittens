# Taffy Flexbox for Window Height Management

Investigation into using [DioxusLabs/taffy](https://github.com/DioxusLabs/taffy) for modeling vertical window splits in niri.

## Overview

Taffy is a high-performance UI layout library written in Rust that implements CSS **Block**, **Flexbox**, and **CSS Grid** layout algorithms. It's used by Servo, Blitz, Bevy, Lapce/Floem, and Zed/GPUI.

### Core Concept: Transpose Model

Our use case is unique: we have a **fixed-height vertical column** of windows (niri's column layout), but want to use flexbox algorithms to control relative heights. The insight:

1. **Physical Reality**: Niri columns have fixed height (screen height minus bars/gaps)
2. **Flexbox Transposition**: Treat this as a fixed-width flexbox container with `FlexDirection::Column`
3. **Benefit**: Get flexbox's `flex_grow`, `flex_shrink`, `min_size`, `max_size`, and `gap` constraints for free

---

# Tech Stack Review

| Component | Purpose | Notes |
|-----------|---------|-------|
| `TaffyTree` | Node tree storage | Slotmap-based, efficient cache |
| `Style` struct | CSS-like styling | All flex/grid/block properties |
| `compute_layout` | Layout algorithm | Returns `Layout` with x/y/width/height |
| `Dimension` | Size values | `Length`, `Percent`, `Auto` |

---

# Journal - Initial Investigation

Started by examining the existing [`longcat.rs`](../../src/commands/longcat.rs) implementation to understand the current approach.

## Current Implementation Analysis

The `Longcat` struct manages window heights in niri columns:

1. **Column Identification**: Windows grouped by `(workspace_id, column_idx)`
2. **Height Tracking**: `ColumnState.window_heights: HashMap<u64, f64>` stores original heights
3. **Proportion System**: `active_height_proportion` (0.7) and `other_height_proportion` (0.3)
4. **Manual Resize Detection**: Debounced check for user-initiated resizes
5. **Height Calculation**: Custom proportion math with `minimum_height` clamping

Key limitations of current approach:
- Proportions are applied uniformly (no per-window weights)
- Two-group model: active vs. others (no fine-grained control)
- Manual minimum height handling
- No concept of flex_grow/flex_shrink ratios

---

# Journal - Taffy API Exploration

Explored taffy's source in [`~/archive/DioxusLabs/taffy/`](file:///home/rektide/archive/DioxusLabs/taffy/).

## High-Level API

```rust
use taffy::prelude::*;

let mut tree: TaffyTree<()> = TaffyTree::new();

// Create leaf nodes (windows)
let window1 = tree.new_leaf(Style {
    size: Size { width: length(800.0), height: auto() },
    flex_grow: 1.0,  // Take available space
    ..Default::default()
})?;

let window2 = tree.new_leaf(Style {
    size: Size { width: length(800.0), height: length(200.0) },
    min_size: Size { width: auto(), height: length(60.0) },  // Minimum height!
    ..Default::default()
})?;

// Column container
let column = tree.new_with_children(
    Style {
        flex_direction: FlexDirection::Column,
        size: Size { width: length(800.0), height: length(1080.0) },  // Fixed column height
        ..Default::default()
    },
    &[window1, window2],
)?;

// Compute layout
tree.compute_layout(column, Size::MAX_CONTENT)?;

// Read results
let layout = tree.layout(window1)?;
println!("Height: {}", layout.size.height);
```

## Key Style Properties for Our Use Case

From [`src/style/mod.rs`](file:///home/rektide/archive/DioxusLabs/taffy/src/style/mod.rs):

| Property | Type | Our Usage |
|----------|------|-----------|
| `flex_direction` | `FlexDirection` | `Column` for vertical stacking |
| `flex_grow` | `f32` | Active window could have higher grow |
| `flex_shrink` | `f32` | Windows that can shrink |
| `flex_basis` | `Dimension` | Initial/preferred height |
| `min_size` | `Size<Dimension>` | Minimum window height (no crushing) |
| `max_size` | `Size<Dimension>` | Maximum window height |
| `size` | `Size<Dimension>` | Explicit height (for fixed windows) |
| `gap` | `Size<LengthPercentage>` | Gaps between windows! |
| `padding` | `Rect<LengthPercentage>` | Column margins |

---

# Journal - Flex Direction Deep Dive

From [`src/style/flex.rs`](file:///home/rektide/archive/DioxusLabs/taffy/src/style/flex.rs):

```rust
pub enum FlexDirection {
    Row,           // +x main axis
    Column,        // +y main axis  <-- WE USE THIS
    RowReverse,    // -x main axis
    ColumnReverse, // -y main axis
}
```

For our niri column model:
- **Container**: `FlexDirection::Column`, fixed height (screen height)
- **Children**: Windows stacked vertically
- **Main axis**: Vertical (height distribution)
- **Cross axis**: Horizontal (width - typically uniform or ignored)

---

# Journal - Dimension Types

From [`src/style/dimension.rs`](file:///home/rektide/archive/DioxusLabs/taffy/src/style/dimension.rs):

```rust
pub enum Dimension {
    Auto,           // Let flexbox decide
    Length(f32),    // Fixed pixels
    Percent(f32),   // Percentage of parent
}
```

This maps well to niri's `SizeChange`:
- `SizeChange::SetFixed(height)` → `Dimension::Length(height as f32)`
- Original proportions → Could use `flex_grow` ratios

---

# Proposed Architecture

## Window → Taffy Node Mapping

```
Niri Column                    Taffy Tree
────────────                   ──────────
Column (workspace, idx)   →    Root node (FlexDirection::Column, fixed height)
Window 1 (id, height)     →    Child node (flex_grow based on original height)
Window 2 (id, height)     →    Child node
Window 3 (id, height)     →    Child node
```

## Focus-Aware Flex Grow

When window is focused:

```rust
fn compute_flex_grow(window: &Window, is_focused: bool, total_original: f64) -> f32 {
    let base_ratio = window.original_height / total_original;
    if is_focused {
        // Active window: higher proportion (e.g., 0.7 total)
        (base_ratio * 2.33) as f32  // Scale factor for 70/30 split
    } else {
        // Other windows: lower proportion (e.g., 0.3 total shared)
        (base_ratio * 0.43) as f32  // Scale factor for remaining 30%
    }
}
```

## Implementation Sketch

```rust
use taffy::prelude::*;

struct ColumnLayout {
    tree: TaffyTree<WindowId>,
    root: NodeId,
    nodes: HashMap<WindowId, NodeId>,
}

impl ColumnLayout {
    fn new(column_height: f32) -> Self {
        let mut tree = TaffyTree::new();
        let root = tree.new_with_children(
            Style {
                flex_direction: FlexDirection::Column,
                size: Size { width: auto(), height: length(column_height) },
                gap: Size { width: zero(), height: length(4.0) },  // Window gaps!
                ..Default::default()
            },
            &[],
        ).unwrap();
        
        Self { tree, root, nodes: HashMap::new() }
    }
    
    fn add_window(&mut self, window_id: WindowId, original_height: f32) {
        let node = self.tree.new_leaf(
            Style {
                size: Size { width: auto(), height: auto() },
                min_size: Size { width: auto(), height: length(60.0) },  // min-height!
                flex_grow: 1.0,
                ..Default::default()
            },
        ).unwrap();
        
        self.tree.add_child(self.root, node).unwrap();
        self.nodes.insert(window_id, node);
    }
    
    fn set_focus(&mut self, focused_id: WindowId) {
        let total_nodes = self.nodes.len();
        let other_count = total_nodes.saturating_sub(1);
        
        for (&wid, &node) in &self.nodes {
            let (grow, shrink) = if wid == focused_id {
                (3.0, 0.5)  // Active: grow more, shrink less
            } else {
                (0.5, 1.0)   // Others: grow less, shrink more
            };
            
            self.tree.set_style(node, Style {
                flex_grow: grow,
                flex_shrink: shrink,
                ..Default::default()
            }).ok();
        }
    }
    
    fn compute(&mut self) -> HashMap<WindowId, f32> {
        self.tree.compute_layout(self.root, Size::MAX_CONTENT).ok();
        
        self.nodes.iter()
            .filter_map(|(&wid, &node)| {
                self.tree.layout(node).ok().map(|l| (wid, l.size.height))
            })
            .collect()
    }
}
```

---

# API Surfaces

## TaffyTree Methods

| Method | Purpose |
|--------|---------|
| `new()` | Create empty tree |
| `new_leaf(style)` | Create childless node |
| `new_with_children(style, &[NodeId])` | Create node with children |
| `add_child(parent, child)` | Add child to existing node |
| `remove_child(parent, child)` | Remove child |
| `set_style(node, style)` | Update node's style |
| `layout(node)` | Get computed `Layout` |
| `compute_layout(root, available)` | Run layout algorithm |

## Layout Output

```rust
pub struct Layout {
    pub order: u32,
    pub size: Size<f32>,      // width, height
    pub location: Point<f32>, // x, y
    pub scrollbar_width: f32,
    pub border: Rect<f32>,
    pub padding: Rect<f32>,
}
```

---

# Journal - Feature Flags

From [`Cargo.toml`](file:///home/rektide/archive/DioxusLabs/taffy/Cargo.toml):

```toml
[features]
default = ["std", "taffy_tree", "flexbox", "grid", "block_layout", "float_layout", "calc", "content_size", "detailed_layout_info"]
flexbox = []           # We need this
grid = []              # Optional, likely not needed
block_layout = []      # Not needed
taffy_tree = []        # We need this
std = []               # Recommended
serde = []             # For config serialization
```

Minimal dependency:
```toml
[dependencies]
taffy = { version = "0.9", default-features = false, features = ["std", "taffy_tree", "flexbox"] }
```

---

# Options and Alternatives

## Option A: Full Taffy Integration

Replace entire `ColumnState` with `TaffyTree`:
- ✅ Native flex_grow/flex_shrink
- ✅ Built-in min_size/max_size
- ✅ Gap support
- ❌ New dependency
- ❌ Need to sync tree with niri window events

## Option B: Hybrid Approach

Keep `ColumnState` for tracking, use taffy only for computation:
- ✅ Minimal change to existing code
- ✅ Taffy handles complex constraint math
- ❌ Redundant state

## Option C: Custom Algorithm

Continue current approach but refine:
- ✅ No new dependencies
- ❌ Reimplementing flexbox poorly
- ❌ Hard to add features like per-window weights

---

# Decision Points

1. **Tree Lifetime**: Create new tree per focus change, or maintain persistent tree?
   - Recommendation: Persistent tree, update styles on focus

2. **Window Addition/Removal**: Handle niri's dynamic window lifecycle
   - Use `add_child`/`remove_child` APIs

3. **Gap Handling**: Use taffy's `gap` or let niri handle?
   - Recommendation: Let taffy compute gaps, apply via niri's column gaps config

4. **Performance**: Is taffy fast enough for interactive focus changes?
   - Benchmarks show ~0.5ms for 1000 nodes; we have ~3-10 windows

---

# Discussion Questions

1. Should we preserve the "original height" concept, or let flexbox fully control?
2. How to handle manual resizes? Update `flex_basis` or set fixed `size`?
3. Should active window use `flex_grow` or a larger `flex_basis`?
4. Integration with niri's existing column gap/stride settings?

---

# Reference Materials

- [`longcat.rs`](../../src/commands/longcat.rs) - Current implementation
- [`taffy/README.md`](file:///home/rektide/archive/DioxusLabs/taffy/README.md)
- [`taffy/examples/basic.rs`](file:///home/rektide/archive/DioxusLabs/taffy/examples/basic.rs)
- [`taffy/examples/flexbox_gap.rs`](file:///home/rektide/archive/DioxusLabs/taffy/examples/flexbox_gap.rs)
- [MDN Flexbox Guide](https://css-tricks.com/snippets/css/a-guide-to-flexbox/)
- [Flexbox Froggy](https://flexboxfroggy.com/) - Interactive tutorial

---

# Journal - Next Steps

1. Prototype integration in a test module
2. Benchmark layout computation time
3. Decide on tree management strategy
4. Write migration plan from current `ColumnState`

---

# Appendix A — Refinement Prompt

Use the following prompt to drive the next design session. It assumes the reader has this document and the current `longcat.rs` source.

---

You are designing the integration of taffy's flexbox layout engine into zooming-kittens' longcat command. The goal: replace the flat `ColumnState` HashMap with a `TaffyTree<WindowMeta>` that models the full workspace → column → window hierarchy, enabling unified state, cross-column coordination, and structured status output.

The existing discovery work is in `doc/discovery/taffy.md`. The current implementation is in `src/commands/longcat.rs`. The niri event registry is in `src/niri/registry.rs` and event types in `src/niri/types.rs`.

Answer the following questions with concrete decisions and code sketches where helpful:

## Tree Lifecycle

The current longcat fetches all windows from niri IPC on every focus event (`get_windows()` at longcat.rs:68). The `TaffyTree` needs to stay in sync with niri's actual window state.

- Do we build the tree once from an initial `Request::Windows` snapshot, then mutate it on events? Or rebuild from IPC on every event?
- The niri registry already emits `NiriEvent::Create`, `NiriEvent::Destroy`, and receives `Event::WindowsChanged` (registry.rs:163). Which events should drive tree mutations?
- What happens if events arrive out of order or are missed? Do we need periodic reconciliation?

## WindowMeta Context

`TaffyTree<T>` stores a generic context per node. What fields does `WindowMeta` need?

- Minimum: `window_id: u64, original_height: f64`
- Do we need `app_id: String`? (for zk-status-json structured output)
- Do we need `is_focused: bool`? Or do we track the focused node id separately?
- Do workspace and column nodes need their own context type, or share `WindowMeta` with unused fields?
- Consider: taffy node context is `Option<T>` — nodes can have no context. We could use `()` for the root, a workspace struct for workspaces, and a window struct for windows.

## Flex Grow Strategy

The current code uses `active_height_proportion` (0.7) and `other_height_proportion` (0.3) as a two-group model. Taffy gives us `flex_grow`, `flex_shrink`, `flex_basis`, `min_size`, `max_size`.

- How do we achieve the 70/30 split with flex properties? Concrete: if 3 windows with original heights [400, 300, 200] and the first is focused, what `flex_grow` values produce 70% of 900=630 for window 1 and 30% of 900=270 shared between windows 2-3?
- Do we set `flex_basis` to original height and use `flex_grow`/`flex_shrink` for the redistribution? Or do we use `flex_grow` alone with `flex_basis: auto`?
- The current code clamps to `minimum_height` with manual redistribution logic. Does taffy's `min_size` handle this correctly, or do we still need post-computation adjustment?
- What happens when the user manually resizes a window? The current code detects this via debounced height comparison and updates proportions. How does this work with taffy — do we update `flex_basis`? `flex_grow`? The node context's `original_height`?

## Column Height Source

Taffy needs a fixed container height for the column node. The current longcat never explicitly knows the column height.

- Can we get screen/monitor height from niri IPC? Check `niri_ipc` types for monitor config.
- Or compute it as the sum of all window heights in the column at rest (before longcat adjustment)?
- Does niri report the column height or total available space anywhere in the `Window` struct's `layout` fields?

## Event Integration

The longcat event loop at longcat.rs:604-632 only handles `Focus` and `Blur`. But `NiriEvent` also defines `Create` and `Destroy`. The niri registry receives `Event::WindowsChanged` but only uses it for initial focus detection.

- Which events do we need to handle to keep the taffy tree in sync?
- `Create` → add node to tree? What if we don't know which column yet?
- `Destroy` → remove node from tree?
- `Focus`/`Blur` → update flex properties + recompute?
- `Event::WindowsChanged` → full reconciliation? Or ignore since we track individual events?
- The `_handle_window_layout_changed` method at longcat.rs:331 is unused. What niri event should trigger it? Does niri fire a specific event when a window moves between columns?

## Migration Path

Can we introduce taffy incrementally, or does it need to replace ColumnState entirely in one shot?

- Option: keep ColumnState for original height tracking, use taffy only for layout computation. This is the "hybrid" approach from the options above.
- Option: replace ColumnState entirely with the taffy tree + WindowMeta context.
- What's the smallest change that lets us start using taffy for the active window height computation?

## Structured Status Output

Ticket zk-status-json wants structured JSON for focus events. The taffy tree would be the query source.

- Walk the tree and serialize all nodes' `Layout` + `WindowMeta`?
- Or just the focused column's windows?
- What's the schema? Can we derive it from the tree walk?

## Thread Safety

`TaffyTree` is `!Sync` (uses `&mut self` for most operations). The current longcat uses `DashMap` for concurrent access.

- The longcat event loop is single-threaded (one `tokio::select!` loop). Do we actually need concurrent access?
- If yes (e.g., status queries from another task), what's the right wrapping? `Arc<Mutex<TaffyTree>>`? `Arc<RwLock<TaffyTree>>`?

---
