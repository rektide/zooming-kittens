mod common;
use common::*;
use serde_json::json;
use zk_desktop::*;
use zk_umbriel::edge;

#[test]
fn reported_workspace_scoped_columns_and_reordering_preserve_member_identity() {
    let mut h = Harness::new();
    let a = window("z-first", "ws", 0);
    let b = window("a-second", "ws", 1);
    let mut c = window("other", "elsewhere", 0);
    c["active"] = json!(false);
    h.send("windows", json!([b, c, a]));
    h.send("outputs", json!([output("OUT")]));
    h.send(
        "workspaces",
        json!([workspace("ws", "OUT"), workspace("elsewhere", "OUT")]),
    );
    let before = h.reducer.capture();
    assert!(before.is_ready());
    assert_eq!(before.groups.len(), 2);
    let g = before
        .groups
        .values()
        .find(|g| g.members.len() == 2)
        .unwrap();
    assert_eq!(g.members[0].native, NativeId::Text("z-first".into()));
    assert_eq!(g.members[1].native, NativeId::Text("a-second".into()));
    assert_eq!(g.membership, Evidence::Reported);
    assert_eq!(g.geometry_evidence, Evidence::Inferred);
    assert_eq!(g.secondary_axis, Some(Axis::Vertical));
    assert_eq!(g.edge_eligibility, EdgeEligibility::Eligible);
    assert!(!before.capabilities.height_by_id);
    assert!(!before.capabilities.atomic_multiwindow);
    assert!(!before.capabilities.exact_restore);
    assert!(
        before
            .windows
            .values()
            .all(|w| w.geometry.tile_size.is_none())
    );
    let mut a = window("z-first", "ws", 1);
    a["active"] = json!(true);
    let mut b = window("a-second", "ws", 0);
    b["active"] = json!(false);
    let after = h.send("windows", json!([a, b]));
    let new = &after.after.groups[&g.key];
    assert_eq!(
        new.members,
        g.members.iter().rev().cloned().collect::<Vec<_>>()
    );
    assert!(after.impact.groups.contains(&g.key));
    assert_eq!(
        before.windows[&g.members[0]].key,
        after.after.windows[&g.members[0]].key
    );
}

#[test]
fn presentation_and_axis_transitions_park_and_resume_without_losing_groups() {
    let mut h = Harness::ready();
    for flag in [
        "tabbed",
        "tab_hidden",
        "fullscreen",
        "maximized",
        "maximized_to_edges",
        "pinned",
        "minimized",
        "floating",
    ] {
        let mut a = window("upper", "ws", 0);
        a[flag] = json!(true);
        let bad = h.send("windows", json!([a, window("lower", "ws", 1)]));
        let g = bad.after.groups.values().next().unwrap();
        assert_eq!(g.membership, Evidence::Reported, "{flag}");
        assert_eq!(
            g.edge_eligibility,
            EdgeEligibility::UnsupportedPresentation,
            "{flag}"
        );
        assert!(edge::prepare(&bad.after, &h.request(0.1)).is_err());
        let good = h.send(
            "windows",
            json!([window("upper", "ws", 0), window("lower", "ws", 1)]),
        );
        assert_eq!(
            good.after.groups.values().next().unwrap().edge_eligibility,
            EdgeEligibility::Eligible
        );
    }
    let mut a = window("upper", "ws", 0);
    let mut b = window("lower", "ws", 1);
    a["w"] = json!(300);
    b["w"] = json!(300);
    b["x"] = json!(340);
    b["y"] = json!(40);
    let horizontal = h.send("windows", json!([a.clone(), b.clone()]));
    assert_eq!(
        horizontal
            .after
            .groups
            .values()
            .next()
            .unwrap()
            .secondary_axis,
        Some(Axis::Horizontal)
    );
    assert_eq!(
        edge::prepare(&horizontal.after, &h.request(-0.1))
            .unwrap()
            .as_str(),
        "window-modify-width-right -0.1 --window \"upper\""
    );
    b["x"] = json!(100);
    let overlap = h.send("windows", json!([a.clone(), b.clone()]));
    assert_eq!(
        overlap.after.groups.values().next().unwrap().secondary_axis,
        None
    );
    assert_eq!(
        overlap
            .after
            .groups
            .values()
            .next()
            .unwrap()
            .edge_eligibility,
        EdgeEligibility::AmbiguousAxis
    );
    b["row"] = json!(0);
    let duplicate = h.send("windows", json!([a.clone(), b]));
    assert_eq!(
        duplicate.after.groups.values().next().unwrap().membership,
        Evidence::Unavailable
    );
    a.as_object_mut().unwrap().remove("maximized");
    let unknown = h.send("windows", json!([a, window("lower", "ws", 1)]));
    assert_eq!(
        unknown
            .after
            .groups
            .values()
            .next()
            .unwrap()
            .edge_eligibility,
        EdgeEligibility::UnsupportedPresentation
    );
}

#[test]
fn independent_family_skew_output_loss_and_epoch_changes_never_authorize_writes() {
    let mut h = Harness::new();
    h.send(
        "windows",
        json!([window("upper", "ws", 0), window("lower", "ws", 1)]),
    );
    h.send("workspaces", json!([workspace("ws", "OUT")]));
    assert!(!h.reducer.capture().is_ready());
    assert!(edge::prepare(&h.reducer.capture(), &h.request(0.1)).is_err());
    h.send("outputs", json!([output("OUT")]));
    assert!(edge::prepare(&h.reducer.capture(), &h.request(0.1)).is_ok());
    let stale = h.send("outputs", json!(null));
    assert!(!stale.after.is_ready());
    assert!(stale.after.windows_status.is_current());
    assert_eq!(stale.after.outputs.len(), 1);
    assert_eq!(
        stale.after.groups.values().next().unwrap().edge_eligibility,
        EdgeEligibility::FamilyUnavailable
    );
    h.send("outputs", json!([output("OUT")]));
    h.send("workspaces", json!([workspace("ws", "NEW")]));
    assert!(h.reducer.capture().is_ready()); // Families current, associations not yet consistent.
    assert!(edge::prepare(&h.reducer.capture(), &h.request(0.1)).is_err());
    h.send("outputs", json!([output("NEW")]));
    let mut a = window("upper", "ws", 0);
    let mut b = window("lower", "ws", 1);
    a["output"] = json!("NEW");
    b["output"] = json!("NEW");
    h.send("windows", json!([a, b]));
    assert!(edge::prepare(&h.reducer.capture(), &h.request(0.1)).is_ok());
    let gone = h.send("outputs", json!([]));
    assert_eq!(
        gone.after.groups.values().next().unwrap().edge_eligibility,
        EdgeEligibility::OutputUnavailable
    );
    assert!(edge::prepare(&gone.after, &h.request(0.1)).is_err());
    h.reducer
        .apply(
            std::time::Duration::ZERO,
            umbriel_client::Observation::Epoch { epoch: 2 },
        )
        .unwrap();
    assert_eq!(
        h.reducer
            .capture()
            .windows
            .keys()
            .next()
            .unwrap()
            .scope
            .epoch,
        Epoch(1)
    );
    assert!(!h.reducer.capture().is_ready());
}

#[test]
fn metadata_and_unrelated_outputs_do_not_restart_group_geometry_quietness() {
    let mut h = Harness::ready();
    let mut a = window("upper", "ws", 0);
    a["title"] = json!("new title");
    let title = h.send("windows", json!([a, window("lower", "ws", 1)]));
    assert!(title.impact.groups.is_empty());
    assert!(title.impact.windows.is_empty());
    assert!(title.impact.workspaces.is_empty());
    let extra = h.send("outputs", json!([output("OUT"), output("UNRELATED")]));
    assert_eq!(extra.impact.outputs.len(), 1);
    assert!(extra.impact.groups.is_empty());
    let mut out = output("OUT");
    out["scale"] = json!(1.25);
    let changed = h.send("outputs", json!([out, output("UNRELATED")]));
    assert_eq!(changed.impact.groups.len(), 1);
    assert_eq!(changed.impact.outputs.len(), 1);
}

#[test]
fn only_latest_finite_qualified_internal_cross_end_commands_compile() {
    let mut h = Harness::ready();
    let c = h.reducer.capture();
    let good = h.request(-0.125);
    assert_eq!(
        edge::prepare(&c, &good).unwrap().as_str(),
        "window-modify-height-down -0.125 --window \"upper\""
    );
    for fraction in [0., f64::NAN, f64::INFINITY, -0.91, 0.91] {
        assert!(
            edge::prepare(
                &c,
                &RelativeEdgeRequest {
                    fraction,
                    ..good.clone()
                }
            )
            .is_err()
        );
    }
    let mut outer = good.clone();
    std::mem::swap(&mut outer.target, &mut outer.following);
    assert!(edge::prepare(&c, &outer).is_err());
    let mut wrong_axis = good.clone();
    wrong_axis.axis = Axis::Horizontal;
    assert!(edge::prepare(&c, &wrong_axis).is_err());
    let mut wrong_epoch = good.clone();
    wrong_epoch.target.scope.epoch = Epoch(2);
    assert!(edge::prepare(&c, &wrong_epoch).is_err());
    h.send(
        "windows",
        json!([window("upper", "ws", 0), window("lower", "ws", 1)]),
    );
    assert!(edge::prepare(&h.reducer.capture(), &good).is_err());
}

#[test]
fn known_nonmembers_do_not_hide_survivors_but_unknown_membership_does() {
    let mut h = Harness::ready();
    let mut pinned = window("pinned", "ws", -1);
    pinned["column"] = json!(-1);
    pinned["pinned"] = json!(true);
    h.send(
        "windows",
        json!([
            window("upper", "ws", 0),
            window("lower", "ws", 1),
            pinned.clone()
        ]),
    );
    assert!(edge::prepare(&h.reducer.capture(), &h.request(0.1)).is_ok());
    pinned.as_object_mut().unwrap().remove("column");
    let unknown = h.send(
        "windows",
        json!([window("upper", "ws", 0), window("lower", "ws", 1), pinned]),
    );
    assert_eq!(
        unknown.after.groups.values().next().unwrap().membership,
        Evidence::Unavailable
    );
    assert!(edge::prepare(&unknown.after, &h.request(0.1)).is_err());
}
