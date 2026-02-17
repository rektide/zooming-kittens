# Kuchikiki DOM for Niri State Representation

Investigation into using [brave/kuchikiki](https://github.com/brave/kuchikiki) for representing and manipulating niri's window/column tree structure.

## Overview

Kuchikiki (口利き) is an HTML tree-manipulation library for Rust, forked from the unmaintained Kuchiki. While designed for HTML/DOM, its tree structure is general enough to represent arbitrary hierarchical data—including niri's workspace/column/window hierarchy.

### Why a DOM-like Tree?

1. **Unified State**: Single tree representing all workspaces, columns, and windows
2. **Rich Traversal**: Built-in iterators for ancestors, descendants, siblings
3. **CSS Selectors**: Query nodes using familiar selector syntax
4. **Tree Manipulation**: `append`, `prepend`, `insert_before/after`, `detach`
5. **Event-like Queries**: Find focused windows with `:focus` pseudo-class

---

# Tech Stack Review

| Component | Purpose | Notes |
|-----------|---------|-------|
| `NodeRef` | Strong reference to node | `Rc<Node>` wrapper |
| `NodeData` | Node content | Element, Text, Document, etc. |
| `ElementData` | Element attributes | Name + attribute map |
| `Attributes` | Key-value store | Could store window properties |
| `NodeIterator` | Tree traversal traits | ancestors, descendants, siblings |
| `Selectors` | CSS selector matching | Query nodes by criteria |

### Dependencies

From [`Cargo.toml`](file:///home/rektide/archive/kuchikiki/Cargo.toml):
- `cssparser` - CSS parsing
- `html5ever` - HTML parsing (we'd mostly ignore this)
- `selectors` - CSS selector matching
- `indexmap` - Ordered hash map

---

# Journal - Tree Structure Analysis

From [`src/tree.rs`](file:///home/rektide/archive/kuchikiki/src/tree.rs):

## Node Types

```rust
pub enum NodeData {
    Element(ElementData),     // Our columns/windows
    Text(RefCell<String>),    // Not needed
    Comment(RefCell<String>), // Not needed
    ProcessingInstruction,    // Not needed
    Doctype(Doctype),         // Not needed
    Document(DocumentData),   // Root of tree
    DocumentFragment,         // Not needed
}
```

We'd primarily use:
- `Document` as the root (represents "all of niri")
- `Element` for workspaces, columns, windows

## Element Structure

```rust
pub struct ElementData {
    pub name: QualName,                    // Tag name: "workspace", "column", "window"
    pub attributes: RefCell<Attributes>,   // Properties: id, height, app_id, etc.
    pub template_contents: Option<NodeRef>,// Not used
}
```

Attributes can store:
- `id`: Window/column ID
- `height`: Current/original height
- `app-id`: Application identifier
- `workspace-id`: Parent workspace
- `focused`: Whether this node is focused
- `original-height`: Stored before adjustment

---

# Journal - Tree Traversal API

From [`src/iter.rs`](file:///home/rektide/archive/kuchikiki/src/iter.rs):

## Available Iterators

```rust
impl NodeRef {
    // Ancestor traversal
    pub fn inclusive_ancestors(&self) -> Ancestors;
    pub fn ancestors(&self) -> Ancestors;
    
    // Sibling traversal  
    pub fn inclusive_preceding_siblings(&self) -> Rev<Siblings>;
    pub fn preceding_siblings(&self) -> Rev<Siblings>;
    pub fn inclusive_following_siblings(&self) -> Siblings;
    pub fn following_siblings(&self) -> Siblings;
    
    // Children
    pub fn children(&self) -> Siblings;
    
    // Descendants (depth-first)
    pub fn inclusive_descendants(&self) -> Descendants;
    pub fn descendants(&self) -> Descendants;
    
    // Edge-based traversal (start/end markers)
    pub fn traverse_inclusive(&self) -> Traverse;
    pub fn traverse(&self) -> Traverse;
}
```

## Filtered Iterators

```rust
// From NodeIterator trait
trait NodeIterator: Iterator<Item = NodeRef> {
    fn elements(self) -> ElementIterator<Self>;
    fn text_nodes(self) -> TextNodeIterator<Self>;
    fn select(self, selectors: &str) -> Result<Select<Self>, ()>;
}
```

---

# Journal - Tree Manipulation API

From [`src/tree.rs:402-489`](file:///home/rektide/archive/kuchikiki/src/tree.rs):

## Core Operations

```rust
impl NodeRef {
    /// Append child after existing children
    pub fn append(&self, new_child: NodeRef);
    
    /// Prepend child before existing children
    pub fn prepend(&self, new_child: NodeRef);
    
    /// Insert sibling after this node
    pub fn insert_after(&self, new_sibling: NodeRef);
    
    /// Insert sibling before this node  
    pub fn insert_before(&self, new_sibling: NodeRef);
}

impl Node {
    /// Remove from parent/siblings (children not affected)
    pub fn detach(&self);
}
```

---

# Journal - CSS Selector Support

From [`src/select.rs`](file:///home/rektide/archive/kuchikiki/src/select.rs):

## Supported Pseudo-Classes

```rust
pub enum PseudoClass {
    AnyLink,
    Link,
    Visited,
    Active,
    Focus,      // ← Useful for focused window!
    Hover,
    Enabled,
    Disabled,
    Checked,
    Indeterminate,
}
```

## Selector Examples

```rust
// Find focused window
let focused = document.select("window[focused]").unwrap();

// Find all windows in column 3
let windows = document.select("column:nth-child(3) > window").unwrap();

// Find windows by app-id
let kitty = document.select("window[app-id='kitty']").unwrap();

// Find active workspace's columns
let cols = document.select("workspace[focused] > column").unwrap();
```

---

# Proposed Architecture

## Niri State Tree

```
Document (niri root)
├── Element "workspace" [id=1, focused=true]
│   ├── Element "column" [idx=0]
│   │   ├── Element "window" [id=123, app-id="kitty", height=400]
│   │   └── Element "window" [id=124, app-id="firefox", height=300]
│   └── Element "column" [idx=1]
│       └── Element "window" [id=125, app-id="code", height=700]
└── Element "workspace" [id=2]
    └── Element "column" [idx=0]
        └── Element "window" [id=126, app-id="terminal", height=500]
```

## Mapping Niri IPC → Kuchikiki

| Niri Concept | Kuchikiki Node | Attributes |
|--------------|----------------|------------|
| Niri instance | `Document` | — |
| Workspace | `Element("workspace")` | `id`, `name`, `focused` |
| Column | `Element("column")` | `idx`, `workspace-id` |
| Window | `Element("window")` | `id`, `app-id`, `title`, `height`, `original-height`, `focused` |

## Integration with Taffy

The tree structure from kuchikiki can feed taffy for layout:

```rust
fn column_to_taffy(column_node: &NodeRef) -> (TaffyTree<()>, NodeId) {
    let mut taffy = TaffyTree::new();
    
    let children: Vec<NodeId> = column_node.children()
        .elements()
        .filter_map(|win| {
            let height = win.as_element()?.attributes.borrow()
                .get("original-height")
                .and_then(|v| v.parse().ok())?;
            
            Some(taffy.new_leaf(Style {
                size: Size { width: auto(), height: length(height) },
                flex_grow: 1.0,
                ..Default::default()
            }).ok()?)
        })
        .collect();
    
    let root = taffy.new_with_children(
        Style { flex_direction: FlexDirection::Column, ..Default::default() },
        &children,
    ).unwrap();
    
    (taffy, root)
}
```

---

# Implementation Sketch

```rust
use kuchikiki::traits::*;
use kuchikiki::{NodeRef, NodeData, ElementData, QualName, Attribute, ExpandedName};

pub struct NiriStateTree {
    root: NodeRef,
}

impl NiriStateTree {
    pub fn new() -> Self {
        Self {
            root: NodeRef::new_document(),
        }
    }
    
    pub fn add_workspace(&self, id: u64, name: &str, focused: bool) -> NodeRef {
        let ws = NodeRef::new_element(
            QualName::new(None, ns!(), local_name!("workspace")),
            vec![
                (expanded_name!("", "id"), Attribute { value: id.to_string() }),
                (expanded_name!("", "name"), Attribute { value: name.to_string() }),
                (expanded_name!("", "focused"), Attribute { value: focused.to_string() }),
            ],
        );
        self.root.append(ws.clone());
        ws
    }
    
    pub fn add_column(&self, workspace: &NodeRef, idx: usize) -> NodeRef {
        let col = NodeRef::new_element(
            QualName::new(None, ns!(), local_name!("column")),
            vec![
                (expanded_name!("", "idx"), Attribute { value: idx.to_string() }),
            ],
        );
        workspace.append(col.clone());
        col
    }
    
    pub fn add_window(
        &self,
        column: &NodeRef,
        id: u64,
        app_id: &str,
        height: f64,
        focused: bool,
    ) -> NodeRef {
        let win = NodeRef::new_element(
            QualName::new(None, ns!(), local_name!("window")),
            vec![
                (expanded_name!("", "id"), Attribute { value: id.to_string() }),
                (expanded_name!("", "app-id"), Attribute { value: app_id.to_string() }),
                (expanded_name!("", "height"), Attribute { value: height.to_string() }),
                (expanded_name!("", "original-height"), Attribute { value: height.to_string() }),
                (expanded_name!("", "focused"), Attribute { value: focused.to_string() }),
            ],
        );
        column.append(win.clone());
        win
    }
    
    pub fn find_focused_window(&self) -> Option<NodeRef> {
        self.root.select("window[focused='true']").ok()?.next()
    }
    
    pub fn get_column_windows(&self, column: &NodeRef) -> Vec<NodeRef> {
        column.children().elements().collect()
    }
    
    pub fn update_window_height(&self, window: &NodeRef, new_height: f64) {
        if let Some(elem) = window.as_element() {
            elem.attributes.borrow_mut()
                .insert("height".into(), Attribute { value: new_height.to_string() });
        }
    }
    
    pub fn get_windows_by_app(&self, app_id: &str) -> Vec<NodeRef> {
        self.root.select(&format!("window[app-id='{}']", app_id))
            .ok()
            .map(|iter| iter.collect())
            .unwrap_or_default()
    }
}
```

---

# API Surfaces

## NodeRef Methods

| Method | Returns | Purpose |
|--------|---------|---------|
| `data()` | `&NodeData` | Access node content |
| `as_element()` | `Option<&ElementData>` | Get element data |
| `parent()` | `Option<NodeRef>` | Parent node |
| `first_child()` / `last_child()` | `Option<NodeRef>` | Children |
| `previous_sibling()` / `next_sibling()` | `Option<NodeRef>` | Siblings |
| `children()` | `Siblings` | Child iterator |
| `descendants()` | `Descendants` | All descendants |
| `select(selector)` | `Result<Select, ()>` | CSS selector query |
| `append(child)` | `()` | Add child |
| `detach()` | `()` | Remove from tree |

## ElementData Methods

| Method | Returns | Purpose |
|--------|---------|---------|
| `name` | `QualName` | Tag name |
| `attributes.borrow()` | `Ref<Attributes>` | Get attributes |
| `attributes.borrow_mut()` | `RefMut<Attributes>` | Modify attributes |

---

# Journal - Connecting to Longcat

The current [`longcat.rs`](../../src/commands/longcat.rs) manages window heights with:

```rust
struct ColumnState {
    window_heights: HashMap<u64, f64>,
    last_height_change: Option<Instant>,
}
```

With kuchikiki + taffy:

1. **State Tree**: Kuchikiki maintains the window/column hierarchy
2. **Layout Engine**: Taffy computes heights based on flex properties
3. **Attribute Storage**: Original heights stored as node attributes
4. **Queries**: Find focused window, column siblings via CSS selectors

### Workflow

```
1. Niri Event → Update kuchikiki tree
2. Focus change → Query focused window, find column
3. Column's windows → Build taffy tree
4. Taffy compute → Get new heights
5. Apply heights → Send to niri via IPC
```

---

# Journal - Attribute Storage

Using element attributes for metadata:

```rust
// Store original height
elem.attributes.borrow_mut().insert(
    ExpandedName::new("", "original-height"),
    Attribute { value: "400.0".to_string() }
);

// Retrieve
let orig = elem.attributes.borrow()
    .get("original-height")
    .and_then(|v| v.parse::<f64>().ok());
```

This replaces the `HashMap<u64, f64>` in `ColumnState`.

---

# Options and Alternatives

## Option A: Full Kuchikiki Integration

Replace all state tracking with DOM tree:
- ✅ Single source of truth
- ✅ Rich traversal/query
- ❌ Heavier than HashMap
- ❌ Learning curve

## Option B: Hybrid State

Keep existing state, add kuchikiki for queries:
- ✅ Minimal change
- ✅ Query capabilities
- ❌ State duplication
- ❌ Sync complexity

## Option C: Custom Tree

Build our own tree structure:
- ✅ Tailored to niri
- ❌ Reimplement traversal
- ❌ No selector support

---

# Decision Points

1. **Element Names**: Use descriptive names (`workspace`, `column`, `window`) or generic (`div`)?
   - Recommendation: Descriptive for clarity

2. **Attribute Types**: String-only or parse on access?
   - Recommendation: Helper methods that parse

3. **Tree Scope**: Per-workspace, per-monitor, or global?
   - Recommendation: Global tree, query as needed

4. **Sync Strategy**: Event-driven updates or polling rebuild?
   - Recommendation: Event-driven (niri IPC events)

---

# Discussion Questions

1. Should we use kuchikiki's HTML parser or build tree programmatically?
2. How to handle node identity across niri window ID changes?
3. Should attributes store computed taffy heights or just originals?
4. Can we use CSS selectors for complex window filtering?

---

# Reference Materials

- [`kuchikiki/README.md`](file:///home/rektide/archive/kuchikiki/README.md)
- [`kuchikiki/src/tree.rs`](file:///home/rektide/archive/kuchikiki/src/tree.rs) - Tree structure
- [`kuchikiki/src/iter.rs`](file:///home/rektide/archive/kuchikiki/src/iter.rs) - Traversal
- [`kuchikiki/src/select.rs`](file:///home/rektide/archive/kuchikiki/src/select.rs) - Selectors
- [`kuchikiki/src/attributes.rs`](file:///home/rektide/archive/kuchikiki/src/attributes.rs) - Attributes
- [docs.rs/kuchikiki](https://docs.rs/kuchikiki) - API docs

---

# Journal - Combined Architecture Vision

```
┌─────────────────────────────────────────────────────────────┐
│                     Niri IPC Events                         │
│  (WindowOpened, WindowClosed, FocusChanged, WorkspaceChanged)│
└────────────────────────┬────────────────────────────────────┘
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                   Kuchikiki State Tree                      │
│  ┌─ Document (niri)                                         │
│  │  ├─ workspace[id=1, focused]                            │
│  │  │  ├─ column[idx=0]                                    │
│  │  │  │  ├─ window[id=123, original-height=400, focused]  │
│  │  │  │  └─ window[id=124, original-height=300]           │
│  │  │  └─ column[idx=1]                                    │
│  │  └─ workspace[id=2]                                     │
└────────────────────────┬────────────────────────────────────┘
                         │ query
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                  Taffy Layout Engine                        │
│  Input: Column node → Extract windows                       │
│  Compute: Flexbox with flex_grow/shrink, min-height         │
│  Output: Computed heights per window                        │
└────────────────────────┬────────────────────────────────────┘
                         │ apply
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                     Niri IPC Actions                        │
│           SetWindowHeight { id, change: SetFixed(h) }       │
└─────────────────────────────────────────────────────────────┘
```

---

# Journal - Next Steps

1. Prototype `NiriStateTree` wrapper
2. Implement IPC event → tree update mapping
3. Integrate taffy layout computation
4. Benchmark query performance
5. Consider serialization for state persistence
