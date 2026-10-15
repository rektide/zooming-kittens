use zk_longcat::{
    Axis, CommandOutcome, Convergence, Decision, Episode, EvidenceError, Focus, Gaps, GroupKey,
    InputError, Noop, Observation, ObservedMember, Phase, Plan, PlanKind, PlanRejection, Policy,
    Source, Suspension, WindowKey,
};

fn source() -> Source {
    Source {
        id: "niri-test".into(),
        epoch: 1,
    }
}

fn focus(id: &str) -> Focus {
    Focus::Window(WindowKey {
        source: source(),
        workspace: "one".into(),
        id: id.into(),
    })
}

fn observation() -> Observation {
    Observation {
        group: GroupKey {
            source: source(),
            workspace: "one".into(),
            topology: "column-0".into(),
            axis: Axis::Vertical,
        },
        version: 1,
        available_extent: 900.0,
        gaps: Gaps::AlreadyExcluded,
        members: [400.0, 300.0, 200.0]
            .into_iter()
            .enumerate()
            .map(|(index, extent)| ObservedMember {
                id: ["a", "b", "c"][index].into(),
                extent,
                minimum: 0,
                maximum: None,
            })
            .collect(),
        focus: focus("a"),
    }
}

fn episode() -> Episode {
    Episode::new(
        observation(),
        Policy {
            geometry_tolerance: 0,
            convergence_timeout_ms: 100,
            ..Policy::default()
        },
    )
    .unwrap()
}

fn plan(decision: Decision) -> Plan {
    match decision {
        Decision::Plan(plan) => *plan,
        other => panic!("expected plan, got {other:?}"),
    }
}

fn extents(plan: &Plan) -> Vec<u32> {
    plan.targets.iter().map(|target| target.extent).collect()
}

fn applied(plan: &Plan, version: u64) -> Observation {
    let mut observation = plan.basis.clone();
    observation.version = version;
    for (member, target) in observation.members.iter_mut().zip(&plan.targets) {
        member.extent = f64::from(target.extent);
    }
    observation
}

fn settle_first(episode: &mut Episode) -> (Plan, Observation) {
    let first = plan(episode.decide(observation(), 10));
    episode
        .record_execution(first.id, vec![CommandOutcome::Handled; 3])
        .unwrap();
    let current = applied(&first, 2);
    assert_eq!(
        episode.decide(current.clone(), 20),
        Decision::Noop(Noop::Stable)
    );
    assert_eq!(episode.phase(), Phase::Settled);
    (first, current)
}

#[test]
fn dry_run_previews_never_claim_effects_or_conflict_with_unexecuted_intent() {
    let episode = episode();
    for now in [10, 50, 150, 1_000] {
        let first = plan(episode.preview(observation(), now));
        assert_eq!(extents(&first), [630, 162, 108]);
        let mut next = observation();
        next.focus = focus("b");
        let second = plan(episode.preview(next, now + 1));
        assert_eq!(extents(&second), [180, 630, 90]);
    }
    assert_eq!(episode.phase(), Phase::Ready);
    assert!(episode.requested().is_none());
    assert!(episode.effects().is_empty());
    assert_eq!(episode.next_reconsideration(), None);
    assert_eq!(episode.observed(), &observation());
}

#[test]
fn requested_baseline_and_observation_remain_distinct() {
    let mut episode = episode();
    let first = plan(episode.decide(observation(), 10));
    assert_eq!(extents(&first), [630, 162, 108]);
    assert_eq!(episode.baseline().observation.members[0].extent, 400.0);
    assert_eq!(episode.observed().members[0].extent, 400.0);
    assert_eq!(
        episode.requested().unwrap().commands,
        vec![CommandOutcome::Unsent; 3]
    );
    assert_eq!(episode.phase(), Phase::Applying);
    assert_eq!(episode.next_reconsideration(), Some(110));
    assert_eq!(
        episode.decide(observation(), 20),
        Decision::Noop(Noop::AwaitingExecution)
    );
}

#[test]
fn focus_inside_group_uses_baseline_not_the_previous_expansion() {
    let mut episode = episode();
    let (_, mut current) = settle_first(&mut episode);
    current.focus = focus("b");
    current.version += 1;
    let second = plan(episode.decide(current, 30));
    assert_eq!(extents(&second), [180, 630, 90]);
    assert_eq!(episode.baseline().allocation.extents, [400, 300, 200]);
    assert_eq!(episode.effects().len(), 2);
    assert_eq!(
        episode.effects()[0].commands,
        vec![CommandOutcome::Handled; 3]
    );
    assert!(matches!(
        episode.effects()[0].convergence,
        Convergence::Compatible {
            observed_version: 2
        }
    ));
}

#[test]
fn explicit_no_focus_restores_and_observes_restoration() {
    let mut episode = episode();
    let (_, mut current) = settle_first(&mut episode);
    current.focus = Focus::None;
    current.version += 1;
    let restore = plan(episode.decide(current, 30));
    assert_eq!(restore.kind, PlanKind::Restore);
    assert_eq!(extents(&restore), [400, 300, 200]);
    episode
        .record_execution(restore.id, vec![CommandOutcome::Handled; 3])
        .unwrap();
    assert_eq!(
        episode.decide(applied(&restore, 4), 40),
        Decision::Noop(Noop::Restored)
    );
    assert_eq!(episode.phase(), Phase::Restored);
    assert_eq!(episode.next_reconsideration(), None);
}

#[test]
fn another_workspace_with_equal_column_and_window_ids_is_outside() {
    let mut episode = episode();
    let (_, mut current) = settle_first(&mut episode);
    current.focus = Focus::Window(WindowKey {
        source: source(),
        workspace: "two".into(),
        id: "a".into(),
    });
    current.version += 1;
    assert_eq!(plan(episode.decide(current, 30)).kind, PlanKind::Restore);
}

#[test]
fn same_window_id_in_another_source_is_outside() {
    let mut episode = episode();
    let (_, mut current) = settle_first(&mut episode);
    current.focus = Focus::Window(WindowKey {
        source: Source {
            id: "umbriel-test".into(),
            epoch: 1,
        },
        workspace: "one".into(),
        id: "a".into(),
    });
    assert_eq!(plan(episode.decide(current, 30)).kind, PlanKind::Restore);
}

#[test]
fn unknown_focus_is_not_blur_and_does_not_authorize_ordinary_restoration() {
    let mut episode = episode();
    let (_, mut current) = settle_first(&mut episode);
    current.focus = Focus::Unknown;
    assert_eq!(
        episode.decide(current, 30),
        Decision::Suspend(Suspension::FocusUnknown)
    );
    assert_eq!(episode.phase(), Phase::Settled);
    assert_eq!(episode.effects().len(), 1);
}

#[test]
fn outside_focus_without_any_effect_never_restores() {
    let mut episode = episode();
    let mut current = observation();
    current.focus = Focus::None;
    assert_eq!(
        episode.decide(current, 10),
        Decision::Noop(Noop::FocusOutside)
    );
    assert!(episode.effects().is_empty());
}

#[test]
fn topology_move_delete_reorder_workspace_and_epoch_invalidate_pending_work() {
    for mutation in 0..7 {
        let mut episode = episode();
        let first = plan(episode.decide(observation(), 10));
        episode
            .record_command(first.id, 0, CommandOutcome::Handled)
            .unwrap();
        let mut current = observation();
        current.version += 1;
        match mutation {
            0 => current.members.swap(0, 1),
            1 => {
                current.members.remove(1);
            }
            2 => current.members[0].id = "replacement".into(),
            3 => current.group.topology = "column-1".into(),
            4 => current.group.workspace = "two".into(),
            5 => current.group.source.epoch += 1,
            6 => current.group.axis = Axis::Horizontal,
            _ => unreachable!(),
        }
        assert_eq!(
            episode.decide(current.clone(), 20),
            Decision::Suspend(Suspension::GroupIdentityChanged)
        );
        assert_eq!(episode.phase(), Phase::Lost);
        assert_eq!(episode.effects()[0].commands[0], CommandOutcome::Handled);
        assert_eq!(episode.effects()[0].convergence, Convergence::Abandoned);
        assert!(!matches!(
            episode.request_restore(current, 30),
            Decision::Plan(_)
        ));
    }
}

#[test]
fn changed_budget_gaps_or_bounds_revokes_old_restore_authority() {
    for mutation in 0..3 {
        let mut episode = episode();
        let (_, mut current) = settle_first(&mut episode);
        match mutation {
            0 => current.available_extent = 901.0,
            1 => current.gaps = Gaps::Between { pixels: 1 },
            2 => current.members[0].minimum = 100,
            _ => unreachable!(),
        }
        assert_eq!(
            episode.decide(current, 30),
            Decision::Suspend(Suspension::LayoutContextChanged)
        );
        assert_eq!(episode.phase(), Phase::Lost);
    }
}

#[test]
fn source_loss_retains_late_evidence_but_forbids_replay_and_restore() {
    let mut episode = episode();
    let first = plan(episode.decide(observation(), 10));
    episode
        .record_command(first.id, 0, CommandOutcome::Handled)
        .unwrap();
    let loss = episode.lose("bounded observer lag");
    assert_eq!(
        loss,
        Decision::Suspend(Suspension::SourceLost {
            reason: "bounded observer lag".into()
        })
    );
    episode
        .record_command(
            first.id,
            1,
            CommandOutcome::PossiblySent {
                message: "connection lost".into(),
            },
        )
        .unwrap();
    episode.finish_execution(first.id).unwrap();
    assert_eq!(episode.effects()[0].commands[0], CommandOutcome::Handled);
    assert!(matches!(
        episode.effects()[0].commands[1],
        CommandOutcome::PossiblySent { .. }
    ));
    assert_eq!(episode.next_reconsideration(), None);
    assert_eq!(episode.decide(applied(&first, 2), 20), loss);
    assert_eq!(episode.request_restore(applied(&first, 3), 30), loss);
}

#[test]
fn native_handling_and_intermediate_geometry_do_not_prove_settlement_or_conflict() {
    let mut episode = episode();
    let first = plan(episode.decide(observation(), 10));
    episode
        .record_execution(first.id, vec![CommandOutcome::Handled; 3])
        .unwrap();
    assert_eq!(
        episode.decide(observation(), 20),
        Decision::Noop(Noop::AwaitingConvergence)
    );
    let mut intermediate = observation();
    intermediate.version = 2;
    intermediate.members[0].extent = 550.0;
    intermediate.members[1].extent = 200.0;
    intermediate.members[2].extent = 150.0;
    assert_eq!(
        episode.decide(intermediate, 30),
        Decision::Noop(Noop::AwaitingConvergence)
    );
    assert_eq!(
        episode.decide(applied(&first, 3), 40),
        Decision::Noop(Noop::Stable)
    );
    assert_eq!(episode.baseline().observation.members[0].extent, 400.0);
}

#[test]
fn final_event_then_silence_fires_clock_injected_convergence_deadline() {
    let mut episode = episode();
    let first = plan(episode.decide(observation(), 10));
    episode
        .record_execution(first.id, vec![CommandOutcome::Handled; 3])
        .unwrap();
    assert_eq!(
        episode.decide(observation(), 109),
        Decision::Noop(Noop::AwaitingConvergence)
    );
    assert_eq!(episode.next_reconsideration(), Some(110));
    // Same capture, no further source event required.
    assert_eq!(
        episode.decide(observation(), 110),
        Decision::Suspend(Suspension::ConvergenceTimedOut)
    );
    assert_eq!(episode.phase(), Phase::Conflicted);
    assert_eq!(
        episode.requested().unwrap().convergence,
        Convergence::TimedOut
    );
    assert_eq!(episode.next_reconsideration(), None);
}

#[test]
fn unfinished_execution_deadline_cannot_trigger_resubmission() {
    let mut episode = episode();
    let first = plan(episode.decide(observation(), 10));
    assert_eq!(
        episode.decide(observation(), 110),
        Decision::Suspend(Suspension::ExecutionTimedOut)
    );
    episode
        .record_execution(first.id, vec![CommandOutcome::Handled; 3])
        .unwrap();
    assert_eq!(
        episode.decide(applied(&first, 2), 120),
        Decision::Suspend(Suspension::ExecutionTimedOut)
    );
    assert_eq!(episode.effects().len(), 1);
}

#[test]
fn focus_change_pending_does_not_drop_receipt_or_queue_stale_plans() {
    let mut episode = episode();
    let first = plan(episode.decide(observation(), 10));
    let mut current = observation();
    current.version = 2;
    current.focus = focus("b");
    assert_eq!(
        episode.decide(current, 20),
        Decision::Noop(Noop::AwaitingExecution)
    );
    episode
        .record_execution(first.id, vec![CommandOutcome::Handled; 3])
        .unwrap();
    let mut current = applied(&first, 3);
    current.focus = focus("c");
    let next = plan(episode.decide(current, 30));
    assert_eq!(next.active.unwrap().id, "c");
    assert_eq!(
        episode.effects()[0].commands,
        vec![CommandOutcome::Handled; 3]
    );
    assert_eq!(episode.effects().len(), 2);
}

#[test]
fn partial_rejection_is_retained_and_never_automatically_rolled_back() {
    let mut episode = episode();
    let first = plan(episode.decide(observation(), 10));
    let outcomes = vec![
        CommandOutcome::Handled,
        CommandOutcome::Rejected {
            message: "native hints".into(),
        },
        CommandOutcome::Unsent,
    ];
    episode
        .record_execution(first.id, outcomes.clone())
        .unwrap();
    assert_eq!(
        episode.decide(observation(), 20),
        Decision::Suspend(Suspension::CommandRejected)
    );
    assert_eq!(episode.effects()[0].commands, outcomes);
    assert!(!matches!(
        episode.request_restore(observation(), 30),
        Decision::Plan(_)
    ));
    assert_eq!(episode.effects().len(), 1);
}

#[test]
fn unknown_transmission_can_observe_compatibility_but_is_not_retried() {
    let mut episode = episode();
    let first = plan(episode.decide(observation(), 10));
    let unknown = CommandOutcome::PossiblySent {
        message: "reply lost".into(),
    };
    episode
        .record_execution(first.id, vec![unknown.clone(); 3])
        .unwrap();
    assert_eq!(
        episode.decide(observation(), 20),
        Decision::Noop(Noop::AwaitingConvergence)
    );
    assert_eq!(episode.effects().len(), 1);
    assert_eq!(
        episode.decide(applied(&first, 2), 30),
        Decision::Noop(Noop::Stable)
    );
    assert_eq!(
        episode.requested().unwrap().commands,
        [unknown.clone(), unknown.clone(), unknown]
    );
}

#[test]
fn late_native_rejection_revokes_a_previously_uncertain_settlement() {
    let mut episode = episode();
    let first = plan(episode.decide(observation(), 10));
    episode
        .record_execution(
            first.id,
            vec![
                CommandOutcome::PossiblySent {
                    message: "reply pending".into()
                };
                3
            ],
        )
        .unwrap();
    let mut current = applied(&first, 2);
    assert_eq!(
        episode.decide(current.clone(), 20),
        Decision::Noop(Noop::Stable)
    );
    episode
        .record_command(
            first.id,
            0,
            CommandOutcome::Rejected {
                message: "late refusal".into(),
            },
        )
        .unwrap();
    current.focus = Focus::None;
    assert_eq!(
        episode.decide(current, 30),
        Decision::Suspend(Suspension::CommandRejected)
    );
    assert_eq!(episode.effects().len(), 1);
}

#[test]
fn entirely_unsent_initial_plan_can_be_recomputed_without_fabricating_an_effect() {
    let mut episode = episode();
    let first = plan(episode.decide(observation(), 10));
    episode
        .record_execution(first.id, vec![CommandOutcome::Unsent; 3])
        .unwrap();
    let second = plan(episode.decide(observation(), 20));
    assert_ne!(first.id, second.id);
    assert_eq!(extents(&first), extents(&second));
    assert_eq!(episode.effects()[0].convergence, Convergence::NoEffect);
    assert_eq!(episode.observed().members[0].extent, 400.0);
}

#[test]
fn zero_observed_extent_can_be_intermediate_without_becoming_a_baseline_weight() {
    let mut episode = episode();
    let first = plan(episode.decide(observation(), 10));
    episode
        .record_execution(first.id, vec![CommandOutcome::Handled; 3])
        .unwrap();
    let mut current = observation();
    current.members[2].extent = 0.0;
    assert_eq!(
        episode.decide(current, 20),
        Decision::Noop(Noop::AwaitingConvergence)
    );
    assert_eq!(episode.baseline().observation.members[2].extent, 200.0);
}

#[test]
fn unexpected_settled_geometry_is_unattributed_not_learned_or_restored() {
    let mut episode = episode();
    let (_, mut current) = settle_first(&mut episode);
    current.members[0].extent += 20.0;
    current.focus = Focus::None;
    assert_eq!(
        episode.decide(current, 30),
        Decision::Suspend(Suspension::UnattributedGeometry)
    );
    assert_eq!(episode.baseline().allocation.extents, [400, 300, 200]);
    assert_eq!(episode.effects().len(), 1);
}

#[test]
fn fully_unsent_replacement_preserves_prior_settled_restore_obligation() {
    let mut episode = episode();
    let (_, mut current) = settle_first(&mut episode);
    current.focus = focus("b");
    current.version += 1;
    let second = plan(episode.decide(current.clone(), 30));
    episode
        .record_execution(second.id, vec![CommandOutcome::Unsent; 3])
        .unwrap();
    current.focus = Focus::None;
    current.version += 1;
    let restore = plan(episode.decide(current, 40));
    assert_eq!(restore.kind, PlanKind::Restore);
    assert_eq!(extents(&restore), [400, 300, 200]);
    assert_eq!(episode.effects()[1].convergence, Convergence::NoEffect);
}

#[test]
fn shutdown_latches_restore_intent_and_uses_normal_convergence() {
    let mut episode = episode();
    let (_, current) = settle_first(&mut episode);
    let restore = plan(episode.request_restore(current, 30));
    assert_eq!(restore.kind, PlanKind::Restore);
    episode
        .record_execution(restore.id, vec![CommandOutcome::Handled; 3])
        .unwrap();
    let current = applied(&restore, 3); // Focus is still inside the group.
    assert_eq!(
        episode.decide(current.clone(), 40),
        Decision::Noop(Noop::Restored)
    );
    assert_eq!(episode.decide(current, 50), Decision::Noop(Noop::Restored));
    assert_eq!(episode.effects().len(), 2);
}

#[test]
fn shutdown_during_apply_waits_then_conditionally_restores() {
    let mut episode = episode();
    let first = plan(episode.decide(observation(), 10));
    assert_eq!(
        episode.request_restore(observation(), 20),
        Decision::Noop(Noop::AwaitingExecution)
    );
    episode
        .record_execution(first.id, vec![CommandOutcome::Handled; 3])
        .unwrap();
    let restore = plan(episode.decide(applied(&first, 2), 30));
    assert_eq!(restore.kind, PlanKind::Restore);
}

#[test]
fn local_plan_admission_checks_identity_focus_geometry_and_capture_order() {
    let episode = episode();
    let first = plan(episode.preview(observation(), 10));
    assert_eq!(first.validate(&observation()), Ok(()));
    let mut current = observation();
    current.version = 0;
    assert_eq!(
        first.validate(&current),
        Err(PlanRejection::StaleObservation)
    );
    current = observation();
    current.focus = focus("b");
    assert_eq!(first.validate(&current), Err(PlanRejection::FocusChanged));
    current = observation();
    current.members.swap(0, 1);
    assert_eq!(
        first.validate(&current),
        Err(PlanRejection::IdentityChanged)
    );
    current = observation();
    current.members[0].extent += 1.0;
    assert_eq!(
        first.validate(&current),
        Err(PlanRejection::GeometryChanged)
    );
    assert_eq!(first.validate_identity(&current), Ok(()));
}

#[test]
fn evidence_reports_are_atomic_and_never_downgrade_handled_or_final_unsent() {
    let mut episode = episode();
    let first = plan(episode.decide(observation(), 10));
    episode
        .record_command(first.id, 0, CommandOutcome::Handled)
        .unwrap();
    let before = episode.effects().to_vec();
    assert_eq!(
        episode.record_execution(first.id, vec![CommandOutcome::Handled]),
        Err(EvidenceError::InvalidCommandCount)
    );
    assert_eq!(episode.effects(), before);
    assert_eq!(
        episode.record_execution(first.id, vec![CommandOutcome::Unsent; 3]),
        Err(EvidenceError::ContradictoryEvidence)
    );
    assert_eq!(episode.effects(), before);
    episode
        .record_execution(
            first.id,
            vec![
                CommandOutcome::Handled,
                CommandOutcome::Unsent,
                CommandOutcome::Unsent,
            ],
        )
        .unwrap();
    assert_eq!(
        episode.record_command(first.id, 1, CommandOutcome::Handled),
        Err(EvidenceError::ContradictoryEvidence)
    );
}

#[test]
fn stale_observation_is_ignored_and_regressed_clock_suspends_safely() {
    let mut episode = episode();
    let (_, current) = settle_first(&mut episode);
    assert_eq!(
        episode.decide(observation(), 30),
        Decision::Suspend(Suspension::StaleObservation)
    );
    assert_eq!(episode.phase(), Phase::Settled);
    assert_eq!(episode.observed(), &current);
    assert_eq!(
        episode.decide(current, 29),
        Decision::Suspend(Suspension::ClockRegressed)
    );
}

#[test]
fn malformed_inputs_and_infeasible_baselines_are_rejected() {
    let mut current = observation();
    current.members[0].extent = f64::NAN;
    assert!(matches!(
        Episode::new(current, Policy::default()),
        Err(InputError::InvalidGeometry { .. })
    ));
    let mut current = observation();
    current.members[1].id = "a".into();
    assert!(matches!(
        Episode::new(current, Policy::default()),
        Err(InputError::DuplicateMember { .. })
    ));
    let mut current = observation();
    current.members[0].minimum = 901;
    assert!(matches!(
        Episode::new(current, Policy::default()),
        Err(InputError::Allocation(_))
    ));
    assert!(matches!(
        Episode::new(
            observation(),
            Policy {
                active_share: 1.0,
                ..Policy::default()
            }
        ),
        Err(InputError::InvalidPolicy)
    ));
}

#[test]
fn plans_decisions_and_retained_episode_status_are_json_serializable() {
    let mut episode = episode();
    let first = episode.decide(observation(), 10);
    let json = serde_json::to_value(&first).unwrap();
    assert_eq!(json["Plan"]["targets"][0]["extent"], 630);
    let status = serde_json::to_value(&episode).unwrap();
    assert_eq!(
        status["baseline"]["observation"]["members"][0]["extent"],
        400.0
    );
    assert_eq!(status["observed"]["members"][0]["extent"], 400.0);
    assert_eq!(status["effects"][0]["commands"][0], "Unsent");
    assert_eq!(serde_json::from_value::<Decision>(json).unwrap(), first);
}
