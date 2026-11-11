---
type: ImplementationReport
title: Stock Longcat boundary — slice A receipt and continuation
description: Implemented native stock models and tracking, semantic projection, qualified internal-edge admission, verification and the B–D handoff.
resource: design/longcat/implementation-a0.gpt61s.md
status: checkpointed
generated: { at: 2026-11-11, by: 'model:openai/gpt-6.1-sol#high' }
sources:
  - resource: opencode://ses_e331fea99ffeEcIAeJ9F8pM2aK
    title: Implement and verify native stock layout/output tracking and the ZK qualified relative-edge boundary without changing application policy or the live desktop
  - resource: feedback0.gpt6a.md
    title: Selected implementation contract and A–D assignments
  - resource: ../../desktop/README.md
    title: Maintained observation and native command boundary
  - resource: ../../desktop/umbriel/src/edge.rs
    title: Epoch-bound server grammar verification, pure preparation and one owned transmission
  - resource: ../../desktop/umbriel/src/stock.rs
    title: Reported native membership and separately inferred physical axis
  - resource: file:///home/rektide/src/umbriel-rs/crates/umbriel-state/src/inventory.rs
    title: Sole native row owner, independent outputs freshness and layout receipt filtering
  - resource: file:///home/rektide/src/umbriel-rs/crates/umbriel-client/src/requests.rs
    title: Reusable local precondition check before connect and immediately before transmission
  - resource: file:///home/rektide/src/umbriel-rs/crates/umbriel-protocol/tests/native_qualified.rs
    title: Actual emitted Rust strings tested against the native parser
  - resource: file:///home/rektide/src/umbriel-working/tests/harness/checks/layout/id_resize_actions.sh
    title: Existing isolated native by-ID sizing and focus-isolation acceptance check
  - resource: file:///home/rektide/src/umbriel-working/src/server/action_request.cpp
    title: Actual structured action fields and native CLI-to-wire encoder, checked in the source-backed oracle
  - resource: file:///home/rektide/src/umbriel-working/src/server/ipc.cpp
    title: Working-line cmd action dispatch, distinct from historical text msg IPC
  - resource: ../../desktop/umbriel/tests/native_stock.rs
    title: Real stock subscription, impossible-ID verification and qualified edge response on an isolated compositor
verification:
  method: Scoped Nextest, doctests, warning-denying all-target Clippy, source-parser conformance and an isolated native headless by-ID check.
  result: Native protocol/state/session/client/tracking scope passes 163 tests plus five doctests; ZK desktop/layout/application scope passes 130 tests. Actual strings and complete requests pass native CLI/server oracles. Both existing by-ID and new Rust stock-boundary headless checks pass.
  limits: No integrated stock feedback controller, running desktop mutation, service change, installation or push. Package-wide inherited rustfmt still reports pre-existing formatting drift; explicit owned-file formatting is checked using the original default source style.
extensions:
  ticket: zk-longcat-return
  related_work: [zk-longcat-user-layout-changes]
---

# Slice A delivered

This implements [assignment A](feedback0.gpt6a.md#a--stock-observation-and-qualified-command-boundary),
not the complete feedback loop. B–D can now use real typed stock facts and the
qualified boundary. The old application route/policy has not been selected as
the new destination merely because its code still compiles.

## Commits and ownership

| Repository | Commit | Coherent stage |
|---|---|---|
| umbriel-rs | `0434dea0` | Adopt current Kubeless 0.23.0 receiving pins/lock; the sibling checkout no longer supplies pinned 0.6.0. |
| umbriel-rs | `8711e755` | Typed stock layout additions, outputs query/event/state and independently fresh tracking, semantic receipt filter, current qualified sizing and native parser conformance. |
| umbriel-rs | `d8eb0b9c` | Reusable one-shot local admission recheck after connect, with a no-byte rejection test. |
| umbriel-rs | `2f089d34` | Preserve older read-only desktop dumps with explicit unavailable stock-output context, without inventing geometry or swallowing other errors. |
| umbriel-rs | `f62b2001` | Correct qualified commands to structured working-line action IPC; compare full Rust requests with native CLI encoding and server decoding. |
| zooming-kittens | `4bc87d18` | Receiving Kubeless pins/lock, including the already-unversioned scheduler's current transitive leaves. |
| zooming-kittens | `d52d0113` | Semantic stock projection, qualified internal-edge boundary, scoped tests and mechanical niri/bespoke/application DTO adaptations. |
| zooming-kittens | `127036ef` | Structured qualified requests at the adapter and a real isolated Rust stock observation/probe/edge acceptance test. |

Both repositories use their existing sibling paths. No Kubeless source changed.
The starting uncommitted `umbriel-rs/.beads/issues.jsonl` history closure belongs
to earlier work and remains uncommitted/unmodified by this implementation.
No commits were squashed or pushed. Native compositor sources were not edited.

## Shared interfaces for B–D

- **Native owner:** `Window` keeps optional current layout/presentation fields;
  `Workspace` keeps additional count/focus/layout metadata; `Output` types the
  logical context and workarea. `Family::Outputs`, `Query::Outputs`,
  `Event::Outputs` and `Snapshot.outputs` are independently installed/freshened.
  Missing older fields are not zero/false evidence. `WindowChanges` still carries
  the actual Inventory replacement receipt; `retain_layout_changes()` filters
  title-only touches while preserving both sides of topology/context edits.
- **Semantic facts:** `WindowView.output` is source/epoch-qualified and
  `presentation` retains reported flags. `DesktopCapture.outputs_status` is
  `Some` for stock and required by `is_ready()` alongside windows/workspaces in
  the same epoch. Niri and bespoke keep it `None`. `outputs` carries scoped
  `OutputView`s. `Impact.outputs` affects only groups associated with changed
  output context, not every group on any outputs publication.
- **Groups:** `GroupView.members` is ordered from reported rows in a single
  windows census, under workspace-qualified column keys. Identity is still
  `WindowKey`, not row/column position. `secondary_axis: Option<Axis>` and
  `geometry_evidence` keep inferred/unknown geometry separate from reported
  membership. `edge_eligibility` explains local admission. Known `(-1,-1)`
  nonmembers do not veto surviving groups; unknown/half-located siblings do.
- **Command DTO:** `RelativeEdgeRequest { group, target, following, basis, axis,
  fraction }` lives in `zk-desktop`, without Umbriel protocol types. `following`
  must be the actual next member; a last/outer edge never compiles. Fraction is
  a signed native tiled-area delta, not a pixel count, output fraction guarantee,
  neutral weight or desired solver share.
- **Preparation:** `zk_umbriel::edge::prepare(&capture, &request)` returns a typed
  native action after local basis/scope/eligibility/adjacency checks. Vertical
  uses height-down and horizontal width-right, including for negative travel.
  The returned native `QualifiedSizing` separates its diagnostic CLI/keybind
  spelling from structured `cmd: action` IPC (`action`, numeric `delta`, and
  nonempty `window` fields). Native CLI `--window` spelling is not the wire
  grammar. This was caught during final source review and corrected separately;
  old `Action`/`Request::Msg` APIs remain historical, not a live fallback. All
  encoding stays with umbriel-protocol; no shell or focus dance is involved.
- **Runtime admission:** stock `Source::edge_actuator().await` verifies both
  qualified physical actions using a nonempty impossible ID containing NUL.
  Native identifiers are C strings; that full token cannot name a mapped window.
  Only the exact `unknown window: <probe-id>` reply admits the handle. Unknown,
  generic rejection and even successful handling deny admission. The real native
  parser test covers preservation of this token. Bespoke sources return `NotStock`.
- **Transmission:** `EdgeActuator::execute(RelativeEdgeRequest)` shares one
  no-queue permit across handles/clones, rechecks current local facts after
  connection, and returns `Unsent`, `Rejected`, `Handled` or `PossiblySent`.
  Verification is bound to its source epoch; old handles cannot reconnect into
  new authority. Source shutdown does not cancel an already admitted one-shot
  future or erase its receipt. The application must own/drain that future.

Stock `height_by_id`, atomic assignment, native revision and exact restoration
remain false. `relative_edge_by_id` describes an eligible adapter operation, **not
server negotiation**; the verified actuator is the additional runtime gate.
Its local precondition cannot close a compositor-side race after transmission.

## Verification receipts

- `cargo nextest run --locked -p umbriel-protocol -p umbriel-state -p umbriel-session -p umbriel-tracking -p umbriel-client`:
  **163 passed**, native-parser opt-in test skipped in this ordinary run.
- Same five native packages: **five doctests** and warning-denying all-target
  Clippy passed.
- `bash crates/umbriel-protocol/tests/native/check-qualified.sh ~/src/umbriel-working`:
  **48 actual Rust action strings and complete requests** checked against native
  `parseAction`, `actionRequest` and `actionFromRequest`, including all eight
  sizing forms and representative quoted/Unicode/NUL IDs. Matching only the
  human parser would have missed the structured-wire requirement.
- Native `meson test -C build-debug keybind-parse --print-errorlogs`: passed.
  `just check layout/id_resize_actions`: passed against an isolated headless
  compositor, including stale IDs and unchanged seat focus. It took 15.18 s,
  over that harness's advisory eight-second budget, but did not fail.
- `bash desktop/umbriel/tests/native/check.sh`: **passed** on its own compositor.
  Actual stock families project an eligible two-member group; both impossible-ID
  structured capability probes succeed by their exact rejection; a real Rust
  qualified edge is handled and its next-row target movement observed. Seat focus
  and unrelated anchor geometry remain unchanged. This verifies the A boundary,
  not Taffy feedback or final content convergence. The Rust test is opt-in/ignored
  in ordinary runs and refuses to run without explicit harness markers.
- `cargo nextest run --locked -p zooming-kittens -p zk-desktop -p zk-niri -p zk-umbriel -p zk-longcat -p zk-layout`:
  **130 passed**. This includes the existing niri and bespoke runner tests plus
  new stock geometry/presentation/family-skew/qualified-command fixtures.
- ZK desktop all-target Clippy passed with denied warnings. Scripted sockets
  establish no stale-ID fallback, unknown-grammar denial, live observation during
  a pending command, no waiting command queue, loss with retained receipt, and
  mandatory re-verification across reconnect epochs. The native client tests
  separately establish no bytes after a failed post-connect local precondition.
- Umbriel-only and niri-only application all-target checks passed. The latter
  reports the existing unused optional `tokio-stream` manifest dependency warning;
  the default-feature strict Clippy scope is clean. D still owns the full final
  feature matrix after its runner changes.

## Continue with B, then C/D

1. Add the pure revisable `Feedback` preference/attempt state beside `Episode`;
   do not route stock guidance through irreversible conflict/restoration states.
   The numerical guidance and lifecycle tests are already commissioned by the
   selected design. No further design wave or human choice is needed.
2. C owns quiet content-budget/strip extraction and the frozen execution model.
   **Do not reacquire constants from mixed-time frames.** A's candidate axis can
   become unavailable when target positions overlap stale committed sizes during
   our resize. Current `edge::prepare` deliberately requires current candidate
   eligibility. C should extend that admission with its validated frozen geometry
   authority if it wants to advance on position response before all client commits;
   do not globally weaken unknown-axis admission or invent reported target sizes.
3. C/D still need the timer-driven one-off windows refresh/installation path;
   A exposes typed `Query::Windows` but adds no synthetic observation ordering or
   query-to-subscription splice. Keep native Inventory/Tracker as the row owner.
4. D changes ordinary Umbriel startup to stock plus qualified sizing, adds the
   feedback bridge/scheduler/diagnostics, `minimum_width`, shutdown and native
   end-to-end controller acceptance. Current runner startup still probes the
   bespoke extension first; it has not been silently changed by this slice.

Readiness and `Impact` are not quietness: impact includes activation and health.
B/C must compare the relevant geometry/context facts and own monotonic quiet/
progress/convergence timers. No preference formula, solver allocation, gain
estimate, donor selection, user-attribution decision or convergence claim is
implemented by A. The work anchor and user-layout task remain open for that work.
