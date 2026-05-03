---
type: Investigation
title: Umbriel IPC and Longcat parity research for zooming-kittens
description: Source-grounded assessment of Umbriel's IPC/actions/layout model against zooming-kittens' Longcat concerns, with live-session ground truth, a proposed narrow Rust client boundary, and a next experiment.
resource: design/umbriel/investigation0.glm53max.md
status: draft
tags: [umbriel, ipc, longcat, window-management, research]
generated:
  at: 2026-10-13
  by: model:zai-coding-plan/glm-5.3#max
sources:
  - resource: opencode://ses_ecc154b5fffeUhXQiZBaZOjPB3
    title: Umbriel source inspection, live IPC ground-truth queries, and Longcat feasibility mapping
  - resource: /home/rektide/archive/noctalia-dev/umbriel
    title: Umbriel compositor checkout (v0.1.0, 2026-10-10 tree) — src/server, src/layout, src/workspace, docs/user
  - resource: src/commands/longcat.rs
    title: zk Longcat controller over niri IPC (comparison anchor)
  - resource: src/niri/registry.rs
    title: zk niri event registry (comparison anchor)
  - resource: /home/rektide/src/tmux-interface-control/README.md
    title: Revamped tmux control library (bridging option)
---

# Umbriel IPC and Longcat parity research

## Situation and question

The user replaced niri with Umbriel (`~/archive/noctalia-dev/umbriel`, a live
checkout at `/run/user/1000/umbriel-wayland-0.sock`). zooming-kittens' Longcat
feature — focus-driven height expansion within a scrolling-layout column, with
baseline restore and manual-resize accommodation — is currently implemented
only against niri IPC. This wave determines, from Umbriel's implementation and
not from its project name: what the IPC actually offers, how each Longcat
concern maps, whether a Rust client library is justified and how narrow it
should be, and what the best first experiment is. Research only; nothing here
implements or modifies Umbriel.

## Project identity and posture

- Umbriel is a **C++23 Wayland compositor on wlroots 0.20** (+ its `umbrielfx`
  fork of SceneFX), from noctalia-dev, `VERSION` = `0.1.0`. Single-threaded
  event loop (`SCOPE.md`: "Threading the event loop" is out of scope). Meson
  build; **no Rust anywhere in the tree** — no client library exists beyond the
  compositor's own `umbriel` CLI.
- `SCOPE.md` is directly relevant: no plugins, no embedded scripting; "If a
  client, a shell, or a script over `umbriel msg` can do it, that is where it
  belongs." A zk-style external controller over IPC is the *philosophically
  blessed* extension point. Also in scope: "IPC actions and inspection output
  for state it already tracks" — good news for small additive asks (below).
- Layouts: scrolling (niri-like column strip with stacked windows), dwindle,
  master — per workspace. README warns keys/behavior may change between
  releases; there is **no IPC version negotiation at all** (no version request,
  field, or handshake anywhere in `src/server/ipc*.cpp`).

## IPC architecture (implementation-grounded)

Transport: one UNIX stream socket, path
`$XDG_RUNTIME_DIR/umbriel-$WAYLAND_DISPLAY.sock` (`src/server/ipc.cpp:112`),
`UMBRIEL_SOCKET` override honored by the CLI client
(`src/cli/ipc_client.cpp:21`). JSON, one object per line: requests
`{"cmd":...}`; replies `{"ok":...}` or `{"err":"..."}`. Request cap 64 KiB;
outbound backlog cap 256 KiB with eviction of non-draining subscribers; a
connection that sends nothing is dropped after 1 s; the deadline is removed
once `subscribe` is accepted (`ipc.cpp:308-312, 369-372`).

- **Queries** (`src/server/ipc_commands.cpp:616-629`): `windows`, `workspaces`,
  `submap`, `layers`, `color`, `tearing`, `keyboard-layouts`, `msg`, plus
  headless `output-create`/`output-destroy`.
- **Actions**: `{"cmd":"msg","arg":"<action>"}` runs the *same* action parser
  as keybinds (`IpcCommands::msg` → `parseAction` → `executeKeybindAction`,
  `ipc_commands.cpp:586-597`; dispatch `src/server/server_keybinds.cpp:104`).
  Full action table: `docs/user/actions.md` and
  `src/config/keybind_parse.cpp:230-345`. Reply is synchronous
  `{"ok":null}` or `{"err":"..."}` naming the requirement.
- **Event stream**: `{"cmd":"subscribe","events":[...]}`. Families: `theme`,
  `overview`, `keyboard_layout`, `windows`, `workspaces`, `submap`
  (`src/server/ipc.h:34`). Unknown family → immediate `err` + close.
  Subscribe writes the **current full state of each family first**, then one
  line per change. Every family payload is a **full snapshot** — the same JSON
  the matching query returns (`ipc.cpp:89-99` reuses `IpcCommands::windows` /
  `workspaces`, so stream and query "can never diverge"). Snapshots are
  coalesced per event-loop iteration via `wl_event_loop_add_idle`
  (`src/server/server_events.cpp:1467-1483`) and byte-deduplicated against the
  last broadcast (`ipc.cpp:418-428`), so "a resize that ends where it started
  wakes nobody".
- **CLI is the reference client** (`src/cli/ipc_client.cpp`): plain socket +
  line JSON, 2 s receive timeout for one-shots, none for streams. `umbriel
  subscribe windows,workspaces` relays lines unparsed. No schemas, no codegen;
  the wire format is hand-built nlohmann JSON.

### Live ground truth (read-only queries, 2026-10-13)

`{"cmd":"windows"}` against the running session confirmed the source reading
and added facts:

- Window `id` is a **32-hex-char string** — the ext-foreign-toplevel
  identifier, reused verbatim as IPC identity (`src/view/view.h:87-91`),
  stable for the window's map lifetime.
- Workspace `id` is a stable `"<output>:<serial>"` string (e.g. `DP-3:4`);
  `name`/`index` are mutable, `named` distinguishes them
  (`src/workspace/workspace.h:56-65`).
- `focused` is **per-workspace remembered focus**; `active` is seat-global
  activation. In the live payload exactly one window had `active:true` while
  five windows on five workspaces each had `focused:true` — the two-flag split
  is real and observable.
- Stacked columns exist in the live session (workspace `DP-3:4`: two kitty
  windows at `x=10`, `y=10`/`y=1086`, `h=1064` each, gap 12; single-window
  columns report `h=2140` = 2160 logical − 2×10 edge padding — consistent with
  **logical pixel** reporting on a scaled output).
- `pid` is `-1` for XWayland (steam), real otherwise; `x` can be negative for
  columns scrolled off-viewport (slot positions, not screen positions).

### Window entry fields (`IpcCommands::windows`, `ipc_commands.cpp:341-384`)

`id`, `workspace` (id), `scratchpad`, `active`, `app_id`, `title`, `xdg_tag`,
`content_type`, `floating`, `focused` (workspace-local), `urgent`, `xwayland`,
`pid`, and geometry: `x`/`y` = the **layout target slot** (`layout().targetBox()`
for tiled windows — the post-animation destination, computed even for hidden
workspaces; float position otherwise), `w`/`h` = `committedContentBox()` — the
size "the client has pixels for", widened toward a configured-but-committed
resize (`src/view/view.h:95-98`). So one payload mixes **target position with
committed size** — important for completion detection (below).

## Layout model relevant to Longcat

`src/layout/layout.h` + `src/layout/scrolling.cpp`:

- `Column` = `views[]` + proportional `heightWeights[]` + optional edge-gap
  weights + `widthFrac` + `savedWidthFrac` (`layout.h:123-130`). Columns are
  identified **positionally** (index) inside the layout — but see identity
  reconstruction below.
- `setWidthFraction(col, f)` clamps `[0.1, 1.0]`, clears `savedWidthFrac`
  (`scrolling.cpp:859-867`). Fraction→pixels solves `w = p·(V+g) − g` so N
  full-fraction columns tile exactly (`layout.h:290-293`).
- `setHeightFraction(view, f)` (`scrolling.cpp:895-922`): in a multi-row
  column, target clamps `[0.1, 0.95]` and sets this row's weight to
  `target·others/(1−target)` — **other rows keep their weights**, i.e. their
  relative proportions are preserved. Solo row: remainder flows to the
  column's edge gaps (fraction `1.0` reclaims it). Weight floor
  `kMinHeightWeight = 0.05` (`scrolling.cpp:19`).
- Client min/max size hints clamp every assigned row/column size
  (`LayoutConstraints`, `layout.h:90-113`, applied at `scrolling.cpp:790-792`)
  — fixed-size windows are handled natively, unlike zk's client-side
  `minimum_height` patching.
- Native baseline restore exists for width only: `savedWidthFrac` +
  `window-toggle-maximize` (`scrolling.cpp:844-857`); column moves preserve
  member order, width, full-width restore value, and row proportions
  (`actions.md` "Columns as a unit").
- **Named scrolling columns** are a config-owned grouping with stable names:
  window rules `default_scrolling_column` / `default_scrolling_column_order`
  (`docs/user/window-rules.md:98-99, 255-274`; `View::namedScrollingColumnName`,
  `Workspace::applyNamedScrollingColumnRule` in `src/workspace/workspace.h:75-130`).
- Pointer resizes go through `ResizeGrab` sessions that mutate the same
  weight/fraction state (`scrolling.cpp:997+`); `window-modify-height-up/down`
  IPC actions drive the same path.

## Longcat concerns, one by one

| Concern | Umbriel answer | Assessment |
|---|---|---|
| Grouping/tree topology | Columns of stacked windows in scrolling layout; `window-consume/-or-expel-*` stack/split; whole-column moves preserve structure | Native columns ≈ niri columns; grouping is real, not just visual |
| Stable identities | Windows: stable ext-foreign string ids; workspaces: stable `<output>:<serial>`; columns: positional index only | Better than niri for windows/workspaces; column identity must be derived — group snapshot entries by (workspace id, target `x`); members of a column share `x` and column width. Live data confirms |
| Focus changes incl. no focus | `windows` family fires on focus; seat focus = the single `active:true` entry; no-focus (layer surface, empty) = no active entry; per-workspace `focused` memory is separate | Clean; no `WindowFocusTimestampChanged`-style event or re-query needed — the snapshot *is* the state |
| Window open/close/move | All fire `windows` (and `workspaces` for occupancy); layout slot `x/y` updates on arrange | Covered by one subscription |
| Layout/geometry events | Full-snapshot `windows` events on any geometry/focus/title/workspace change, coalesced per loop iteration, byte-deduped | Equivalent info to niri's `WindowsChanged` but with no diffing burden |
| Logical vs physical px | Everything IPC-visible is wlroots layout (logical) coordinates; docs state logical px for corner radius, struts, reveal padding; live h=2140 on a 2160-logical panel matches | Logical throughout; one wording wrinkle: layout.md says floats' extent actions resize "physical width" — likely loose prose, verify on a scaled output (uncertainty below) |
| Proportional/fixed resize | Row heights are weights (proportional); column width a fraction; fixed sizes honored via client size-hint clamping; no set-fixed-px IPC for tiled windows | Longcat's pixel-exact `SetFixed` becomes fraction set/restore — same outcome modulo rounding; min-height policy becomes native hint clamping |
| Baseline restore vs automatic | No native height baseline; weights *are* the state. Native precedent: `savedWidthFrac` for width | Controller keeps its own per-window original fraction; restore = one fraction set (math below) |
| Command completion | `msg` `ok` = action parsed + executed, layout state mutated synchronously; arrange flushed pre-frame; snapshot arrives on the idle callback; `w/h` converge when the client commits | Reply ≠ geometry; watch the next snapshots until the active window's committed `h` reaches target (epsilon). Event-driven, no polling |
| Reconnect/loss/startup | Socket unlinked at shutdown; EOF on compositor exit; reconnect + resubscribe yields fresh initial snapshots per family; no replay needed by design (snapshots); a fresh subscriber always gets its initial line even when the broadcast was deduped (`ipc.cpp:83-85`) | Simple and robust; no niri-style registry bookkeeping |
| Manual resize attribution | None. Pointer grabs and `msg` actions mutate identical state; no attribution/request-tag fields | Same heuristic as niri (observed-vs-baseline + debounce), but cheaper: full snapshots carry all sibling geometry in one line |
| Config/WM-owned policies | `gap`, `extent_presets`, `default_extent_fraction`, `center_underfull_strip`, `center_focused`, `reveal/snap_padding`, cursor-follows-focus, window rules, live TOML reload | Longcat proportions remain zk-side policy, as with niri |
| Native equivalents | `savedWidthFrac`/maximize restore; `window-cycle-*-extent` presets; named scrolling columns; per-workspace focus memory; scratchpad roaming | Several Longcat-adjacent behaviors already exist as compositor features |

### The expansion/restore math (why this maps so well)

On focus gain of window W in column C, zk-niri sends one fixed-height action
per member (`longcat.rs:471-499`). On Umbriel, `setHeightFraction` preserves
other rows' weights, so expansion is **one action**:

```
msg window-set-secondary-extent:<active_proportion>   # W is focused already
```

Because siblings' weights are untouched, restoring W's own remembered
original fraction later restores every sibling exactly (the restored weight
`f_a/(1−f_a)·others` recomputes to the original `w_a` since `others` is
unchanged). No per-window loop, no proportion bookkeeping in the common case.

**The one real constraint: extent actions act only on the focused window**
(`Workspace::setFocusedHeight` uses `m_focusedView`,
`src/workspace/workspace.cpp:1805-1819`; no `[<window-id>]` argument form
exists for extent actions in `keybind_parse.cpp` — only `window-close` and
`window-toggle-floating` take an optional id). Restoring a column *after focus
left it* therefore requires a focus round-trip:
`window-focus:<expanded-id>` → `window-set-secondary-extent:<orig>` →
`window-focus-last`. Two costs:

1. Same-workspace restore: brief focus churn (border flicker; cursor warp if
   `input.cursor.follows_focus` is on — plain `window-focus` respects that
   setting, `window-focus-warp` always warps).
2. Cross-workspace restore: `FocusManager::focusView` **activates the target
   window's workspace** when it is not active (`src/server/focus.cpp:80-94`)
   — an eager restore of a column on another workspace would visibly switch
   workspaces. Policy mitigation: restore eagerly only on same-workspace
   focus exit; defer cross-workspace restores until that workspace is
   re-entered (or re-baseline instead of restoring).

The cheap structural fix is upstream: allow `window-set/modify-primary/secondary-extent:[<window-id>]`
following the existing `OptionalWindowId` precedent. `SCOPE.md` explicitly
keeps "IPC actions … for state it already tracks" in scope; this is a
parameter widening, not new machinery. That is the one upstream ask this
research recommends, and it is not needed for a first working port (the
round-trip is viable; zk's Longcat currently restores exactly on column exit,
mostly a same-workspace event anyway).

## Anchor comparison against the niri implementation

- `src/niri/registry.rs` (zk): subscribes to niri's event stream but only
  consumes focus timestamps; synthesizes Blur by remembering the last id;
  re-queries `Request::Windows` per focus event (`get_window_info`,
  registry.rs:193-220); ignores lifecycle/layout events. Parent findings:
  "registry ignores lifecycle/layout events, uses focus timestamps/query-heavy
  reads".
  → On Umbriel this whole layer collapses: one `subscribe windows,workspaces`
  connection yields complete state including geometry; `active`/`focused`
  flags replace timestamp+query inference; "no focus" is the absence of an
  active entry rather than a `WindowFocusChanged{id:None}` special case.
- `src/commands/longcat.rs` (zk): keys columns by `(workspace_id: u64,
  column_idx: usize)` from `pos_in_scrolling_layout` — the parent finding
  "restore keyed by mutable column index" is a real hazard on niri.
  → On Umbriel, key columns by `(workspace id string, member window ids)` or
  slot `x`; both are stable across reordering and scroll. The DashMap /
  RwLock / socket-pool concurrency (`longcat.rs:24-30`) stays unnecessary —
  one subscriber task plus one short-lived request socket per action mirrors
  the existing design and is sufficient.
- Manual-resize detection (`check_manual_resize_and_update_proportions`,
  longcat.rs:132-222, baseline-compare + debounce): ports as-is conceptually,
  with better inputs (one snapshot line carries all siblings) and a native
  guarantee that client min-size clamping never lets a manual shrink break the
  layout.
- kitty tools (`src/kitty/`, kitty-rc) and export code paths are WM-agnostic
  and unaffected by the niri→Umbriel swap.

## Rust integration: justified, but narrow

There is no protocol library to bind to — the wire format is ~300 lines of
hand-rolled JSON over a Unix socket, documented in `docs/user/ipc.md` and
implemented twice in-tree (server `src/server/ipc*.cpp`, client
`src/cli/ipc_client.cpp`). A generated/schema'd client is overkill; a thin
typed client is not. Proposed boundary, starting **as a zk-internal module**
`src/umbriel/` mirroring `src/niri/` (extract to a crate only when a second
consumer appears):

1. **Transport/discovery**: resolve `UMBRIEL_SOCKET` →
   `$XDG_RUNTIME_DIR/umbriel-$WAYLAND_DISPLAY.sock`; one-shot
   request/reply with timeout; auto-reconnect for the subscriber.
2. **Typed model**: serde structs for the `windows` entry (string id!),
   `workspaces` entry, `submap`; reply enum (`ok`/`err`) distinguishing the
   two line shapes (`{"ok"|"err"}` replies vs `{"event":...}` events — a
   subscriber connection can also carry one-shot replies, so the reader must
   discriminate by first key).
3. **Subscriber semantics baked in**: full-snapshot replacement (never diff),
   initial-state lines in fixed family order, dedup already done server-side.
4. **Action builders** for the Longcat subset (`window-set-secondary-extent`,
   `window-modify-secondary-extent`, `window-focus`, `window-focus-last`,
   `window-focus-warp`) with fraction/delta range validation
   (`0.1..=1.0` / `-0.9..=0.9`) and no further vocabulary — keep the crate
   honest about what zk uses.
5. **Derived helpers**: column reconstruction (group by workspace + target
   `x`, order by `y`), active-window resolution, geometry-convergence check
   (target vs committed `h` within epsilon).
6. Deliberately excluded: full action catalog, codegen, version negotiation
   (none exists upstream — pin to the observed 0.1.x behavior and cope
   additively).

Longcat-Umbriel then becomes a sibling of `src/commands/longcat.rs` sharing
config, with `active_proportion` → `window-set-secondary-extent` and the
restore policy from above.

## tmux bridging (extra, not prerequisite)

Distinct from compositor support: when the focused window is a terminal
hosting tmux, inner-pane focus is invisible to any compositor.
`tmux-interface-control` (revamped fork, README at
`/home/rektide/src/tmux-interface-control`) already provides the pieces a
bridge would need: `endpoint::Tmux` + `session().connect()` persistent control
client with correlated replies, typed `query::Panes` (pane ids, titles,
`pane_pid`), command builders (`select-pane`, `resize-pane -Z`), and
`subscribe_format` for sampled pane titles. Correlation to Umbriel windows is
feasible today via window `title` (live session shows kitty titles like
`"tmux-tracker - opencode"` — session name is already in the title) and/or
`pid` ancestry. Recommendation: keep this a separate, later effort; the
compositor-side Longcat port neither needs nor blocks it.

## Uncertainties

1. **Commit-vs-target lag**: `x/y` are post-animation targets while `w/h` are
   client-committed sizes; during animated resizes a snapshot can mix states.
   Completion logic must compare committed `h` against the *expected target in
   logical px*, tolerating intermediate frames. Needs live observation.
2. **"Physical width" wording for floats** in `docs/user/layout.md` vs logical
   everywhere else; Longcat only touches tiled geometry, but verify once on a
   scaled output if float support ever matters.
3. **Protocol churn**: 0.1.0, explicit instability warning, no version
   handshake. Field set observed live matches the checkout exactly (same
   tree), but a client must tolerate added fields and unknown event families
   gracefully.
4. **Subscribe+request mixing**: a subscribed connection answers further
   one-shot cmds inline (readable in `ipc.cpp:299-305` + `readRequest`
   continuing), interleaving reply and event lines; the CLI never does this.
   Safe to design around, but it is undocumented behavior — treat as
   unspecified until confirmed upstream.
5. **Umbriel-only columns**: snapshot column reconstruction relies on shared
   target `x` per column; a zero-width or transitional arrangement during
   animations could momentarily merge/split groups. Keying by member ids (not
   x) sidesteps most of it.

## Best next experiment

A read-mostly live probe script (`.test-agent/umbriel-probe/` when it
happens), no Rust yet, against the running session which already contains
stacked columns (e.g. `DP-3:4`):

1. `umbriel subscribe windows` in one pane; in another, focus a stacked window
   and send `msg window-set-secondary-extent:0.7` (this is the one mutating
   step; do it in a scratch column). Observe: snapshot count until the active
   window's `h` converges (reply→event latency), and that siblings keep
   relative proportions — validating the one-action expansion claim.
2. Pointer-resize a sibling mid-column; confirm snapshots arrive with no
   distinguishing field (attribution heuristic viability) and that client
   min-size clamping behaves.
3. Try the restore round-trip (`window-focus:<id>` → set extent →
   `window-focus-last`) within one workspace, then across workspaces to
   measure the workspace-activation disruption.
4. Only then: `src/umbriel/` module skeleton + Longcat-Umbriel port behind the
   existing config.

## Evidence index

- IPC server: `~/archive/noctalia-dev/umbriel/src/server/ipc.h` / `ipc.cpp`
  (subscribe `:340-403`, broadcast dedup `:418-452`, limits `:27-30`); event
  coalescing `src/server/server_events.cpp:1467-1501`.
- Queries/actions: `src/server/ipc_commands.cpp` (windows payload `:341-384`,
  workspaces `:386-413`, msg `:586-597`, command table `:616-629`); action
  grammar `src/config/keybind_parse.cpp:230-345`; dispatch
  `src/server/server_keybinds.cpp:104`.
- Layout: `src/layout/layout.h` (Column `:123-130`, constraints `:90-113`,
  presets `:147-171`); `src/layout/scrolling.cpp` (weights `:19,56-61`,
  width `:834-881`, height fraction `:883-922`, resize grab `:997+`).
- Workspace/focus: `src/workspace/workspace.h` (ids `:56-65`, named columns
  `:75-130`, markArrange `:132-140`); `src/workspace/workspace.cpp:1780-1860`;
  `src/server/focus.cpp:49-168` (workspace activation on focus `:80-94`).
- View: `src/view/view.h:60-199` (ext id, activated, committedContentBox,
  layoutTargetX/Y).
- Docs: `docs/user/ipc.md`, `actions.md`, `layout.md` (sizing, presets),
  `window-rules.md:98-99,255-274` (named scrolling columns), `SCOPE.md`.
- zk anchors: `src/commands/longcat.rs`, `src/niri/registry.rs`,
  `src/niri/types.rs`.
- tmux option: `/home/rektide/src/tmux-interface-control/README.md`.
