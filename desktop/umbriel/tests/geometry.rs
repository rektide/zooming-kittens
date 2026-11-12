mod common;
use common::*;
use serde_json::json;
use zk_desktop::*;
use zk_umbriel::{edge, geometry::*};

fn census(
    h: &mut Harness,
    starts: &[f64],
    extents: &[f64],
    horizontal: bool,
) -> std::sync::Arc<DesktopCapture> {
    let windows: Vec<_> = starts
        .iter()
        .zip(extents)
        .enumerate()
        .map(|(i, (p, e))| {
            let mut w = window(&format!("w{i}"), "ws", i as i32);
            if horizontal {
                w["y"] = json!(20);
                w["h"] = json!(800);
            }
            w[if horizontal { "x" } else { "y" }] = json!(*p as i32);
            w[if horizontal { "w" } else { "h" }] = json!(*e as i32);
            w
        })
        .collect();
    h.send("windows", json!(windows)).after
}

fn initial(horizontal: bool) -> (Harness, FrozenGeometry) {
    let mut h = Harness::ready();
    let c = census(&mut h, &[50., 170., 405.], &[100., 200., 300.], horizontal);
    let g = c.groups.keys().next().unwrap();
    let model = FrozenGeometry::acquire(&c, g, 2).unwrap();
    (h, model)
}

#[test]
fn measured_heterogeneous_strips_preserve_outer_envelope_on_both_axes() {
    for horizontal in [false, true] {
        let (h, model) = initial(horizontal);
        assert_eq!(model.content_budget(), 600.);
        assert_eq!(model.strips(), [20., 35.]);
        assert_eq!(model.envelope(), (50., 705.));
        assert_eq!(
            model.desired_starts(&[300., 100., 200.]).unwrap(),
            [50., 370., 505.]
        );
        let compiled = model.compile(&h.reducer.capture(), 0, 200.).unwrap();
        assert_eq!(compiled.travel, 16.);
        let action =
            edge::prepare_frozen(&h.reducer.capture(), &compiled.request, &compiled.authority)
                .unwrap();
        assert!(action.as_str().starts_with(if horizontal {
            "window-modify-width-right"
        } else {
            "window-modify-height-down"
        }));
        assert!(model.compile(&h.reducer.capture(), 2, 10.).is_err());
    }
}

#[test]
fn mixed_time_axis_loss_uses_validated_authority_not_relaxed_global_eligibility() {
    let (mut h, mut model) = initial(false);
    let first = model.compile(&h.reducer.capture(), 0, -80.).unwrap();
    model
        .begin(
            MotionId {
                attempt: 1,
                sequence: 1,
            },
            &first,
            0,
        )
        .unwrap();
    // Overshoot from a negative layout strut. Native positions advance before
    // clients commit: row 0's old rectangle now overlaps row 1.
    let c = census(&mut h, &[50., 130., 405.], &[100., 200., 300.], false);
    assert_eq!(
        c.groups.values().next().unwrap().edge_eligibility,
        EdgeEligibility::AmbiguousAxis
    );
    let assessment = model.observe(&c, 10).unwrap();
    assert!(!assessment.frame.coherent);
    assert_eq!(assessment.frame.target_extents, [60., 240., 300.]);
    assert_eq!(model.content_budget(), 600.);
    assert!(matches!(
        assessment.response,
        Some(PositionResponse::Progress {
            overshoot: true,
            ..
        })
    ));
    let compiled = model.compile(&c, 1, 20.).unwrap();
    assert!(edge::prepare(&c, &compiled.request).is_err());
    assert!(edge::prepare_frozen(&c, &compiled.request, &compiled.authority).is_ok());
    for change in [
        "tabbed",
        "output",
        "axis",
        "membership",
        "geometry",
        "epoch",
    ] {
        let mut bad = (*c).clone();
        let member = model.members()[0].clone();
        match change {
            "tabbed" => bad.windows.get_mut(&member).unwrap().presentation.tabbed = Some(true),
            "output" => bad.outputs.values_mut().next().unwrap().scale = Some(2.),
            "axis" => {
                bad.groups.values_mut().next().unwrap().secondary_axis = Some(Axis::Horizontal)
            }
            "membership" => bad.groups.values_mut().next().unwrap().members.reverse(),
            "geometry" => {
                bad.windows
                    .get_mut(&member)
                    .unwrap()
                    .geometry
                    .content_size
                    .as_mut()
                    .unwrap()
                    .height += 1.
            }
            "epoch" => bad.scope.epoch = Epoch(2),
            _ => unreachable!(),
        }
        assert!(
            edge::prepare_frozen(&bad, &compiled.request, &compiled.authority).is_err(),
            "{change}"
        );
    }
}

#[test]
fn empirical_scale_partial_commits_and_cumulative_late_commits_keep_original_budget() {
    let (mut h, mut model) = initial(false);
    let first = model.compile(&h.reducer.capture(), 0, 100.).unwrap();
    let key = MotionId {
        attempt: 1,
        sequence: 1,
    };
    model.begin(key, &first, 0).unwrap();
    let c = census(&mut h, &[50., 186., 405.], &[116., 200., 300.], false);
    let a = model.observe(&c, 10).unwrap();
    assert_eq!(a.frame.committed.iter().sum::<f64>(), 616.);
    assert_eq!(model.content_budget(), 600.);
    assert_eq!(model.empirical_scale(), Some(1050.));
    let second = model.compile(&c, 1, 100.).unwrap();
    assert_eq!(second.travel, 100.);
    let key = MotionId {
        attempt: 1,
        sequence: 2,
    };
    model.begin(key, &second, 11).unwrap();
    let c = census(&mut h, &[50., 186., 505.], &[116., 184., 300.], false);
    let a = model.observe(&c, 20).unwrap();
    assert_eq!(a.owned, Some(key));
    assert!(!a.frame.coherent);
    let c = census(&mut h, &[50., 186., 505.], &[116., 284., 200.], false);
    let a = model.observe(&c, 500).unwrap();
    assert_eq!(a.owned, Some(key));
    assert!(a.frame.coherent);
    assert_eq!(a.frame.target_extents, [116., 284., 200.]);
    assert_eq!(model.strips(), [20., 35.]);
    // A later actual user edit is no longer hidden inside an old commit corridor.
    let c = census(&mut h, &[50., 186., 505.], &[116., 270., 200.], false);
    assert!(model.observe(&c, 501).is_err());
    assert!(!model.is_valid());
}

#[test]
fn silent_deduplicated_stream_gets_one_query_then_a_bounded_timer_outcome() {
    let (h, mut model) = initial(false);
    let edge = model.compile(&h.reducer.capture(), 0, 100.).unwrap();
    let id = MotionId {
        attempt: 1,
        sequence: 1,
    };
    model.begin(id, &edge, 100).unwrap();
    assert_eq!(model.on_timer(131).unwrap(), TimerAction::WaitUntil(132));
    assert_eq!(
        model.on_timer(132).unwrap(),
        TimerAction::RefreshWindows(id)
    );
    assert_eq!(model.on_timer(163).unwrap(), TimerAction::WaitUntil(164));
    assert_eq!(model.on_timer(164).unwrap(), TimerAction::Blocked(id));
    assert_eq!(model.on_timer(200).unwrap(), TimerAction::Idle);
    assert!(model.on_timer(199).is_err());
}

#[test]
fn external_boundary_primary_envelope_and_topology_motion_revoke_frozen_model() {
    for motion in [
        "other boundary",
        "reverse",
        "outer start",
        "primary",
        "presentation",
    ] {
        let (mut h, mut model) = initial(false);
        let edge = model.compile(&h.reducer.capture(), 0, 100.).unwrap();
        model
            .begin(
                MotionId {
                    attempt: 1,
                    sequence: 1,
                },
                &edge,
                0,
            )
            .unwrap();
        let (p, e) = match motion {
            "other boundary" => ([50., 186., 420.], [100., 200., 300.]),
            "reverse" => ([50., 160., 405.], [100., 200., 300.]),
            "outer start" => ([40., 186., 405.], [100., 200., 300.]),
            _ => ([50., 186., 405.], [100., 200., 300.]),
        };
        let mut c = (*census(&mut h, &p, &e, false)).clone();
        if motion == "primary" {
            c.windows
                .values_mut()
                .next()
                .unwrap()
                .geometry
                .content_size
                .as_mut()
                .unwrap()
                .width -= 20.;
        }
        if motion == "presentation" {
            c.windows
                .values_mut()
                .next()
                .unwrap()
                .presentation
                .title_bar = Some(false);
        }
        assert!(model.observe(&c, 10).is_err(), "{motion}");
    }
}
