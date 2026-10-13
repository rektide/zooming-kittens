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
revisions:
  - at: 2026-10-13
    by: model:zai-coding-plan/glm-5.3#max
    note: revision 1 integrates parent-review corrections (see "Revision 1 — corrections"); original claims preserved in change pvtwpsyu
sources:
  - resource: opencode://ses_ecc154b5fffeUhXQiZBaZOjPB3
    title: Umbriel source inspection, live IPC ground-truth queries, Longcat feasibility mapping, and revision 1 corrections
  - resource: opencode://ses_ecc23c827ffemoNX74kyxXknpv
    title: Parent review corrections — restoration algebra preconditions, slot-x instability, expansion race, manual-detection redesign, library boundary
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
implements or modifies Umbriel. **Revision 1** corrects several first-wave
overclaims — restoration preconditions, slot-x stability, an expansion race,
the manual-detection approach, and the library boundary; see
[Revision 1 — corrections](#revision-1--corrections) before relying on the
expansion/restore and identity claims below.

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
  wakes nobody". Two caveats a controller must respect: each family is
  individually consistent but the families are **not an atomic joint
  snapshot** (a `windows` line and an adjacent `workspaces` line may bracket
  other mutations), and per-iteration coalescing means **intermediate states
  can be hidden** — a focus that moved twice within one event-loop iteration
  is observed only at its landing point.
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
| Stable identities | Windows: stable ext-foreign string ids; workspaces: stable `<output>:<serial>`; columns: positional index only | Better than niri for windows/workspaces. Column identity has **no durable handle**: slot `x` is scroll- and order-relative (see Revision 1), and member-id sets survive reorder but change on merge/split. Geometry (shared `x`) is a *heuristic for present membership*, never an identity |
| Focus changes incl. no focus | `windows` family fires on focus; seat focus = the single `active:true` entry; no-focus (layer surface, empty) = no active entry; per-workspace `focused` memory is separate | Clean; no `WindowFocusTimestampChanged`-style event or re-query needed — the snapshot *is* the state |
| Window open/close/move | All fire `windows` (and `workspaces` for occupancy); layout slot `x/y` updates on arrange | Covered by one subscription |
| Layout/geometry events | Full-snapshot `windows` events on any geometry/focus/title/workspace change, coalesced per loop iteration, byte-deduped | Equivalent info to niri's `WindowsChanged` but with no diffing burden |
| Logical vs physical px | Everything IPC-visible is wlroots layout (logical) coordinates; docs state logical px for corner radius, struts, reveal padding; live h=2140 on a 2160-logical panel matches | Logical throughout; one wording wrinkle: layout.md says floats' extent actions resize "physical width" — likely loose prose, verify on a scaled output (uncertainty below) |
| Proportional/fixed resize | Row heights are weights (proportional); column width a fraction; fixed sizes honored via client size-hint clamping; no set-fixed-px IPC for tiled windows | Longcat's pixel-exact `SetFixed` becomes fraction set/restore — but exact restoration is conditional (Revision 1): fractions/weights are not observable, so restores carry recovery error; min-height policy becomes native hint clamping |
| Baseline restore vs automatic | No native height baseline; weights *are* the state. Native precedent: `savedWidthFrac` for width | Controller keeps its own baseline; restore fidelity is bounded by what the IPC exposes (see expansion/restore preconditions) |
| Command completion | `msg` `ok` = action parsed + executed, layout state mutated synchronously; arrange flushed pre-frame; snapshot arrives on the idle callback; `w/h` converge when the client commits | Reply ≠ geometry; watch the next snapshots until the active window's committed `h` reaches target (epsilon). Event-driven, no polling |
| Reconnect/loss/startup | Socket unlinked at shutdown; EOF on compositor exit; reconnect + resubscribe yields fresh initial snapshots per family; no replay needed by design (snapshots); a fresh subscriber always gets its initial line even when the broadcast was deduped (`ipc.cpp:83-85`) | Families are individually snapshot-consistent but **not an atomic joint snapshot**, coalescing can hide intermediate focuses, and no missed interval is recoverable. Clients must treat a new stream as a new interval — no silent auto-reconnect |
| Manual resize attribution | None. Pointer grabs and `msg` actions mutate identical state; no attribution/request-tag fields | The observed-vs-baseline detector is the *existing zk bug* and must not be ported as-is. No attribution is available at any layer; a redesign distinguishing baseline / requested / observed, with conditional restore or suspension on conflicting change, is required (Revision 1) |
| Config/WM-owned policies | `gap`, `extent_presets`, `default_extent_fraction`, `center_underfull_strip`, `center_focused`, `reveal/snap_padding`, cursor-follows-focus, window rules, live TOML reload | Longcat proportions remain zk-side policy, as with niri |
| Native equivalents | `savedWidthFrac`/maximize restore; `window-cycle-*-extent` presets; named scrolling columns; per-workspace focus memory; scratchpad roaming | Several Longcat-adjacent behaviors already exist as compositor features |

### The expansion/restore math — and its preconditions

On focus gain of window W in column C, zk-niri sends one fixed-height action
per member (`longcat.rs:471-499`). On Umbriel, `setHeightFraction` preserves
other rows' weights, so expansion is **one action**:

```
msg window-set-secondary-extent:<active_proportion>   # W is focused already
```

**Restoration is the hard part, not the easy part.** The algebra "set W back
to its remembered original fraction and every sibling is restored exactly"
(the restored weight `f_a/(1−f_a)·others` recomputes the original `w_a`
because `others` is unchanged) holds only under three preconditions
(Revision 1):

1. the controller knows W's **original internal `heightFraction`** — not an
   approximation of it;
2. sibling weights *and edge-gap weights* are unchanged since the baseline
   (a pointer drag on any sibling, or a solo-row episode that moved remainder
   into the column's edge gaps, silently invalidates it);
3. no clamp or floor intervened — the `0.05` weight floor, the `0.95`
   multi-row target clamp, and client min/max size-hint clamping
   (`LayoutConstraints` applied per row, `scrolling.cpp:790-792`) can all make
   observed geometry diverge from the requested fraction.

Condition 1 is not satisfiable from the IPC today, and that is an
**additional protocol gap** distinct from command id-targeting: the `windows`
payload exposes target `x/y` and *committed* content `w/h` only — no weights,
no fractions, no target `w/h` (`IpcCommands::windows`,
`ipc_commands.cpp:341-384`). A client cannot universally recover the exact
original fraction from observed heights: size-hint clamps, pixel rounding,
edge-gap absorption, and configure lag all decorrelate committed pixels from
the internal fraction. Fraction-derived baselines are approximations, and
restoring from an approximation drifts.

**A second gap: focused-window-only actions are a race, not an inconvenience.**
Extent actions act on the focused window (`Workspace::setFocusedHeight` uses
`m_focusedView`, `src/workspace/workspace.cpp:1805-1819`). Between the
snapshot that told the controller "W is active" and the `msg` that executes,
focus may have moved — the action then resizes the wrong window. Safe
automation needs explicit window-id targets for extent commands; id-targeting
is therefore *necessary for expansion itself*, not merely a convenience for
restoration. Note the grammar constraint: an action takes **at most one
argument** (`actions.md` "Argument forms"; `ActionArgKind`,
`keybind_parse.h:259-280`), so id + fraction is a designed command/API
extension — a second argument, a distinct action, or a structured `msg`
payload — not simply swapping in the existing `OptionalWindowId` precedent.

The focus round-trip workaround (`window-focus:<id>` → set extent →
`window-focus-last`) is **a scratch-only experiment for characterizing
behavior, not a production recommendation** (Revision 1). Its costs are real:
same-workspace churn (borders; cursor warp under `input.cursor.follows_focus`,
which plain `window-focus` respects) and, worse,
`FocusManager::focusView` **activates the target window's workspace** when it
is not active (`src/server/focus.cpp:80-94`) — a cross-workspace restore
visibly switches workspaces, and no "focus without activation" action exists
to work around it.

**Upstream asks this research recommends (two, deliberately separate):**

1. *Layout-state query*: expose actual per-window `heightFraction` and
   per-column `widthFraction`, target slots, column/group membership, and the
   workspace's strip axis — state the layout already tracks, which `SCOPE.md`
   keeps in scope ("IPC actions and inspection output for state it already
   tracks"). This addresses baseline fidelity and identity, and is
   independent of command acknowledgement.
2. *Id-targeted extent commands*: a grammar extension so automation never
   depends on focus racing. `window-close`/`window-toggle-floating`'s
   `OptionalWindowId` is the precedent for id-targeting, but the
   two-argument need makes this a designed extension, not a parameter
   widening.

Neither is needed to *observe* (the experiment below), and a throwaway
prototype can tolerate approximation — but production Longcat semantics
should not be built on the focus round-trip or on fraction guessing.

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
  → Umbriel is better but not solved: the member-window-id *set* is the most
  durable column handle available (survives reorder and scroll) yet still
  changes on merge/split (`window-consume`/`-expel`), and slot `x` is
  **scroll-relative** — `arrange` computes each slot as
  `runningColumnX − m_scroll` (`scrolling.cpp:752-765`) — so shared-`x`
  grouping is only a heuristic for *present* membership, never an identity
  (Revision 1). The DashMap / RwLock / socket-pool concurrency
  (`longcat.rs:24-30`) stays unnecessary — one subscriber task plus one
  short-lived request socket per action mirrors the existing design and is
  sufficient.
- Manual-resize detection (`check_manual_resize_and_update_proportions`,
  `longcat.rs:132-222`, baseline-compare + debounce): this detector **is the
  existing bug and must not be ported as-is** (Revision 1). It conflates
  baseline, requested, and observed geometry; on Umbriel no attribution
  exists at any layer, so a replacement must track all three explicitly —
  the controller's own last request, the baseline it intends to restore, and
  the committed snapshot — and treat a change it did not request as a
  *conflict* that suspends or conditionally re-baselines expansion, rather
  than silently absorbing it into new proportions. Native client min-size
  clamping keeps the layout *valid* under a manual shrink; it does not tell
  the controller who owns the change or make the shrink recoverable.
- Export and kitty are **not** uniformly WM-agnostic (Revision 1): the export
  pipeline is niri-specific today (`src/export/snapshot.rs`, `record.rs`,
  `stream.rs`, `facet_json.rs` consume `niri_ipc` types), though its
  serialization/filtering patterns are reusable for an Umbriel feed; the
  kitty focus tracker (`src/kitty/resizer.rs`) also depends on niri events,
  while the standalone font/systemd operations (`src/commands/fonts.rs`,
  `systemd.rs`) do not.

## Rust integration: a neutral thin client library is reasonable

There is no protocol library to bind to — the wire format is ~300 lines of
hand-rolled JSON over a Unix socket, documented in `docs/user/ipc.md` and
implemented twice in-tree (server `src/server/ipc*.cpp`, client
`src/cli/ipc_client.cpp`). A generated/schema'd client is overkill; a thin
typed client is not. The user explicitly welcomes a Rust library, so a small
**reusable, WM-neutral client crate is a reasonable artifact in its own
right** — not contingent on staying internal or on a second consumer
appearing (Revision 1; zk can be its first consumer via a path dependency,
mirroring how `tmux-interface-control` is consumed). Proposed core:

1. **Socket discovery**: `UMBRIEL_SOCKET` →
   `$XDG_RUNTIME_DIR/umbriel-$WAYLAND_DISPLAY.sock`.
2. **Typed IDs and queries**: serde types for the `windows` entry (string
   id), `workspaces` entry, `submap`; reply enum (`ok`/`err`) with errors and
   timeouts surfaced, not swallowed.
3. **Event envelopes**: typed full-snapshot event lines per family, with
   family-order initial state and replacement (never diffing) semantics
   encoded in the API shape.
4. **Raw `msg` escape hatch**: typed action builders for a *finite* set
   (fraction/delta args validated to `0.1..=1.0` / `-0.9..=0.9`), plus the
   raw string form for anything else — the crate does not pretend to own the
   action vocabulary.
5. **Separate request and subscribe sockets by default**: reply lines
   (`{"ok"|"err"}`) and event lines (`{"event":...}`) can interleave on one
   subscribed connection (see uncertainty 4), so the safe default is one
   connection per role.

Deliberately **outside** the protocol library (Revision 1): Longcat policy
(proportions, expansion/restore/suspend rules), projection/topology inference
(column reconstruction from geometry), geometry-convergence checking, and
**automatic reconnection** — EOF or stream loss is surfaced to the caller,
who explicitly establishes the new stream interval, because families are not
an atomic joint snapshot and missed intermediate states are unrecoverable.

Longcat-Umbriel then lives in zk as a sibling of `src/commands/longcat.rs`,
sharing config, consuming the client crate, and owning the policy layer.

## tmux bridging (extra, not prerequisite)

Distinct from compositor support: when the focused window is a terminal
hosting tmux, inner-pane focus is invisible to any compositor.
`tmux-interface-control` (revamped fork, README at
`/home/rektide/src/tmux-interface-control`) already provides the pieces a
bridge would need: `endpoint::Tmux` + `session().connect()` persistent control
client with correlated replies, typed `query::Panes` (pane ids, titles,
`pane_pid`), command builders (`select-pane`, `resize-pane -Z`), and
`subscribe_format` for sampled pane titles. **Title correlation is only a
heuristic** (Revision 1): kitty titles like `"tmux-tracker - opencode"` embed
session names today, but titles are client-controlled and change freely.
Trustworthy resolution should use the typed `Clients` query — client PID and
TTY — joined against the terminal window's `pid` and its child processes'
TTYs, with the title match as a fallback or confirmation, not as the primary
key. Recommendation: keep this a separate, later effort; the compositor-side
Longcat port neither needs nor blocks it.

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
5. **Column reconstruction is heuristic**: shared target `x` groups only
   present membership; `x` is scroll-relative (`scrolling.cpp:752-765`) and
   transitional arrangements during animations can momentarily merge/split
   groups; member-id sets change on merge/split. Durable column identity
   needs the upstream layout-state query.
6. **Strip axis is not exposed**: on an output with horizontal workspaces the
   strip runs vertically and primary/secondary axes swap meaning
   (`docs/user/layout.md` "Horizontal and vertical scrolling"); the
   `workspaces` payload carries `layout` but not the axis. A prototype should
   be limited to a confirmed horizontal scrolling strip, or the axis should
   be added to the layout-state query (Revision 1).

## Best next experiment

A read-mostly live probe script (`.test-agent/umbriel-probe/` when it
happens), no Rust yet, against the running session which already contains
stacked columns (e.g. `DP-3:4`). Scope it to a **confirmed horizontal
scrolling strip** (uncertainty 6); this research performed only read-only
queries — the mutating steps below are for the experiment session, in a
scratch column:

1. `umbriel subscribe windows` in one pane; in another, focus a stacked window
   and send `msg window-set-secondary-extent:0.7` (the one mutating step; use
   a scratch column). Observe: snapshot count until the active window's `h`
   converges (reply→event latency), and whether siblings keep relative
   proportions — validating the one-action expansion claim. Then restore by
   fraction and measure the residual error against the original committed
   heights: this quantifies the fraction-recovery gap (Revision 1 correction
   1) on real clients with real size hints.
2. Pointer-resize a sibling mid-column; confirm snapshots arrive with no
   distinguishing field (no attribution anywhere) and that the requested/
   observed split is actually detectable from the stream — grounding the
   baseline/requested/observed redesign rather than the old detector.
3. Scratch-only characterization of the focus round-trip
   (`window-focus:<id>` → set extent → `window-focus-last`) within one
   workspace, then across workspaces, to measure the workspace-activation
   disruption — evidence for the id-targeted-command upstream ask, not a
   production technique.
4. Only then: thin Rust client crate (protocol core only, per the boundary
   above) + Longcat-Umbriel policy module behind the existing config.

## Revision 1 — corrections

Parent review (`ses_ecc23c827ffemoNX74kyxXknpv`) identified eight errors or
overclaims in the first wave (preserved in change `pvtwpsyu`); this revision
integrates them:

1. **Restoration algebra**: "restore W's original fraction and siblings
   restore exactly" holds only with a known original internal
   `heightFraction`, unchanged sibling *and edge-gap* weights, and no
   clamp/floor intervention. The IPC never exposes weights, fractions, or
   target `w/h`, and committed pixels cannot universally recover the fraction
   (hints, rounding, edge gaps, configure lag). This is a *protocol gap* on
   top of id-targeting; the recommended upstream layout-state query covers
   it and is separate from command acknowledgement.
2. **Slot `x` is not identity**: `arrange` computes slots as
   `runningColumnX − m_scroll` (`scrolling.cpp:752-765`), so `x` shifts with
   scroll and reorder. Geometry now marks present-membership inference only;
   member-id sets are the best available handle but change on merge/split.
   Vertical strips rotate axes and the axis is not exposed — prototype scope
   now says "confirmed horizontal scrolling" or seek layout metadata.
3. **Manual detection**: the baseline-vs-observed detector is zk's existing
   *bug*, not a portable asset. The revision reframes to
   baseline/requested/observed with conflict-triggered suspension, and drops
   the implication that native min-size clamps solve ownership.
4. **Focus-only actions are a race** (snapshot→`msg` TOCTOU), making
   id-targeted extent commands necessary for safe automation, not a restore
   convenience. The focus round-trip is downgraded to a scratch-only
   characterization. The single-argument action grammar makes id+fraction a
   designed command/API extension, not an `OptionalWindowId` swap.
5. **Export is niri-specific** in current zk (`src/export/{snapshot,record,
   stream,facet_json}.rs`), with reusable serialization/filtering patterns;
   the kitty focus tracker depends on niri; standalone font ops do not.
6. **Non-atomic snapshots**: families are individually consistent but not a
   joint snapshot; coalescing can hide intermediate focuses; reconnect
   cannot recover missed intervals. The library boundary now forbids
   automatic reconnection and surfaces EOF/loss for explicit interval
   re-establishment.
7. **Library posture**: the user welcomes a Rust library; the revision
   presents a neutral thin reusable client as reasonable on its own, keeps
   Longcat policy / topology inference / convergence / auto-reconnect
   outside it, and defaults to separate request and subscribe sockets.
8. **tmux correlation**: title matching is heuristic only; trustworthy
   resolution needs typed `Clients` (PID/TTY) joined to the terminal window.

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
  arrange slot/scroll math `:736-802`, width `:834-881`,
  height fraction `:883-922`, resize grab `:997+`).
- Action grammar: `docs/user/actions.md` "Argument forms" (at most one
  argument); `src/config/keybind_parse.h:259-296` (`ActionArgKind`,
  `parseAction`).
- Workspace/focus: `src/workspace/workspace.h` (ids `:56-65`, named columns
  `:75-130`, markArrange `:132-140`); `src/workspace/workspace.cpp:1780-1860`;
  `src/server/focus.cpp:49-168` (workspace activation on focus `:80-94`).
- View: `src/view/view.h:60-199` (ext id, activated, committedContentBox,
  layoutTargetX/Y).
- Docs: `docs/user/ipc.md`, `actions.md`, `layout.md` (sizing, presets),
  `window-rules.md:98-99,255-274` (named scrolling columns), `SCOPE.md`.
- zk anchors: `src/commands/longcat.rs`, `src/niri/registry.rs`,
  `src/niri/types.rs`; niri-coupled export pipeline
  `src/export/{snapshot,record,stream,facet_json}.rs`; niri-coupled kitty
  focus tracker `src/kitty/resizer.rs`.
- tmux option: `/home/rektide/src/tmux-interface-control/README.md`
  (typed `Clients`/`Panes`, `session()`, `subscribe_format`).
