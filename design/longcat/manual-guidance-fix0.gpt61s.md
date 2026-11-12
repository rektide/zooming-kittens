---
type: ImplementationReport
title: Manual resize guidance across primary-envelope motion
description: Live-discovered snapback correction separating preference context from frozen execution geometry, with focused runner and native corner-drag evidence.
resource: design/longcat/manual-guidance-fix0.gpt61s.md
status: implemented-awaiting-live-retest
generated: { at: 2026-11-12, by: 'model:openai/gpt-6.1-sol#high' }
sources:
  - resource: opencode://ses_e310e7816ffeno1fBCUgYLykPq
    title: Reproduce simultaneous-axis snapback, preserve semantic guidance while revoking execution geometry, and verify focused Rust/native corner-drag checks without touching the live desktop
  - resource: ../../app/zk/src/runner/feedback.rs
    title: Stock preference-context projection, primary motion tracking and frozen execution invalidation
  - resource: ../../app/zk/src/runner/stock_tests.rs
    title: Simultaneous-axis, primary-only, pending-command and real context-reset regressions
  - resource: ../../layout/longcat/src/feedback/mod.rs
    title: Existing context-reset, settled reference, yield, coherence and receipt semantics; unchanged
  - resource: ../../layout/longcat/src/feedback/preferences.rs
    title: Existing normalized-share guidance and hidden-neutral protection; unchanged
  - resource: ../../desktop/umbriel/src/geometry.rs
    title: Frozen primary/output/presentation authority, native compilation and late-commit scope; unchanged
  - resource: ../../app/zk/tests/headless/stock_feedback.py
    title: Real executable with native held corner drag, both-axis changes, no corrective writes and learned focus return
  - resource: ../../app/zk/src/runner/README.md
    title: Maintained runner contract updated with the context/geometry distinction
  - resource: implementation-e0.gpt61s.md
    title: Earlier native acceptance and the narrower middle-bottom pointer coverage
verification:
  method: Failing predecessor regression, affected Nextest scope, strict Clippy, owned formatting, isolated native basic slice and Rekon document checks.
  result: 146 affected Rust tests pass, including 23 stock-runner tests; native basic passes with actual primary width change and remembered manual secondary allocation.
  limits: No live desktop mutation, installation, service change, push or public API/package change. Parent owns bounded live retest. Full historical native matrix not rerun.
extensions:
  ticket: zk-longcat-user-layout-changes
---

# Outcome

**Manual two-axis resizing no longer discards the learned-layout opportunity.**
The correction is internal to the stock adapter: primary movement revokes native
execution authority while preserving the settled semantic reference used to learn
the simultaneous secondary edit. It does not relax frozen geometry checks or add
a stock user-input claim to pure Feedback.

## Failure and bounded correction

The parent supplied actual interactive evidence in
`.test-agent/live-longcat/interactive.jsonl`: an equal `[1064,1064]` stack settled
to `[1490,638]`, but width/x edits changed each member's primary coordinates in
`raw_group.context`. That recreated `Slot` and pure `GroupState` together, losing
the expected/reference vector before the secondary edit could be accepted. The
captured run had five acquisitions/eight attempts settled but zero guidance.

The first new regression failed on the predecessor: after the manually selected
`[1200,928]` became quiet, it received another **+16px** native correction rather
than accepted guidance. That failing checkpoint is `f128d87b`.

The fix (`f6c0b458`) separates two scopes:

- **Preference context:** reported output/presentation/placement plus axis and
  ordered identity scope. Real changes still reset the group's reference and
  reacquire, without teaching their changed secondary geometry as a manual share.
- **Primary execution geometry:** per-member primary start/extent, held privately
  by `Slot`. Any exact movement drops `FrozenGeometry`, its calibration/position
  timer, owned prediction and coupled-response hold. Both primary and secondary
  changes restart the adapter quiet timer. With no model, the pure observation has
  no target estimates and is incoherent, so no stale goal can drive or learn early.

After both axes quiet and any native receipt drains, acquisition creates fresh
strips/budget/authority. Pure Feedback's unchanged yield path uses its retained
expected/emphasized reference to learn a changed secondary share. A width-only
edit changes no secondary vector and teaches no active share. Pending overlap is
explicitly unattributed, not stamped as a response from the retired model.

## Evidence and checks

- Simultaneous edits use one capture, then primary-only movement continues past
  the secondary quiet deadline. Zero writes through motion; one guidance event
  only on the final version; one preference acquisition; neutral weight stays
  1064 and focus return reproduces 1200px.
- Primary-only movement with a focus change is covered on both axes: no writes
  during movement, no guidance/share invention, and the replacement model starts
  with a fresh capped 16px probe rather than old empirical scale.
- An outstanding command drains before new geometry can learn or execute; its
  old tail cannot cross the envelope change. Output/presentation changes remain
  separate reference resets, and existing coupled/late-commit guards still pass.
- `cargo nextest run --locked -p zk-longcat -p zk-umbriel -p zooming-kittens
  --no-default-features --features umbriel`: **146 passed**, two feature-disabled
  tests skipped. Includes **23 stock-runner tests**.
- Scoped `cargo clippy --locked -p zooming-kittens --no-default-features --features
  umbriel --all-targets -- -D warnings` passes; Cargo separately reports its
  pre-existing optional unused `tokio-stream` manifest warning. Owned Rust files
  use default rustfmt config (`--config-path /dev/null`), matching their prior style.

The actual native pointer test now starts in the bottom-right **corner**, not the
middle-bottom thirds zone. Merely adding x motion to the old grab did not change
width; an assertion caught that insufficient fixture. The corner changes width
**480 → 519** and contents **`[290,163,62]` → `[238,215,62]`**, keeps the secondary
outer envelope/strips and unrelated anchor unchanged, submits no corrective edges
during the >500ms held motion, learns `.4621359223…`, and reproduces the layout on
focus return. The isolated `basic` slice passes in **14.86s** on the same native
`d34b33c77752` binary as E. Passing evidence is
`.test-agent/longcat-feedback-e/native.DL6GJwFE/`; the insufficient-grab assertion
receipt remains in `native.ToxDe7N3/`. No compositor code was changed.

```sh
cargo build --locked -p zooming-kittens --no-default-features --features umbriel
ZK_STOCK_CASE=basic ZK_TEST_BIN="$PWD/target/debug/zk" \
  CHECK_DIR="$PWD/app/zk/tests/headless" \
  bash ../umbriel-working/tests/harness/check.sh \
  ../umbriel-working/build-debug/umbriel umbriel_stock_feedback -j 1 -v
```

## Parent live retest

Run the corrected executable under the parent's bounded interactive capture; do
not infer that this headless receipt has already changed the installed service.
After initial emphasis settles, drag a corner to change both width and height,
continue width-only motion briefly, then release. Confirm no corrective intents
during motion, guidance increments after trailing quietness rather than fresh
preference acquisition, the selected share stays put, and focus away/back returns
to it. A width-only edit should retain that share without teaching a new one.
Stock snapshots still cannot prove causality for every concurrent client change.
