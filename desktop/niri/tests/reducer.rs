use niri_ipc::{Event, Window, WindowLayout, Workspace};
use zk_desktop::*;
use zk_niri::{Limits, Reducer};

fn reducer() -> Reducer {
    Reducer::new(
        SourceId::new(Backend::Niri, "script"),
        Epoch(1),
        Limits::default(),
    )
}
fn layout(col: usize, row: usize, height: i32) -> WindowLayout {
    WindowLayout {
        pos_in_scrolling_layout: Some((col, row)),
        tile_size: (800., height as f64 + 2.),
        window_size: (798, height),
        tile_pos_in_workspace_view: Some((0., row as f64 * 400.)),
        window_offset_in_tile: (1., 1.),
    }
}
fn window(id: u64, workspace: u64, row: usize, focused: bool) -> Window {
    Window {
        id,
        title: Some(format!("window {id}")),
        app_id: Some("test".into()),
        pid: Some(7),
        workspace_id: Some(workspace),
        is_focused: focused,
        is_floating: false,
        is_urgent: false,
        layout: layout(1, row, 400),
        focus_timestamp: None,
    }
}
fn workspace(id: u64, output: &str, focused: bool) -> Workspace {
    Workspace {
        id,
        idx: id as u8,
        name: None,
        output: Some(output.into()),
        is_urgent: false,
        is_active: focused,
        is_focused: focused,
        active_window_id: None,
    }
}

#[test]
fn unknown_and_known_empty_census_are_distinct() {
    let mut r = reducer();
    assert_eq!(r.capture().windows_status.health, Health::Unseen);
    assert!(!r.capture().is_ready());
    r.apply(Event::WindowsChanged { windows: vec![] }).unwrap();
    assert_eq!(r.capture().windows_status.health, Health::Current);
    r.apply(Event::WorkspacesChanged { workspaces: vec![] })
        .unwrap();
    assert!(r.capture().is_ready());
    assert!(r.capture().active_window.is_none());
}

#[test]
fn no_focus_open_close_and_metadata_changes_preserve_semantic_impact() {
    let mut r = reducer();
    r.apply(Event::WindowsChanged {
        windows: vec![window(1, 10, 1, true), window(2, 10, 2, false)],
    })
    .unwrap();
    let blur = r.apply(Event::WindowFocusChanged { id: None }).unwrap();
    assert!(blur.impact.focus_changed);
    assert!(blur.after.active_window.is_none());
    assert_eq!(blur.impact.groups.len(), 1);
    let open = r
        .apply(Event::WindowOpenedOrChanged {
            window: window(3, 20, 1, true),
        })
        .unwrap();
    assert_eq!(
        open.after.active_window.as_ref().unwrap().native,
        NativeId::Number(3)
    );
    let mut changed = window(3, 20, 1, true);
    changed.title = Some("renamed".into());
    let metadata = r
        .apply(Event::WindowOpenedOrChanged { window: changed })
        .unwrap();
    assert!(metadata.impact.windows.is_empty());
    let close = r.apply(Event::WindowClosed { id: 3 }).unwrap();
    assert!(close.after.active_window.is_none());
    assert_eq!(
        close.impact.workspaces.iter().next().unwrap().native,
        NativeId::Number(20)
    );
    assert_eq!(close.impact.groups.len(), 1);
    assert!(close.before.windows.len() > close.after.windows.len());
}

#[test]
fn focused_open_clears_previous_active_and_cross_workspace_same_ordinal_is_not_same_group() {
    let mut r = reducer();
    r.apply(Event::WindowsChanged {
        windows: vec![window(1, 10, 1, true)],
    })
    .unwrap();
    let update = r
        .apply(Event::WindowOpenedOrChanged {
            window: window(2, 20, 1, true),
        })
        .unwrap();
    assert_eq!(
        update.after.windows.values().filter(|w| w.active).count(),
        1
    );
    assert_eq!(update.impact.groups.len(), 2);
    assert_eq!(update.impact.workspaces.len(), 2);
    let moved = r
        .apply(Event::WindowOpenedOrChanged {
            window: window(1, 20, 2, false),
        })
        .unwrap();
    assert_eq!(moved.impact.workspaces.len(), 2);
    assert_eq!(moved.impact.groups.len(), 2);
    assert_eq!(moved.after.groups.len(), 1);
    assert_eq!(
        moved
            .after
            .groups
            .values()
            .next()
            .unwrap()
            .members
            .iter()
            .map(|w| w.native.clone())
            .collect::<Vec<_>>(),
        vec![NativeId::Number(2), NativeId::Number(1)]
    );
}

#[test]
fn layout_event_is_one_coherent_capture_and_renumbering_keeps_member_evidence() {
    let mut r = reducer();
    r.apply(Event::WindowsChanged {
        windows: vec![window(1, 10, 1, true), window(2, 10, 2, false)],
    })
    .unwrap();
    let before = r.capture();
    let update = r
        .apply(Event::WindowLayoutsChanged {
            changes: vec![(1, layout(2, 2, 630)), (2, layout(2, 1, 162))],
        })
        .unwrap();
    assert_eq!(update.after.version.0, before.version.0 + 1);
    assert_eq!(update.impact.windows.len(), 2);
    assert_eq!(update.impact.groups.len(), 2);
    let group = update.after.groups.values().next().unwrap();
    assert_eq!(group.members[0].native, NativeId::Number(2));
    assert_eq!(group.membership, Evidence::Reported);
    assert_eq!(group.geometry_evidence, Evidence::Inferred);
    assert_eq!(
        before
            .windows
            .values()
            .next()
            .unwrap()
            .geometry
            .content_size
            .unwrap()
            .height,
        400.
    );
}

#[test]
fn duplicate_rows_and_partial_unknown_layout_reject_without_partial_installation() {
    let mut r = reducer();
    r.apply(Event::WindowsChanged {
        windows: vec![window(1, 10, 1, true)],
    })
    .unwrap();
    let before = r.capture();
    assert!(
        r.apply(Event::WindowLayoutsChanged {
            changes: vec![(1, layout(1, 1, 700)), (99, layout(1, 2, 200))]
        })
        .is_err()
    );
    assert_eq!(r.capture().windows, before.windows);
    assert!(!r.capture().connected);
    assert!(matches!(
        r.capture().windows_status.health,
        Health::Stale(LossReason::InvalidSnapshot)
    ));
    r.begin_epoch(Epoch(2)).unwrap();
    assert!(
        r.apply(Event::WindowsChanged {
            windows: vec![window(2, 20, 1, false), window(2, 20, 2, false)]
        })
        .is_err()
    );
    assert_eq!(r.capture().windows, before.windows);
}

#[test]
fn loss_retains_old_identity_but_requires_new_epoch_and_censuses() {
    let mut r = reducer();
    r.apply(Event::WindowsChanged {
        windows: vec![window(1, 10, 1, true)],
    })
    .unwrap();
    r.apply(Event::WorkspacesChanged {
        workspaces: vec![workspace(10, "one", true)],
    })
    .unwrap();
    let old = r.capture().active_window.clone().unwrap();
    r.loss(LossReason::Disconnected);
    assert!(!r.capture().is_ready());
    assert!(r.apply(Event::WindowFocusChanged { id: None }).is_err());
    r.begin_epoch(Epoch(2)).unwrap();
    assert_eq!(r.capture().windows.keys().next().unwrap(), &old);
    let fresh = r
        .apply(Event::WindowsChanged {
            windows: vec![window(1, 10, 1, false)],
        })
        .unwrap();
    assert_ne!(fresh.after.windows.keys().next().unwrap(), &old);
    assert_eq!(fresh.impact.workspaces.len(), 2);
    assert!(!fresh.after.is_ready());
    r.apply(Event::WorkspacesChanged {
        workspaces: vec![workspace(10, "one", true)],
    })
    .unwrap();
    assert!(r.capture().is_ready());
    assert!(r.begin_epoch(Epoch(2)).is_err());
}

#[test]
fn workspace_activation_is_per_output_and_does_not_invent_seat_window_focus() {
    let mut r = reducer();
    r.apply(Event::WorkspacesChanged {
        workspaces: vec![
            workspace(1, "one", true),
            workspace(2, "one", false),
            workspace(3, "two", false),
        ],
    })
    .unwrap();
    r.apply(Event::WorkspaceActivated {
        id: 3,
        focused: false,
    })
    .unwrap();
    let update = r
        .apply(Event::WorkspaceActivated {
            id: 2,
            focused: true,
        })
        .unwrap();
    assert!(update.after.active_window.is_none());
    assert_eq!(
        update
            .after
            .workspaces
            .values()
            .filter(|w| w.active)
            .count(),
        2
    );
    assert_eq!(
        update
            .after
            .workspaces
            .values()
            .filter(|w| w.focused)
            .count(),
        1
    );
    assert_eq!(
        update
            .after
            .workspaces
            .values()
            .find(|w| w.focused)
            .unwrap()
            .key
            .native,
        NativeId::Number(2)
    );
}

#[test]
fn ambiguous_rows_do_not_claim_order_and_floating_windows_do_not_enter_groups() {
    let mut r = reducer();
    let mut floating = window(3, 10, 3, false);
    floating.is_floating = true;
    r.apply(Event::WindowsChanged {
        windows: vec![window(1, 10, 1, true), window(2, 10, 1, false), floating],
    })
    .unwrap();
    let capture = r.capture();
    assert_eq!(
        capture.groups.values().next().unwrap().membership,
        Evidence::Unavailable
    );
    assert_eq!(capture.groups.values().next().unwrap().members.len(), 2);
}
