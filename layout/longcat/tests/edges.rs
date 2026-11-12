use zk_longcat::{Axis, GroupKey, Source, Target, feedback::edges::*, feedback::*};

fn goal(desired: &[u32], minimum: &[u32]) -> FeedbackGoal {
    let source = Source {
        id: "pure".into(),
        epoch: 1,
    };
    FeedbackGoal {
        id: 1,
        inputs: GoalInputs {
            group: GroupKey {
                source,
                workspace: "ws".into(),
                topology: "col".into(),
                axis: Axis::Vertical,
            },
            context: "ctx".into(),
            content_budget: desired.iter().sum::<u32>() as f64,
            active: None,
            members: minimum
                .iter()
                .enumerate()
                .map(|(i, min)| GoalMember {
                    id: i.to_string(),
                    neutral_weight: 1.0,
                    learned_share: None,
                    minimum_extent: *min,
                    maximum_extent: None,
                })
                .collect(),
        },
        preference_generation: 1,
        basis_version: 1,
        targets: desired
            .iter()
            .enumerate()
            .map(|(i, extent)| Target {
                member: i.to_string(),
                extent: *extent,
            })
            .collect(),
        created_at_ms: 0,
        deadline_ms: 1000,
        geometry_tolerance: 2,
        submission_limit: 8 * (desired.len() - 1) + 2,
    }
}

fn simulate(mut current: Vec<f64>, desired: &[u32]) -> Vec<usize> {
    let goal = goal(desired, &vec![100; desired.len()]);
    let mut boundaries = vec![];
    for _ in 0..goal.submission_limit {
        match select_edge(&goal, &current, None) {
            EdgeSelection::Step(step) => {
                current[step.boundary] += step.travel;
                current[step.boundary + 1] -= step.travel;
                assert!(current.iter().all(|v| *v >= 100.0));
                boundaries.push(step.boundary);
            }
            EdgeSelection::PositionsMatch => {
                assert!(
                    current
                        .iter()
                        .zip(desired)
                        .all(|(a, b)| (a - f64::from(*b)).abs() <= 2.0)
                );
                return boundaries;
            }
            other => panic!("unexpected {other:?}"),
        }
    }
    panic!("did not converge")
}

#[test]
fn donor_chains_choose_available_slack_in_both_directions() {
    assert_eq!(simulate(vec![100., 100., 500.], &[500, 100, 100]), [1, 0]);
    assert_eq!(simulate(vec![500., 100., 100.], &[100, 100, 500]), [0, 1]);
    simulate(vec![450., 180., 270.], &[180, 450, 270]);
    simulate(vec![150., 250., 350., 450.], &[500, 150, 400, 150]);
}

#[test]
fn blocked_directions_reopen_only_after_adjacent_geometry_changes() {
    let goal = goal(&[500, 100, 100], &[100, 100, 100]);
    let current = [100., 100., 500.];
    let mut attempt = FeedbackAttempt {
        goal: goal.clone(),
        commands: vec![EdgeEvidence {
            key: CommandKey {
                attempt: 1,
                sequence: 1,
            },
            intent: EdgeIntent {
                basis_version: 1,
                boundary: 1,
                travel: 400.,
            },
            created_at_ms: 0,
            outcome: Some(zk_longcat::CommandOutcome::Handled),
            response: Some(EdgeResponse {
                observed_version: 1,
                kind: ResponseKind::Blocked {
                    reason: "native clamp".into(),
                },
            }),
            response_estimates: Some([100., 500.]),
        }],
        stopping: None,
        result: None,
    };
    assert!(matches!(
        select_edge(&goal, &current, Some(&attempt)),
        EdgeSelection::Limited { .. }
    ));
    assert!(matches!(
        select_edge(&goal, &[100., 200., 400.], Some(&attempt)),
        EdgeSelection::Step(_)
    ));
    attempt.goal.id = 2;
    assert!(matches!(
        select_edge(&goal, &current, Some(&attempt)),
        EdgeSelection::Limited { .. }
    ));
}

#[test]
fn known_recipient_maximum_and_bad_bases_are_not_ignored() {
    let mut goal = goal(&[500, 100, 100], &[100, 100, 100]);
    goal.inputs.members[1].maximum_extent = Some(100);
    assert!(matches!(
        select_edge(&goal, &[100., 100., 500.], None),
        EdgeSelection::Limited { .. }
    ));
    assert!(matches!(
        select_edge(&goal, &[100., 100., 499.], None),
        EdgeSelection::Limited { .. }
    ));
    assert!(matches!(
        select_edge(&goal, &[f64::NAN, 100., 500.], None),
        EdgeSelection::Limited { .. }
    ));
}
