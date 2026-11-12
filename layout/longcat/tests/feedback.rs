use zk_longcat::feedback::*;
use zk_longcat::{Axis, CommandOutcome, Feedback, Focus, GroupKey, Source, WindowKey};

fn source() -> Source {
    Source {
        id: "stock".into(),
        epoch: 1,
    }
}

fn focus(id: &str) -> Focus {
    Focus::Window(WindowKey {
        source: source(),
        workspace: "ws".into(),
        id: id.into(),
    })
}

fn group(ids: &[&str], extents: &[f64], topology: &str) -> FeedbackGroup {
    let mut start = 0.0;
    FeedbackGroup {
        key: GroupKey {
            source: source(),
            workspace: "ws".into(),
            topology: topology.into(),
            axis: Axis::Vertical,
        },
        context: "output/presentation".into(),
        unavailable: None,
        members: ids
            .iter()
            .zip(extents)
            .map(|(&id, &extent)| {
                let member = FeedbackMember {
                    id: id.into(),
                    start,
                    extent,
                    minimum_extent: 60,
                    maximum_extent: None,
                };
                start += extent + 10.0;
                member
            })
            .collect(),
        content_budget: extents.iter().sum(),
        target_extents: Some(extents.to_vec()),
        coherent: true,
        motion: Motion::Unclassified,
    }
}

fn input() -> FeedbackInput {
    FeedbackInput {
        source: source(),
        version: 1,
        live_members: ["a", "b", "c"]
            .map(|id| LiveMember {
                id: id.into(),
                workspace: "ws".into(),
                context: "normal".into(),
            })
            .to_vec(),
        groups: vec![group(&["a", "b", "c"], &[400.0, 300.0, 200.0], "0")],
        focus: focus("a"),
    }
}

fn controller() -> Feedback {
    Feedback::new(FeedbackPolicy::default()).unwrap()
}

fn drive(decision: FeedbackDecision) -> FeedbackGoal {
    match decision {
        FeedbackDecision::Drive(goal) => *goal,
        other => panic!("expected goal, got {other:?}"),
    }
}

fn wait(decision: FeedbackDecision, phase: FeedbackPhase, reason: WaitReason) {
    assert_eq!(decision, FeedbackDecision::Wait { phase, reason });
}

fn extents(goal: &FeedbackGoal) -> Vec<u32> {
    goal.targets.iter().map(|t| t.extent).collect()
}

fn preference<'a>(feedback: &'a Feedback, id: &str) -> &'a Preference {
    feedback
        .preferences()
        .iter()
        .find(|p| p.window.id == id)
        .unwrap()
}

fn acquired() -> (Feedback, FeedbackInput, FeedbackGoal, u64) {
    let mut feedback = controller();
    let input = input();
    wait(
        feedback.observe(input.clone(), 0).unwrap(),
        FeedbackPhase::Acquiring,
        WaitReason::Quiet,
    );
    assert_eq!(feedback.next_reconsideration(), Some(500));
    let goal = drive(feedback.reconsider(500).unwrap());
    assert_eq!(extents(&goal), [630, 162, 108]);
    (feedback, input, goal, 500)
}

#[test]
fn native_rounding_below_policy_floor_still_allows_sub_tolerance_correction() {
    let mut feedback = controller();
    let mut input = input();
    geometry(&mut input, &[200., 200., 200.], Motion::Unclassified);
    for member in &mut input.groups[0].members {
        member.minimum_extent = 100;
    }
    feedback.observe(input.clone(), 0).unwrap();
    let goal = drive(feedback.reconsider(500).unwrap());
    assert_eq!(extents(&goal), [400, 100, 100]);
    let generation = feedback.preference_generation();
    let key = submit(&mut feedback, &input, &goal, 500);
    feedback
        .record_outcome(key, CommandOutcome::Handled)
        .unwrap();
    // Seen on the real positive-strut headless fixture: tolerance must not turn
    // its 98px donor into a declared success against a 100px effective floor.
    geometry(&mut input, &[400., 102., 98.], Motion::Owned(key));
    feedback.observe(input.clone(), 510).unwrap();
    feedback
        .record_response(
            key,
            EdgeResponse {
                observed_version: input.version,
                kind: ResponseKind::Progress,
            },
        )
        .unwrap();
    drive(feedback.reconsider(510).unwrap());
    feedback
        .begin_command(
            goal.id,
            EdgeIntent {
                basis_version: input.version,
                boundary: 1,
                travel: -2.,
            },
            510,
        )
        .unwrap();
    assert_eq!(feedback.preference_generation(), generation);
}

fn geometry(input: &mut FeedbackInput, values: &[f64], motion: Motion) {
    input.version += 1;
    let group = &mut input.groups[0];
    let mut start = group.members[0].start;
    for (member, &value) in group.members.iter_mut().zip(values) {
        member.start = start;
        member.extent = value;
        start += value + 10.0;
    }
    group.content_budget = values.iter().sum();
    group.target_extents = Some(values.to_vec());
    group.coherent = true;
    group.motion = motion;
}

fn submit(
    feedback: &mut Feedback,
    input: &FeedbackInput,
    goal: &FeedbackGoal,
    now: u64,
) -> CommandKey {
    feedback
        .begin_command(
            goal.id,
            EdgeIntent {
                basis_version: input.version,
                boundary: 0,
                travel: 16.0,
            },
            now,
        )
        .unwrap()
}

fn settle(
    feedback: &mut Feedback,
    input: &mut FeedbackInput,
    goal: &FeedbackGoal,
    now: &mut u64,
) -> CommandKey {
    let key = submit(feedback, input, goal, *now);
    let values: Vec<_> = extents(goal).into_iter().map(f64::from).collect();
    geometry(input, &values, Motion::Owned(key));
    *now += 10;
    wait(
        feedback.observe(input.clone(), *now).unwrap(),
        FeedbackPhase::Driving,
        WaitReason::PendingCommand,
    );
    feedback
        .record_response(
            key,
            EdgeResponse {
                observed_version: input.version,
                kind: ResponseKind::Progress,
            },
        )
        .unwrap();
    feedback
        .record_outcome(key, CommandOutcome::Handled)
        .unwrap();
    *now += 500;
    wait(
        feedback.reconsider(*now).unwrap(),
        FeedbackPhase::Ready,
        WaitReason::Completion,
    );
    let completed = feedback.take_completed().unwrap();
    assert_eq!(completed.result.unwrap().end, AttemptEnd::Settled);
    assert!(feedback.current().is_none());
    key
}

fn settled() -> (Feedback, FeedbackInput, u64) {
    let (mut feedback, mut input, goal, mut now) = acquired();
    settle(&mut feedback, &mut input, &goal, &mut now);
    wait(
        feedback.reconsider(now).unwrap(),
        FeedbackPhase::Ready,
        WaitReason::Stable,
    );
    (feedback, input, now)
}

#[test]
fn numerical_guidance_restore_and_focus_return_do_not_amplify_baseline() {
    let (mut f, mut input, mut now) = settled();
    let generation = f.preference_generation();
    geometry(&mut input, &[540.0, 216.0, 144.0], Motion::Unclassified);
    now += 1;
    wait(
        f.observe(input.clone(), now).unwrap(),
        FeedbackPhase::Yielding,
        WaitReason::Quiet,
    );
    assert_eq!(f.preference_generation(), generation);
    now += 500;
    wait(
        f.reconsider(now).unwrap(),
        FeedbackPhase::Ready,
        WaitReason::Stable,
    );
    assert_eq!(preference(&f, "a").active_share, Some(0.6));
    assert_eq!(preference(&f, "a").neutral_weight, 400.0);
    assert_eq!(preference(&f, "b").neutral_weight, 300.0);
    assert_eq!(preference(&f, "c").neutral_weight, 200.0);
    assert_eq!(
        f.guidance().next().unwrap().reason,
        GuidanceReason::ExternalGeometry
    );
    let generation = f.preference_generation();
    input.version += 1;
    input.focus = Focus::None;
    let neutral = drive(f.observe(input.clone(), now).unwrap());
    assert_eq!(extents(&neutral), [400, 300, 200]);
    settle(&mut f, &mut input, &neutral, &mut now);
    input.version += 1;
    input.focus = focus("a");
    let focused = drive(f.observe(input.clone(), now).unwrap());
    assert_eq!(extents(&focused), [540, 216, 144]);
    settle(&mut f, &mut input, &focused, &mut now);
    assert_eq!(f.preference_generation(), generation);
}

#[test]
fn sibling_boundary_teaches_only_visible_sibling_ratios() {
    let (mut f, mut input, mut now) = settled();
    geometry(&mut input, &[630.0, 200.0, 70.0], Motion::Unclassified);
    now += 1;
    f.observe(input.clone(), now).unwrap();
    now += 500;
    wait(
        f.reconsider(now).unwrap(),
        FeedbackPhase::Ready,
        WaitReason::Stable,
    );
    assert_eq!(preference(&f, "a").active_share, None);
    assert_eq!(preference(&f, "a").neutral_weight, 400.0);
    let b = preference(&f, "b").neutral_weight;
    let c = preference(&f, "c").neutral_weight;
    assert!((b + c - 500.0).abs() < 1e-9);
    assert!((b / c - 200.0 / 70.0).abs() < 1e-9);
}

#[test]
fn learned_share_survives_later_sibling_only_edit() {
    let (mut f, mut input, mut now) = settled();
    for values in [[540.0, 216.0, 144.0], [540.0, 280.0, 80.0]] {
        geometry(&mut input, &values, Motion::Unclassified);
        now += 1;
        f.observe(input.clone(), now).unwrap();
        now += 500;
        wait(
            f.reconsider(now).unwrap(),
            FeedbackPhase::Ready,
            WaitReason::Stable,
        );
    }
    assert_eq!(preference(&f, "a").active_share, Some(0.6));
    assert!(
        (preference(&f, "b").neutral_weight / preference(&f, "c").neutral_weight - 3.5).abs()
            < 1e-9
    );
}

#[test]
fn hundred_focus_switches_remain_usable_without_neutral_drift_or_history_veto() {
    let (mut f, mut input, mut now) = settled();
    let preferences = f.preferences().to_vec();
    let generation = f.preference_generation();
    for i in 0..100 {
        input.version += 1;
        input.focus = focus(if i % 2 == 0 { "b" } else { "a" });
        let goal = drive(f.observe(input.clone(), now).unwrap());
        assert!(f.completed().is_none());
        settle(&mut f, &mut input, &goal, &mut now);
        assert_eq!(f.preferences(), preferences);
        assert_eq!(f.preference_generation(), generation);
    }
    assert_eq!(f.counters().settled, 101);
    assert_eq!(f.counters().guidance, 0);
}

#[test]
fn neutral_guidance_preserves_total_and_remembered_shares() {
    let (mut f, mut input, mut now) = settled();
    geometry(&mut input, &[540.0, 216.0, 144.0], Motion::Unclassified);
    now += 1;
    f.observe(input.clone(), now).unwrap();
    now += 500;
    f.reconsider(now).unwrap();
    input.version += 1;
    input.focus = Focus::None;
    let goal = drive(f.observe(input.clone(), now).unwrap());
    settle(&mut f, &mut input, &goal, &mut now);
    geometry(&mut input, &[360.0, 300.0, 240.0], Motion::Unclassified);
    now += 1;
    f.observe(input.clone(), now).unwrap();
    now += 500;
    wait(
        f.reconsider(now).unwrap(),
        FeedbackPhase::Ready,
        WaitReason::Stable,
    );
    assert_eq!(
        f.preferences()
            .iter()
            .map(|p| p.neutral_weight)
            .collect::<Vec<_>>(),
        [360.0, 300.0, 240.0]
    );
    assert_eq!(preference(&f, "a").active_share, Some(0.6));
}

#[test]
fn continuous_drag_and_focus_changes_never_issue_a_new_write_during_yield() {
    let (mut f, mut input, mut now) = settled();
    let writes = f.counters().submissions;
    for i in 0..30 {
        geometry(
            &mut input,
            &[600.0 - f64::from(i), 180.0 + f64::from(i), 120.0],
            Motion::Unclassified,
        );
        input.focus = focus(if i % 2 == 0 { "a" } else { "b" });
        now += 100;
        wait(
            f.observe(input.clone(), now).unwrap(),
            FeedbackPhase::Yielding,
            WaitReason::Quiet,
        );
        assert!(
            f.begin_command(
                1,
                EdgeIntent {
                    basis_version: input.version,
                    boundary: 0,
                    travel: 1.0
                },
                now
            )
            .is_err()
        );
        assert_eq!(f.counters().submissions, writes);
        assert_eq!(f.counters().guidance, 0);
    }
    now += 500;
    let goal = drive(f.reconsider(now).unwrap());
    assert_eq!(goal.inputs.active.as_deref(), Some("b"));
    assert_eq!(preference(&f, "a").active_share, Some(571.0 / 900.0));
    assert_eq!(preference(&f, "b").active_share, None);
    assert_eq!(
        f.guidance().next().unwrap().emphasized.as_deref(),
        Some("a")
    );
}

#[test]
fn manual_small_extent_lowers_only_contextual_floor_and_is_not_bounced() {
    let (mut f, mut input, mut now) = settled();
    geometry(&mut input, &[810.0, 60.0, 30.0], Motion::Unclassified);
    now += 1;
    f.observe(input.clone(), now).unwrap();
    now += 500;
    wait(
        f.reconsider(now).unwrap(),
        FeedbackPhase::Ready,
        WaitReason::Stable,
    );
    assert_eq!(preference(&f, "c").manual_floor, Some(30));
    input.version += 1;
    input.focus = focus("b");
    let goal = drive(f.observe(input.clone(), now).unwrap());
    assert_eq!(goal.inputs.members[2].minimum_extent, 30);
    settle(&mut f, &mut input, &goal, &mut now);
    input.version += 1;
    input.live_members[2].context = "new-presentation".into();
    input.groups[0].context = "new-presentation".into();
    input.groups[0].motion = Motion::Unclassified;
    f.observe(input.clone(), now).unwrap();
    assert_eq!(preference(&f, "c").manual_floor, None);
}

#[test]
fn preview_cannot_create_preferences_command_receipts_or_timer_obligations() {
    let mut f = controller();
    let input = input();
    f.observe(input.clone(), 0).unwrap();
    let before = serde_json::to_value(&f).unwrap();
    let goal = drive(f.preview(input.clone(), 500).unwrap());
    assert_eq!(extents(&goal), [630, 162, 108]);
    assert_eq!(serde_json::to_value(&f).unwrap(), before);
    assert!(
        f.begin_command(
            goal.id,
            EdgeIntent {
                basis_version: input.version,
                boundary: 0,
                travel: 16.0
            },
            500
        )
        .is_err()
    );
    assert!(f.current().is_none());
}

#[test]
fn quiet_acquisition_uses_timer_and_ignores_focus_only_events() {
    let mut f = controller();
    let mut input = input();
    f.observe(input.clone(), 0).unwrap();
    input.version += 1;
    input.focus = focus("b");
    wait(
        f.observe(input, 400).unwrap(),
        FeedbackPhase::Acquiring,
        WaitReason::Quiet,
    );
    assert_eq!(f.next_reconsideration(), Some(500));
    assert_eq!(extents(&drive(f.reconsider(500).unwrap())), [180, 630, 90]);
}

#[test]
fn owned_target_response_then_partial_and_late_clients_never_teach() {
    let (mut f, mut input, goal, mut now) = acquired();
    let generation = f.preference_generation();
    let key = submit(&mut f, &input, &goal, now);
    input.version += 1;
    input.groups[0].target_extents = Some(vec![630.0, 162.0, 108.0]);
    input.groups[0].members[1].start = 640.0;
    input.groups[0].members[2].start = 812.0;
    input.groups[0].coherent = false;
    input.groups[0].motion = Motion::Owned(key);
    now += 10;
    f.observe(input.clone(), now).unwrap();
    f.record_outcome(key, CommandOutcome::Handled).unwrap();
    f.record_response(
        key,
        EdgeResponse {
            observed_version: input.version,
            kind: ResponseKind::Progress,
        },
    )
    .unwrap();
    wait(
        f.reconsider(now).unwrap(),
        FeedbackPhase::Driving,
        WaitReason::ClientCommit,
    );
    input.version += 1;
    input.groups[0].members[0].extent = 630.0;
    now += 100;
    f.observe(input.clone(), now).unwrap();
    assert_eq!(f.current().unwrap().goal.inputs.content_budget, 900.0);
    assert_eq!(f.preference_generation(), generation);
    geometry(&mut input, &[630.0, 162.0, 108.0], Motion::Owned(key));
    now += 100;
    f.observe(input.clone(), now).unwrap();
    now += 500;
    f.reconsider(now).unwrap();
    f.take_completed().unwrap();
    // Matching late noise still has explicit frozen prediction authority.
    input.version += 1;
    input.groups[0].members[0].extent += 0.5;
    now += 1;
    f.observe(input.clone(), now).unwrap();
    now += 500;
    f.reconsider(now).unwrap();
    assert_eq!(f.preference_generation(), generation);
    assert_eq!(f.counters().guidance, 0);
}

#[test]
fn reply_does_not_allow_next_command_without_position_response() {
    let (mut f, mut input, goal, mut now) = acquired();
    let key = submit(&mut f, &input, &goal, now);
    f.record_outcome(key, CommandOutcome::Handled).unwrap();
    wait(
        f.reconsider(now).unwrap(),
        FeedbackPhase::Driving,
        WaitReason::PositionResponse,
    );
    assert!(
        f.begin_command(
            goal.id,
            EdgeIntent {
                basis_version: input.version,
                boundary: 1,
                travel: 16.0
            },
            now
        )
        .is_err()
    );
    geometry(&mut input, &[416.0, 284.0, 200.0], Motion::Owned(key));
    now += 10;
    f.observe(input.clone(), now).unwrap();
    f.record_response(
        key,
        EdgeResponse {
            observed_version: input.version,
            kind: ResponseKind::Progress,
        },
    )
    .unwrap();
    drive(f.reconsider(now).unwrap());
    let next = f
        .begin_command(
            goal.id,
            EdgeIntent {
                basis_version: input.version,
                boundary: 1,
                travel: 16.0,
            },
            now,
        )
        .unwrap();
    assert_eq!(next.sequence, 2);
}

#[test]
fn ambiguous_overlap_drains_before_unattributed_guidance() {
    let (mut f, mut input, goal, mut now) = acquired();
    let key = submit(&mut f, &input, &goal, now);
    geometry(
        &mut input,
        &[540.0, 216.0, 144.0],
        Motion::Unattributed(key),
    );
    now += 10;
    wait(
        f.observe(input.clone(), now).unwrap(),
        FeedbackPhase::Yielding,
        WaitReason::PendingCommand,
    );
    assert!(f.current().unwrap().stopping.is_some());
    now += 500;
    f.reconsider(now).unwrap();
    assert_eq!(f.counters().guidance, 0);
    f.record_outcome(key, CommandOutcome::Handled).unwrap();
    f.reconsider(now).unwrap();
    assert_eq!(
        f.take_completed().unwrap().commands[0].outcome,
        Some(CommandOutcome::Handled)
    );
    wait(
        f.reconsider(now).unwrap(),
        FeedbackPhase::Ready,
        WaitReason::Stable,
    );
    assert_eq!(preference(&f, "a").active_share, Some(0.6));
    assert_eq!(
        f.guidance().next().unwrap().reason,
        GuidanceReason::UnattributedOverlap
    );
}

#[test]
fn silent_handled_attempt_is_limited_once_without_teaching_and_focus_retries() {
    let (mut f, mut input, goal, now) = acquired();
    let generation = f.preference_generation();
    let key = submit(&mut f, &input, &goal, now);
    f.record_outcome(key, CommandOutcome::Handled).unwrap();
    assert_eq!(f.next_reconsideration(), Some(goal.deadline_ms));
    wait(
        f.reconsider(goal.deadline_ms).unwrap(),
        FeedbackPhase::Limited,
        WaitReason::Completion,
    );
    let done = f.take_completed().unwrap();
    assert_eq!(
        done.result.unwrap().end,
        AttemptEnd::Limited(LimitReason::Deadline)
    );
    for time in 0..50 {
        wait(
            f.reconsider(goal.deadline_ms + time).unwrap(),
            FeedbackPhase::Limited,
            WaitReason::Limited,
        );
    }
    assert_eq!(f.counters().attempts, 1);
    assert_eq!(f.preference_generation(), generation);
    input.version += 1;
    input.focus = focus("b");
    let new = drive(f.observe(input, goal.deadline_ms + 100).unwrap());
    assert_eq!(new.id, 2);
    assert_eq!(new.inputs.active.as_deref(), Some("b"));
}

#[test]
fn native_blocked_result_is_not_user_guidance_or_universal_bound() {
    let (mut f, mut input, goal, mut now) = acquired();
    let key = submit(&mut f, &input, &goal, now);
    geometry(&mut input, &[405.0, 295.0, 200.0], Motion::Owned(key));
    now += 10;
    f.observe(input.clone(), now).unwrap();
    f.record_outcome(key, CommandOutcome::Handled).unwrap();
    f.record_response(
        key,
        EdgeResponse {
            observed_version: input.version,
            kind: ResponseKind::Blocked {
                reason: "native clamp".into(),
            },
        },
    )
    .unwrap();
    f.limit_current(
        goal.id,
        LimitReason::Executor {
            reason: "no donor route".into(),
        },
    )
    .unwrap();
    f.reconsider(now).unwrap();
    let result = f.take_completed().unwrap();
    assert_eq!(result.commands.len(), 1);
    assert!(matches!(result.result.unwrap().end, AttemptEnd::Limited(_)));
    now += 500;
    wait(
        f.reconsider(now).unwrap(),
        FeedbackPhase::Limited,
        WaitReason::Limited,
    );
    assert_eq!(preference(&f, "a").active_share, None);
    assert_eq!(preference(&f, "a").neutral_weight, 400.0);
    assert_eq!(f.counters().guidance, 0);
    assert!(f.preferences().iter().all(|p| p.manual_floor.is_none()));
}

#[test]
fn no_progress_commands_are_bounded_even_when_executor_keeps_offering_edges() {
    let (mut f, input, goal, now) = acquired();
    assert_eq!(goal.submission_limit, 18);
    for i in 0..goal.submission_limit {
        let key = f
            .begin_command(
                goal.id,
                EdgeIntent {
                    basis_version: input.version,
                    boundary: i % 2,
                    travel: 1.0,
                },
                now,
            )
            .unwrap();
        f.record_outcome(key, CommandOutcome::Handled).unwrap();
        f.record_response(
            key,
            EdgeResponse {
                observed_version: input.version,
                kind: ResponseKind::Blocked {
                    reason: "no movement".into(),
                },
            },
        )
        .unwrap();
        let decision = f.reconsider(now).unwrap();
        if i + 1 < goal.submission_limit {
            drive(decision);
        } else {
            wait(decision, FeedbackPhase::Limited, WaitReason::Completion);
        }
    }
    assert_eq!(
        f.take_completed().unwrap().result.unwrap().end,
        AttemptEnd::Limited(LimitReason::SubmissionBudget)
    );
    assert!(f.current().is_none());
}

#[test]
fn loss_retains_pending_receipt_blocks_old_epoch_and_resumes_only_after_drain() {
    let (mut f, mut input, goal, now) = acquired();
    let key = submit(&mut f, &input, &goal, now);
    f.lose("disconnect", now + 1).unwrap();
    assert!(f.current().unwrap().stopping.is_some());
    assert_eq!(
        f.observe(input.clone(), now + 2),
        Err(FeedbackError::StaleObservation)
    );
    input.source.epoch = 2;
    input.groups[0].key.source = input.source.clone();
    if let Focus::Window(window) = &mut input.focus {
        window.source = input.source.clone();
    }
    input.version = 1;
    f.observe(input.clone(), now + 2).unwrap();
    assert!(f.preferences().is_empty());
    f.reconsider(now + 600).unwrap();
    assert!(f.current().unwrap().pending().is_some());
    f.record_outcome(
        key,
        CommandOutcome::PossiblySent {
            message: "lost after write".into(),
        },
    )
    .unwrap();
    wait(
        f.reconsider(now + 600).unwrap(),
        FeedbackPhase::Acquiring,
        WaitReason::Completion,
    );
    let done = f.take_completed().unwrap();
    assert!(matches!(
        done.result.unwrap().end,
        AttemptEnd::Retired(RetirementReason::SourceLost { .. })
    ));
    let new = drive(f.reconsider(now + 600).unwrap());
    assert_ne!(new.id, goal.id);
    assert_eq!(new.inputs.group.source.epoch, 2);
    assert_eq!(
        f.record_outcome(key, CommandOutcome::Handled),
        Err(FeedbackError::UnknownCommand)
    );
    assert_eq!(f.counters().continuity_losses, 1);
}

#[test]
fn topology_open_close_reorder_renumber_and_tab_resume_preserve_preferences() {
    let (mut f, mut input, mut now) = settled();
    let original = f.preferences().to_vec();
    input.version += 1;
    input.live_members.push(LiveMember {
        id: "d".into(),
        workspace: "ws".into(),
        context: "normal".into(),
    });
    input.groups[0] = group(
        &["c", "a", "d", "b"],
        &[108.0, 630.0, 100.0, 162.0],
        "renumbered",
    );
    now += 1;
    f.observe(input.clone(), now).unwrap();
    now += 500;
    let goal = drive(f.reconsider(now).unwrap());
    assert_eq!(
        goal.inputs
            .members
            .iter()
            .map(|m| m.neutral_weight)
            .collect::<Vec<_>>(),
        [200.0, 400.0, 100.0, 300.0]
    );
    settle(&mut f, &mut input, &goal, &mut now);
    // Tabbed is parked, not removed from the full live census.
    input.version += 1;
    input.groups[0].unavailable = Some("tabbed".into());
    input.groups[0].motion = Motion::Unclassified;
    f.observe(input.clone(), now).unwrap();
    assert_eq!(preference(&f, "a").neutral_weight, 400.0);
    input.version += 1;
    input.groups[0].unavailable = None;
    now += 1;
    f.observe(input.clone(), now).unwrap();
    now += 500;
    f.reconsider(now).unwrap();
    assert_eq!(preference(&f, "b").neutral_weight, 300.0);
    // Close d; surviving members retain their neutral data, not emphasized sizes.
    input.version += 1;
    input.live_members.retain(|m| m.id != "d");
    input.groups[0] = group(&["a", "b", "c"], &[630.0, 162.0, 108.0], "0");
    now += 1;
    f.observe(input.clone(), now).unwrap();
    now += 500;
    f.reconsider(now).unwrap();
    assert_eq!(f.preferences(), original);
}

#[test]
fn cross_group_move_uses_neutral_weight_and_workspace_move_resets_only_share() {
    let (mut f, mut input, mut now) = settled();
    geometry(&mut input, &[540.0, 216.0, 144.0], Motion::Unclassified);
    now += 1;
    f.observe(input.clone(), now).unwrap();
    now += 500;
    f.reconsider(now).unwrap();
    input.version += 1;
    input.focus = Focus::None;
    input.live_members.push(LiveMember {
        id: "d".into(),
        workspace: "ws".into(),
        context: "normal".into(),
    });
    input.groups = vec![
        group(&["b", "c"], &[216.0, 144.0], "0"),
        group(&["d", "a"], &[300.0, 540.0], "1"),
    ];
    now += 1;
    f.observe(input.clone(), now).unwrap();
    now += 500;
    let goal = drive(f.reconsider(now).unwrap());
    assert_eq!(
        goal.inputs
            .members
            .iter()
            .map(|m| m.neutral_weight)
            .collect::<Vec<_>>(),
        [300.0, 400.0]
    );
    assert_eq!(preference(&f, "a").active_share, Some(0.6));
    // Retire this unsent attempt by moving the group into another workspace.
    input.version += 1;
    input.groups[1].key.workspace = "other".into();
    for live in &mut input.live_members {
        if live.id == "a" || live.id == "d" {
            live.workspace = "other".into();
        }
    }
    now += 1;
    f.observe(input.clone(), now).unwrap();
    f.take_completed().unwrap();
    assert_eq!(preference(&f, "a").neutral_weight, 400.0);
    assert_eq!(preference(&f, "a").active_share, None);
    assert_eq!(preference(&f, "a").window.workspace, "other");
}

#[test]
fn topology_change_cancels_tail_but_never_discards_in_flight_command() {
    let (mut f, mut input, goal, now) = acquired();
    let key = submit(&mut f, &input, &goal, now);
    input.version += 1;
    input.groups[0].members.swap(0, 1);
    input.groups[0].motion = Motion::Unclassified;
    f.observe(input.clone(), now + 1).unwrap();
    assert_eq!(
        f.current().unwrap().stopping,
        Some(AttemptEnd::Retired(RetirementReason::TopologyChanged))
    );
    assert!(
        f.begin_command(
            goal.id,
            EdgeIntent {
                basis_version: input.version,
                boundary: 0,
                travel: 1.0
            },
            now + 1
        )
        .is_err()
    );
    f.record_outcome(key, CommandOutcome::Handled).unwrap();
    f.reconsider(now + 2).unwrap();
    assert_eq!(
        f.take_completed().unwrap().commands[0].outcome,
        Some(CommandOutcome::Handled)
    );
}

#[test]
fn explicit_neutral_restore_uses_current_preferences_and_yields_to_user() {
    let (mut f, mut input, mut now) = settled();
    geometry(&mut input, &[540.0, 216.0, 144.0], Motion::Unclassified);
    now += 1;
    f.observe(input.clone(), now).unwrap();
    wait(
        f.request_restore(now).unwrap(),
        FeedbackPhase::Yielding,
        WaitReason::Quiet,
    );
    now += 500;
    let goal = drive(f.reconsider(now).unwrap());
    assert_eq!(goal.inputs.active, None);
    assert_eq!(extents(&goal), [400, 300, 200]);
    settle(&mut f, &mut input, &goal, &mut now);
    assert!(f.restoration_requested());
    input.version += 1;
    input.focus = focus("b");
    wait(
        f.observe(input, now).unwrap(),
        FeedbackPhase::Ready,
        WaitReason::Stable,
    );
}

#[test]
fn new_epoch_drops_hidden_preferences_instead_of_claiming_recovery_from_pixels() {
    let (mut f, mut input, now) = settled();
    input.source.epoch = 2;
    input.groups[0].key.source = input.source.clone();
    input.groups[0].motion = Motion::Unclassified;
    if let Focus::Window(window) = &mut input.focus {
        window.source = input.source.clone();
    }
    input.version = 1;
    f.observe(input, now).unwrap();
    f.reconsider(now + 500).unwrap();
    assert_eq!(preference(&f, "a").neutral_weight, 630.0);
    assert_eq!(preference(&f, "a").active_share, None);
    assert_eq!(f.counters().continuity_losses, 1);
}

#[test]
fn completion_is_explicit_bounded_backpressure_not_an_audit_vector() {
    let (mut f, mut input, goal, now) = acquired();
    f.limit_current(
        goal.id,
        LimitReason::Executor {
            reason: "test".into(),
        },
    )
    .unwrap();
    f.reconsider(now).unwrap();
    let before = f.completed().cloned();
    for i in 0..100 {
        input.version += 1;
        input.focus = focus(if i % 2 == 0 { "a" } else { "b" });
        wait(
            f.observe(input.clone(), now).unwrap(),
            FeedbackPhase::Limited,
            WaitReason::Completion,
        );
        assert_eq!(f.completed(), before.as_ref());
        assert!(f.current().is_none());
    }
    f.take_completed().unwrap();
    assert_eq!(drive(f.reconsider(now).unwrap()).id, 2);
}

#[test]
fn invalid_input_clock_evidence_and_outer_edges_are_rejected_without_mutation() {
    let (mut f, input, goal, now) = acquired();
    let before = serde_json::to_value(&f).unwrap();
    let mut invalid = input.clone();
    invalid.groups[0].members[0].extent = f64::NAN;
    assert!(matches!(
        f.observe(invalid, now),
        Err(FeedbackError::InvalidInput { .. })
    ));
    assert_eq!(f.reconsider(now - 1), Err(FeedbackError::ClockRegressed));
    for boundary in [2, 3, usize::MAX] {
        assert_eq!(
            f.begin_command(
                goal.id,
                EdgeIntent {
                    basis_version: input.version,
                    boundary,
                    travel: 1.0
                },
                now
            ),
            Err(FeedbackError::InvalidEdge)
        );
    }
    assert_eq!(serde_json::to_value(&f).unwrap(), before);
    let key = submit(&mut f, &input, &goal, now);
    assert!(
        f.record_response(
            key,
            EdgeResponse {
                observed_version: input.version,
                kind: ResponseKind::Progress
            }
        )
        .is_err()
    );
    f.record_outcome(key, CommandOutcome::Handled).unwrap();
    assert_eq!(
        f.record_outcome(key, CommandOutcome::Unsent),
        Err(FeedbackError::ContradictoryEvidence)
    );
}

#[test]
fn horizontal_secondary_axis_uses_the_same_scalar_taffy_allocation() {
    let mut input = input();
    input.groups[0].key.axis = Axis::Horizontal;
    let mut f = controller();
    f.observe(input, 0).unwrap();
    let goal = drive(f.reconsider(500).unwrap());
    assert_eq!(goal.inputs.group.axis, Axis::Horizontal);
    assert_eq!(extents(&goal), [630, 162, 108]);
}

#[test]
fn returning_to_an_old_focus_is_a_new_generation_not_an_old_limitation() {
    let (mut f, mut input, goal, mut now) = acquired();
    f.limit_current(
        goal.id,
        LimitReason::Executor {
            reason: "clamp".into(),
        },
    )
    .unwrap();
    f.reconsider(now).unwrap();
    f.take_completed().unwrap();
    wait(
        f.reconsider(now).unwrap(),
        FeedbackPhase::Limited,
        WaitReason::Limited,
    );
    input.version += 1;
    input.focus = focus("b");
    let other = drive(f.observe(input.clone(), now).unwrap());
    settle(&mut f, &mut input, &other, &mut now);
    input.version += 1;
    input.focus = focus("a");
    let returned = drive(f.observe(input, now).unwrap());
    assert_eq!(returned.inputs.active.as_deref(), Some("a"));
    assert_eq!(returned.id, 3);
}

#[test]
fn late_owned_commit_after_limitation_refreshes_expected_geometry_not_preferences() {
    let (mut f, mut input, goal, mut now) = acquired();
    let key = submit(&mut f, &input, &goal, now);
    geometry(&mut input, &[500.0, 240.0, 160.0], Motion::Owned(key));
    now += 10;
    f.observe(input.clone(), now).unwrap();
    f.record_outcome(key, CommandOutcome::Handled).unwrap();
    f.limit_current(
        goal.id,
        LimitReason::Executor {
            reason: "slow client deadline".into(),
        },
    )
    .unwrap();
    f.reconsider(now).unwrap();
    f.take_completed().unwrap();
    let generation = f.preference_generation();
    geometry(&mut input, &[630.0, 162.0, 108.0], Motion::Owned(key));
    now += 1;
    f.observe(input.clone(), now).unwrap();
    now += 500;
    f.reconsider(now).unwrap();
    assert_eq!(f.preference_generation(), generation);
    // Editing only a sibling boundary must NOT infer an active-share edit by
    // comparing against the stale partial 500px reference.
    geometry(&mut input, &[630.0, 180.0, 90.0], Motion::Unclassified);
    now += 1;
    f.observe(input.clone(), now).unwrap();
    now += 500;
    wait(
        f.reconsider(now).unwrap(),
        FeedbackPhase::Ready,
        WaitReason::Stable,
    );
    assert_eq!(preference(&f, "a").active_share, None);
    assert_eq!(preference(&f, "a").neutral_weight, 400.0);
    assert!(
        (preference(&f, "b").neutral_weight / preference(&f, "c").neutral_weight - 2.0).abs()
            < 1e-9
    );
}

#[test]
fn lost_geometry_model_stops_a_goal_instead_of_driving_from_committed_fallback() {
    let (mut f, mut input, goal, now) = acquired();
    input.version += 1;
    input.groups[0].target_extents = None;
    f.observe(input.clone(), now).unwrap();
    let done = f.take_completed().unwrap();
    assert_eq!(
        done.result.unwrap().end,
        AttemptEnd::Retired(RetirementReason::Unavailable)
    );
    assert!(
        f.begin_command(
            goal.id,
            EdgeIntent {
                basis_version: input.version,
                boundary: 0,
                travel: 1.0
            },
            now
        )
        .is_err()
    );
    wait(
        f.reconsider(now).unwrap(),
        FeedbackPhase::Acquiring,
        WaitReason::CoherentGeometry,
    );
    input.version += 1;
    input.groups[0].target_extents = Some(vec![400.0, 300.0, 200.0]);
    assert_eq!(drive(f.observe(input, now).unwrap()).id, 2);
}

#[test]
fn unsent_focus_attempt_does_not_claim_an_emphasis_when_later_neutral_guidance_arrives() {
    let (mut f, mut input, goal, mut now) = acquired();
    let key = submit(&mut f, &input, &goal, now);
    f.record_outcome(key, CommandOutcome::Unsent).unwrap();
    f.reconsider(now).unwrap();
    f.take_completed().unwrap();
    geometry(&mut input, &[360.0, 300.0, 240.0], Motion::Unclassified);
    now += 1;
    f.observe(input.clone(), now).unwrap();
    now += 500;
    let new = drive(f.reconsider(now).unwrap());
    assert_eq!(f.guidance().next().unwrap().emphasized, None);
    assert_eq!(preference(&f, "a").neutral_weight, 360.0);
    assert_eq!(preference(&f, "a").active_share, None);
    assert_eq!(extents(&new), [630, 150, 120]);
}

#[test]
fn outer_envelope_resize_preserving_active_share_does_not_teach_hidden_neutral_weight() {
    let (mut f, mut input, mut now) = settled();
    geometry(&mut input, &[315.0, 81.0, 54.0], Motion::Unclassified);
    now += 1;
    f.observe(input.clone(), now).unwrap();
    now += 500;
    wait(
        f.reconsider(now).unwrap(),
        FeedbackPhase::Ready,
        WaitReason::Stable,
    );
    assert_eq!(preference(&f, "a").active_share, None);
    assert_eq!(preference(&f, "a").neutral_weight, 400.0);
    assert_eq!(preference(&f, "b").neutral_weight, 300.0);
    assert_eq!(preference(&f, "c").neutral_weight, 200.0);
    assert_eq!(preference(&f, "c").manual_floor, Some(54));
}

#[test]
fn unknown_focus_never_means_blur_and_infeasible_inputs_do_not_create_native_intent() {
    let mut f = controller();
    let mut input = input();
    input.focus = Focus::Unknown;
    f.observe(input.clone(), 0).unwrap();
    wait(
        f.reconsider(500).unwrap(),
        FeedbackPhase::Unavailable,
        WaitReason::FocusUnknown,
    );
    assert!(f.current().is_none());
    input.version += 1;
    input.focus = focus("a");
    for member in &mut input.groups[0].members {
        member.minimum_extent = 400;
    }
    f.observe(input, 501).unwrap();
    assert!(matches!(
        f.reconsider(1001).unwrap(),
        FeedbackDecision::Infeasible {
            error: zk_layout::Error::Infeasible(_),
            ..
        }
    ));
    assert_eq!(f.counters().submissions, 0);
    assert!(f.current().is_none());
}

#[test]
fn empty_group_projection_preserves_live_preferences_then_automatically_reacquires() {
    let (mut f, mut input, mut now) = settled();
    let preferences = f.preferences().to_vec();
    let mut group = input.groups.pop().unwrap();
    group.motion = Motion::Unclassified;
    input.version += 1;
    f.observe(input.clone(), now).unwrap();
    assert_eq!(f.preferences(), preferences);
    assert_eq!(f.phase(), FeedbackPhase::Unavailable);
    input.version += 1;
    input.groups.push(group);
    now += 1;
    f.observe(input.clone(), now).unwrap();
    now += 500;
    wait(
        f.reconsider(now).unwrap(),
        FeedbackPhase::Ready,
        WaitReason::Stable,
    );
    assert_eq!(f.preferences(), preferences);
}

#[test]
fn two_group_focus_transition_restores_old_group_before_emphasizing_new_group() {
    let (mut f, mut input, mut now) = settled();
    for id in ["d", "e"] {
        input.live_members.push(LiveMember {
            id: id.into(),
            workspace: "ws".into(),
            context: "normal".into(),
        });
    }
    input
        .groups
        .insert(0, group(&["d", "e"], &[450.0, 450.0], "new"));
    input.version += 1;
    input.groups[1].motion = Motion::Unclassified;
    now += 1;
    f.observe(input.clone(), now).unwrap();
    now += 500;
    f.reconsider(now).unwrap();
    input.version += 1;
    input.focus = focus("d");
    let restore = drive(f.observe(input.clone(), now).unwrap());
    assert_eq!(restore.inputs.group.topology, "0");
    assert_eq!(restore.inputs.active, None);
    assert_eq!(extents(&restore), [400, 300, 200]);
}

#[test]
fn response_blockage_retains_adjacent_geometry_without_inventing_a_client_minimum() {
    let (mut f, input, goal, now) = acquired();
    let key = submit(&mut f, &input, &goal, now);
    f.record_outcome(key, CommandOutcome::Handled).unwrap();
    f.record_response(
        key,
        EdgeResponse {
            observed_version: input.version,
            kind: ResponseKind::Blocked {
                reason: "observed clamp".into(),
            },
        },
    )
    .unwrap();
    f.limit_current(
        goal.id,
        LimitReason::Executor {
            reason: "no route".into(),
        },
    )
    .unwrap();
    f.reconsider(now).unwrap();
    let done = f.take_completed().unwrap();
    let result = done.result.unwrap();
    assert_eq!(
        result.blocked_edges,
        [BlockedEdge {
            command: key,
            boundary: 0,
            positive_travel: true,
            adjacent_estimates: Some([400.0, 300.0]),
            observed_version: input.version,
            reason: "observed clamp".into()
        }]
    );
    assert!(f.preferences().iter().all(|p| p.manual_floor.is_none()));
}

#[test]
fn context_retirement_retains_receipt_but_not_old_prediction_authority() {
    let (mut f, mut input, goal, now) = acquired();
    let key = submit(&mut f, &input, &goal, now);
    input.version += 1;
    input.groups[0].context = "new-output".into();
    f.observe(input.clone(), now + 1).unwrap();
    assert_eq!(
        f.current().unwrap().stopping,
        Some(AttemptEnd::Retired(RetirementReason::ContextChanged))
    );
    f.record_outcome(key, CommandOutcome::Handled).unwrap();
    f.reconsider(now + 2).unwrap();
    assert_eq!(f.take_completed().unwrap().result.unwrap().achieved, None);
    input.version += 1;
    input.groups[0].motion = Motion::Owned(key);
    assert_eq!(
        f.observe(input, now + 3),
        Err(FeedbackError::UnknownCommand)
    );
}
