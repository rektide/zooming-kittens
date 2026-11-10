---
type: Report
title: Umbriel IPC landscape — handoff for Longcat reimplementations
description: Breadth survey of umbriel-side observation and control options, tooling, and preserved work, for alternate Longcat implementations.
resource: design/longcat/handoff0.glm53m.md
status: active
generated: { at: 2026-11-10, by: 'model:zai-coding-plan/glm-5.3#max' }
sources:
  - resource: opencode://ses_e3e46571dffet3gsSg7Uq4gW00
    title: Umbriel working-line recomposition, idtarget sizing qualification, and IPC landscape research
  - resource: file:///home/rektide/src/umbriel-working/.test-agent/longcat-cleanup/ipc-explainer0.glm53m.md
    title: Deep-dive on the layout-state/layout-allocate family — purpose, surface, semantics, port shape
  - resource: file:///home/rektide/src/umbriel-working/.test-agent/longcat-cleanup/longcat-needs0.glm53m.md
    title: Whether running longcat requires the bespoke IPC — verdict and zk code paths
  - resource: file:///home/rektide/src/umbriel-working/.test-agent/longcat-cleanup/research0.glm53m.md
    title: What longcat is, the control/deploy lines, and umbriel-rs
  - resource: file:///home/rektide/src/umbriel-working/.test-agent/longcat-cleanup/update-plan0.glm53m.md
    title: Deploy plan with operator decisions (2026-11-09)
---

# Umbriel IPC landscape — handoff for Longcat reimplementations

On 2026-11-09/10 the "port the bespoke layout-control family?" question resolved
into a different shape: upstream's stock IPC already carries most of what a
loose Longcat needs, and the one genuinely missing capability — sizing without
stealing focus — landed as a small pattern-consistent extension instead of the
bespoke family. This note maps the whole option/tool/system space so
reimplementations and alternate adapters can choose deliberately.

## Three umbriel surfaces for control

| Surface | Read | Write | Targeting | Exactness | Atomicity | Where |
| --- | --- | --- | --- | --- | --- | --- |
| Stock query/action IPC | rich (below) | focus-anchored deltas/fractions | focused only | pixel-level | none | upstream `main` |
| + `idtarget` sizing qualification | same | same, plus id-qualified | **by window id** | pixel-level | none | `idtarget-20261110` (upstream-based), `working-20261110` (integrated) |
| Bespoke layout-control family | full basis | whole-column weights | by identity | weight-exact | guarded, reject-before-mutate | `longcat-control-20261022`, `longcat-deploy-candidate-20261022` (not deployed, not in working) |

## What stock IPC reads

Commands: `windows`, `focused-window`, `workspaces`, `outputs`, `layers`,
`submap`, `effects`, `input-capture-permissions`. The `windows` payload is the
workhorse: per-window **geometry** (explicitly *layout targets*, reported ahead
of client redraw), **size updates that follow committed client geometry** —
i.e. the target-vs-committed distinction ships in the stock stream — plus
`column`/`row` placement (scrolling strip indices), tab-group membership
(`tabbed`, `tab_index`, `tab_hidden`), flags, workspace and output. `outputs`
reports workareas and logical geometry. Streams: `subscribe` with the
`windows`/`workspaces`/`layers`/`outputs` families — **full-snapshot payloads,
deduplicated, coalesced to at most one per 16 ms** (upstream #723's pacing), so
a burst always ends at the newest state. No framebuffer/pixel access is needed
anywhere in a control loop; committed sizes prove convergence. Wire is
newline-delimited JSON over `$XDG_RUNTIME_DIR/umbriel-<wayland-socket>.sock`;
trust model is socket authority, same as `msg` — no per-command gating.

What stock IPC **cannot** read: the allocation *basis* — weights (exact
doubles), constraints, spans. Geometry is the rendered consequence; two
different weight vectors can render identically.

## Writing without focus — the idtarget qualification (2026-11-10)

The sizing family (`window-modify-height-up/down`, `window-modify-width-left/right`,
`window-set/modify-{primary,secondary}-extent`) now accepts the established
`--window <window-id>` qualifier (the same doctrine as `window-move-by`):

- width forms resolve the id to its **containing column** and resize it;
- height forms act on the **named window**;
- bare forms are unchanged (focused), floating targets resize themselves;
- stale ids get the standard reply: `{"err":"unknown window: <id>"}`.

Preserved as `idtarget`/`idtarget-20261110` (standalone atop upstream `main`)
and integrated as `working`/`working-20261110` (build 187/187,
unit 153/153, targeted harness 6/6). Tests: `tests/unit/ipc_commands.cpp`,
`tests/unit/keybind_parse.cpp`, harness `layout/id_resize_actions.sh`. The
change follows upstream's own qualification doctrine verbatim, so it is a
natural upstream contribution candidate.

Do **not** approximate this with a focus dance: in a scrolling compositor,
focusing has viewport side effects (reveal-from-scratchpad, scroll-to-show,
cursor-warp variants) plus client-visible focus events. The controller would
move the user's view, not just their focus.

## If you need basis-truth — the preserved bespoke family

`layout-state`/`layout-allocate` uniquely provides: the complete allocation
basis with exact-doubles weights (bit-exact native restoration — including
sub-floor weights from tab dissolution — and rejection of same-pixel
different-weights edits); atomic whole-column weight assignment guarded by
compare-current-state deep equality (concurrent user edits reject instead of
clobbering; twelve distinct error replies; validation on a cloned layout via
the same `arrange()` math); and focus-free targeting. It lives only on
`longcat-control-20261022` (`5432ace1`, upstream-based) and
`longcat-deploy-candidate-20261022` (`96b118cf`, composed-based); its contract
doc is `docs/user/layout-control.md` inside those trees. Not deployed, not in
any working line. A future port is narrower than it was in October — upstream
absorbed the committed-size publication and the title bars exist on both sides
now — with one real decision: fitting the notify chain into the 16 ms
`EventPacer` instead of the candidate's direct broadcast.

## The loop an alternate implementation can close on stock

subscribe → read targets/placement → compute → id-qualified write → confirm
via committed sizes. Exact: geometry save/restore, convergence detection,
placement facts. Approximate: restore is to observed pixels (not underlying
weights), and a write sequence is unguarded (a user edit mid-sequence is
overridden, not rejected) — check-then-act windows are real but tolerable for
a loose toolkit.

## Adapter evidence stance

zk's `Evidence::Reported` requirement is a policy in zk's code, not an upstream
wall: the compositor *does* report placement (`column`/`row`, tab structure)
and geometry/committed sizes — only the basis is unreported. Treating
stock-reported placement as evidence is a coherent relaxation for loose
toolkits; receipts and convergence remain provable from the stream.

## Client tooling

- **umbriel-rs** (`~/src/umbriel-rs`): typed client with first-class
  subscriptions (`crates/umbriel-client/src/driver.rs`, `pub async fn
  subscribe`), the `tracking` crate (observation installation,
  Inventory-owned freshness), and the running `activity` service
  (`umbriel-activity.service`). Integration examples under
  `integration/kopacetic-active`.
- **zk's own adapter** (`desktop/umbriel/src/`): `source.rs` (`n`/`n_layout`
  constructors) feeding the reducer over `Windows`/`Workspaces` families — the
  dry-run inventory is already stream-fed.

## State map

Umbriel repo (colocated jj+git at `/mnt/bitblaster/archive/noctalia-dev/umbriel`;
workspaces `~/src/umbriel-working` integration, `~/src/umbriel-idtarget`
feature): `main` (upstream), `working`+`working-{20261009,20261109,20261110}`
(integration lineage), `power-memory`+stamps, `pwr-20261009`,
`movering`/`pad`/`vit`+stamps, `longcat-control-20261022`,
`longcat-deploy-candidate-20261022`, `idtarget`+`idtarget-20261110` — all
datestamps pushed to `fork` (github rektide/umbriel). Research notes:
`~/src/umbriel-working/.test-agent/longcat-cleanup/` (`research0`,
`longcat-needs0`, `ipc-explainer0`, `update-plan0`, plus the deploy-worktree
snapshot and integration record in `../reintegrate-20261109/`).

## Open threads

- `zk-longcat-return-native-deploy` remains open; the composed-deploy path it
  tracked is superseded by the working-line route (compfuzor pins
  `working-YYYYMMDD@rektide` directly).
- The evidence relaxation above is a zk-side decision no one has made yet.
- Verification gaps on idtarget: the full 463-check harness and ASAN were not
  run; unit (153) plus six targeted harness checks were.
- The idtarget change is a strong upstream PR candidate as-is.
