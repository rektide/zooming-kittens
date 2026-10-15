use std::{sync::Arc, time::Duration};
use umbriel_client::Observation;
use zk_desktop::*;
use zk_umbriel::Reducer;

fn reducer() -> Reducer {
    let mut r = Reducer::new(SourceId::new(Backend::Umbriel, "script"));
    r.apply(Duration::ZERO, Observation::Epoch { epoch: 1 })
        .unwrap();
    r
}
fn frame(epoch: u64, position: u64, text: String) -> Observation {
    Observation::Frame {
        epoch,
        position,
        bytes: Arc::from(text.as_bytes()),
    }
}
fn window(id: &str, workspace: &str, active: bool, focused: bool) -> String {
    format!(
        r#"{{"id":"{id}","workspace":"{workspace}","scratchpad":"","active":{active},"app_id":"app","title":"title","xdg_tag":"","content_type":"none","floating":false,"focused":{focused},"urgent":false,"xwayland":false,"pid":-1,"x":30,"y":40,"w":800,"h":400}}"#
    )
}
fn windows(rows: &[String]) -> String {
    format!(r#"{{"event":"windows","data":[{}]}}"#, rows.join(","))
}
fn workspaces() -> String {
    r#"{"event":"workspaces","data":[{"id":"old","name":"one","named":false,"index":1,"output":"display","active":true,"focused":true,"occupied":true,"layout":"scrolling"}]}"#.into()
}
fn apply(r: &mut Reducer, position: u64, text: String) -> DesktopUpdate {
    r.apply(Duration::ZERO, frame(1, position, text))
        .unwrap()
        .unwrap()
}

#[test]
fn seat_activation_not_workspace_remembered_focus_and_no_invented_columns() {
    let mut r = reducer();
    let update = apply(&mut r, 1, windows(&[window("A", "old", false, true)]));
    assert!(update.after.active_window.is_none());
    assert!(update.after.groups.is_empty());
    assert!(!update.after.capabilities.height_by_id);
    assert_eq!(
        update
            .after
            .windows
            .values()
            .next()
            .unwrap()
            .geometry
            .target_position,
        Some(Point { x: 30., y: 40. })
    );
    assert_eq!(
        update
            .after
            .windows
            .values()
            .next()
            .unwrap()
            .geometry
            .content_size,
        Some(Size {
            width: 800.,
            height: 400.
        })
    );
    apply(&mut r, 2, workspaces());
    assert!(r.capture().is_ready());
    assert_eq!(
        r.capture().workspaces.values().next().unwrap().grouping,
        Evidence::Unavailable
    );
    let activate = apply(&mut r, 3, windows(&[window("A", "old", true, false)]));
    assert!(activate.impact.focus_changed);
    assert_eq!(
        activate.after.active_window.as_ref().unwrap().native,
        NativeId::Text("A".into())
    );
    let blur = apply(&mut r, 4, windows(&[window("A", "old", false, true)]));
    assert!(blur.impact.focus_changed);
    assert!(blur.after.active_window.is_none());
}

#[test]
fn replacement_reports_move_deletion_and_equal_census_without_spurious_impact() {
    let mut r = reducer();
    apply(&mut r, 1, windows(&[window("A", "old", true, true)]));
    let moved = apply(&mut r, 2, windows(&[window("A", "new", true, true)]));
    assert_eq!(
        moved
            .impact
            .workspaces
            .iter()
            .map(|w| w.native.clone())
            .collect::<Vec<_>>(),
        vec![NativeId::Text("new".into()), NativeId::Text("old".into())]
    );
    let equal = apply(&mut r, 3, windows(&[window("A", "new", true, true)]));
    assert!(equal.impact.windows.is_empty());
    assert!(equal.impact.workspaces.is_empty());
    assert_eq!(equal.continuity, Continuity::Continuous);
    let removed = apply(&mut r, 4, windows(&[]));
    assert!(removed.after.windows.is_empty());
    assert_eq!(
        removed.impact.workspaces.iter().next().unwrap().native,
        NativeId::Text("new".into())
    );
}

#[test]
fn invalid_typed_census_retains_rows_and_family_stamp_then_valid_replacement_reacquires() {
    let mut r = reducer();
    apply(&mut r, 1, windows(&[window("A", "old", true, true)]));
    apply(&mut r, 2, workspaces());
    let old = r.capture();
    let malformed = apply(&mut r, 3, r#"{"event":"windows","data":null}"#.into());
    assert!(!malformed.after.is_ready());
    assert_eq!(malformed.after.windows, old.windows);
    assert_eq!(
        malformed.after.windows_status.stamp,
        old.windows_status.stamp
    );
    assert!(malformed.after.workspaces_status.is_current());
    let duplicate = apply(
        &mut r,
        4,
        windows(&[
            window("B", "new", false, false),
            window("B", "new", false, false),
        ]),
    );
    assert_eq!(duplicate.after.windows, old.windows);
    let valid = apply(&mut r, 5, windows(&[window("B", "new", false, false)]));
    assert!(valid.after.is_ready());
    assert_eq!(
        valid.continuity,
        Continuity::Discontinuous(LossReason::InvalidSnapshot)
    );
    assert_eq!(valid.impact.workspaces.len(), 2);
}

#[test]
fn new_epoch_preserves_retained_row_identity_and_independent_family_readiness() {
    let mut r = reducer();
    apply(&mut r, 1, windows(&[window("A", "old", true, true)]));
    apply(&mut r, 2, workspaces());
    r.loss(LossReason::Disconnected);
    assert!(!r.capture().is_ready());
    let denied = apply(&mut r, 3, windows(&[]));
    assert_eq!(denied.after.windows.len(), 1);
    r.apply(Duration::ZERO, Observation::Epoch { epoch: 2 })
        .unwrap();
    assert_eq!(
        r.capture().windows.keys().next().unwrap().scope.epoch,
        Epoch(1)
    );
    let update = r
        .apply(
            Duration::ZERO,
            frame(2, 1, windows(&[window("A", "old", true, true)])),
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        update.after.windows.keys().next().unwrap().scope.epoch,
        Epoch(2)
    );
    assert_eq!(
        update.after.workspaces.keys().next().unwrap().scope.epoch,
        Epoch(1)
    );
    assert_eq!(
        update.continuity,
        Continuity::Discontinuous(LossReason::EpochChanged)
    );
    assert_eq!(update.impact.workspaces.len(), 2);
    assert!(!update.after.is_ready());
    r.apply(Duration::ZERO, frame(2, 2, workspaces())).unwrap();
    assert!(r.capture().is_ready());
    assert!(
        r.apply(Duration::ZERO, Observation::Epoch { epoch: 2 })
            .is_err()
    );
}

#[test]
fn scratchpad_unknown_workspace_and_empty_census_are_representable() {
    let mut r = reducer();
    let pad = window("A", "", false, true).replace("\"scratchpad\":\"\"", "\"scratchpad\":\"pad\"");
    let update = apply(
        &mut r,
        1,
        windows(&[pad, window("B", "missing", false, false)]),
    );
    assert_eq!(
        update.after.windows.values().next().unwrap().placement,
        Placement::Scratchpad("pad".into())
    );
    assert!(
        update
            .after
            .windows
            .values()
            .next()
            .unwrap()
            .workspace
            .is_none()
    );
    assert_eq!(update.impact.workspaces.len(), 1);
    let empty = apply(&mut r, 2, windows(&[]));
    assert!(empty.after.windows_status.is_current());
    assert!(empty.after.windows.is_empty());
}

#[test]
fn malformed_envelope_and_ambiguous_activation_cannot_make_fresh_empty_state() {
    let mut r = reducer();
    apply(&mut r, 1, windows(&[window("A", "old", true, true)]));
    apply(&mut r, 2, workspaces());
    let invalid = apply(&mut r, 3, "{".into());
    assert!(!invalid.after.windows_status.is_current());
    assert!(!invalid.after.workspaces_status.is_current());
    let ambiguous = apply(
        &mut r,
        4,
        windows(&[
            window("A", "old", true, false),
            window("B", "old", true, false),
        ]),
    );
    assert_eq!(ambiguous.after.windows.len(), 1);
    assert!(
        r.apply(Duration::ZERO, frame(1, 5, String::new()))
            .unwrap()
            .is_none()
    );
}
