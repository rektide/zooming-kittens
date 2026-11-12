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

#[test]
fn later_same_boundary_drag_is_not_unbounded_owned_progress() {
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
    let c = census(&mut h, &[50., 186., 405.], &[116., 184., 300.], false);
    assert!(model.observe(&c, 10).unwrap().frame.coherent);
    let c = census(&mut h, &[50., 206., 405.], &[136., 164., 300.], false);
    assert!(
        model
            .observe(&c, 20)
            .unwrap_err()
            .contains("after completed")
    );
}

fn policy_input(
    model: &FrozenGeometry,
    owned: Option<MotionId>,
) -> zk_longcat::feedback::FeedbackInput {
    use zk_longcat::{
        Axis as PolicyAxis, Focus, GroupKey as PolicyGroup, Source, WindowKey as PolicyWindow,
        feedback::*,
    };
    let source = Source {
        id: "stock".into(),
        epoch: model.group().workspace.scope.epoch.0,
    };
    let ids: Vec<_> = model
        .members()
        .iter()
        .map(|id| match &id.native {
            NativeId::Text(id) => id.clone(),
            _ => unreachable!(),
        })
        .collect();
    FeedbackInput {
        source: source.clone(),
        version: model.frame().version.0,
        live_members: ids
            .iter()
            .map(|id| LiveMember {
                id: id.clone(),
                workspace: "ws".into(),
                context: "normal".into(),
            })
            .collect(),
        groups: vec![FeedbackGroup {
            key: PolicyGroup {
                source: source.clone(),
                workspace: "ws".into(),
                topology: "0".into(),
                axis: match model.axis() {
                    Axis::Vertical => PolicyAxis::Vertical,
                    Axis::Horizontal => PolicyAxis::Horizontal,
                },
            },
            context: "normal".into(),
            unavailable: None,
            members: ids
                .iter()
                .enumerate()
                .map(|(i, id)| FeedbackMember {
                    id: id.clone(),
                    start: model.frame().starts[i],
                    extent: model.frame().committed[i],
                    minimum_extent: 60,
                    maximum_extent: None,
                })
                .collect(),
            content_budget: model.content_budget(),
            target_extents: Some(model.frame().target_extents.clone()),
            coherent: model.frame().coherent,
            motion: owned.map_or(Motion::Unclassified, |id| {
                Motion::Owned(CommandKey {
                    attempt: id.attempt,
                    sequence: id.sequence,
                })
            }),
        }],
        focus: Focus::Window(PolicyWindow {
            source,
            workspace: "ws".into(),
            id: ids[0].clone(),
        }),
    }
}

#[test]
fn real_model_classification_drives_b_without_teaching_partial_or_late_commits() {
    use zk_longcat::{CommandOutcome, feedback::edges::*, feedback::*};
    let (mut h, mut model) = initial(false);
    let mut feedback = Feedback::new(FeedbackPolicy::default()).unwrap();
    feedback.observe(policy_input(&model, None), 0).unwrap();
    let FeedbackDecision::Drive(goal) = feedback.reconsider(500).unwrap() else {
        panic!("expected goal")
    };
    let preferences = feedback.preferences().to_vec();
    let generation = feedback.preference_generation();
    let mut now = 500;
    let mut last = None;
    loop {
        match select_edge(&goal, &model.frame().target_extents, feedback.current()) {
            EdgeSelection::Step(step) => {
                let c = h.reducer.capture();
                let compiled = model.compile(&c, step.boundary, step.travel).unwrap();
                let key = feedback
                    .begin_command(
                        goal.id,
                        EdgeIntent {
                            basis_version: c.version.0,
                            boundary: step.boundary,
                            travel: compiled.travel,
                        },
                        now,
                    )
                    .unwrap();
                let id = MotionId {
                    attempt: key.attempt,
                    sequence: key.sequence,
                };
                model.begin(id, &compiled, now).unwrap();
                feedback
                    .record_outcome(key, CommandOutcome::Handled)
                    .unwrap();
                let mut starts = model.frame().starts.clone();
                starts[step.boundary + 1] += compiled.travel.round();
                let c = census(&mut h, &starts, &[100., 200., 300.], false);
                now += 10;
                let assessment = model.observe(&c, now).unwrap();
                assert_eq!(assessment.owned, Some(id));
                // Capture installation precedes response evidence, as required by B.
                feedback
                    .observe(policy_input(&model, assessment.owned), now)
                    .unwrap();
                let Some(PositionResponse::Progress { version, .. }) = assessment.response else {
                    panic!("missing position response")
                };
                feedback
                    .record_response(
                        key,
                        EdgeResponse {
                            observed_version: version.0,
                            kind: ResponseKind::Progress,
                        },
                    )
                    .unwrap();
                feedback.reconsider(now).unwrap();
                last = Some(id);
            }
            EdgeSelection::PositionsMatch => break,
            other => panic!("unexpected {other:?}"),
        }
    }
    assert!(!model.frame().coherent);
    let target = model.frame().target_extents.clone();
    // Clients settle separately, after the final position response. Their mixed
    // committed sum is neither the next budget nor accepted preference guidance.
    let mut committed = vec![100., 200., 300.];
    for i in 0..3 {
        committed[i] = target[i];
        let c = census(&mut h, &model.frame().starts.clone(), &committed, false);
        now += 20;
        let assessment = model.observe(&c, now).unwrap();
        assert_eq!(assessment.owned, last);
        feedback
            .observe(policy_input(&model, assessment.owned), now)
            .unwrap();
        assert_eq!(feedback.preferences(), preferences);
        assert_eq!(feedback.preference_generation(), generation);
        assert_eq!(model.content_budget(), 600.);
    }
    assert!(model.frame().coherent);
    feedback.reconsider(now + 500).unwrap();
    assert_eq!(
        feedback.completed().unwrap().result.as_ref().unwrap().end,
        AttemptEnd::Settled
    );
    assert_eq!(feedback.preferences(), preferences);
}
