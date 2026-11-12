//! Opt-in real arrangement tests; never point this at the running desktop.
use std::{sync::Arc, time::Duration};
use zk_desktop::*;
use zk_longcat::{
    CommandOutcome, Target,
    feedback::{self as f, edges::*},
};
use zk_umbriel::{
    Source,
    edge::{EdgeActuator, Transmission},
    geometry::*,
};

fn goal(model: &FrozenGeometry, desired: &[u32], id: u64) -> f::FeedbackGoal {
    let members: Vec<_> = desired
        .iter()
        .map(|d| zk_layout::Member {
            weight: f64::from(*d),
            minimum: 100,
            maximum: None,
        })
        .collect();
    let answer = zk_layout::allocate(zk_layout::Request {
        extent: model.content_budget(),
        gaps: zk_layout::Gaps::AlreadyExcluded,
        members: &members,
        emphasis: None,
    })
    .unwrap();
    let ids: Vec<_> = model
        .members()
        .iter()
        .map(|id| match &id.native {
            NativeId::Text(id) => id.clone(),
            _ => unreachable!(),
        })
        .collect();
    let source = zk_longcat::Source {
        id: "native-fixture".into(),
        epoch: model.group().workspace.scope.epoch.0,
    };
    f::FeedbackGoal {
        id,
        inputs: f::GoalInputs {
            group: zk_longcat::GroupKey {
                source,
                workspace: "harness".into(),
                topology: "fixture".into(),
                axis: match model.axis() {
                    Axis::Vertical => zk_longcat::Axis::Vertical,
                    Axis::Horizontal => zk_longcat::Axis::Horizontal,
                },
            },
            context: "fixture".into(),
            content_budget: model.content_budget(),
            active: None,
            members: ids
                .iter()
                .map(|id| f::GoalMember {
                    id: id.clone(),
                    neutral_weight: 1.,
                    learned_share: None,
                    minimum_extent: 100,
                    maximum_extent: None,
                })
                .collect(),
        },
        preference_generation: 1,
        basis_version: model.frame().version.0,
        targets: ids
            .into_iter()
            .zip(answer.extents)
            .map(|(member, extent)| Target { member, extent })
            .collect(),
        created_at_ms: 0,
        deadline_ms: 4000,
        geometry_tolerance: if desired.len() == 3
            && model.axis() == Axis::Vertical
            && std::env::var("ZK_GEOMETRY_STRUT").as_deref() == Ok("20")
        {
            0
        } else {
            2
        },
        submission_limit: 8 * (desired.len() - 1) + 2,
    }
}

async fn drive(
    source: &Source,
    observer: &mut Observer,
    actuator: &EdgeActuator,
    model: &mut FrozenGeometry,
    desired: &[u32],
    id: u64,
) -> Result<usize, String> {
    let goal = goal(model, desired, id);
    let mut attempt = f::FeedbackAttempt {
        goal: goal.clone(),
        commands: vec![],
        stopping: None,
        result: None,
    };
    let origin = tokio::time::Instant::now();
    // Models span goals; their clock must too.
    let base = id * 10_000;
    let refresh = source.windows_refresh().unwrap();
    loop {
        let now = base + origin.elapsed().as_millis() as u64;
        if origin.elapsed() >= Duration::from_secs(4) {
            return Err("convergence deadline".into());
        }
        let selection = select_edge(&goal, &model.frame().target_extents, Some(&attempt));
        match selection {
            EdgeSelection::PositionsMatch
                if model.frame().coherent
                    && model.frame().committed.iter().zip(&goal.targets).all(
                        |(extent, target)| {
                            *extent >= 100.
                                && (*extent - f64::from(target.extent)).abs()
                                    <= f64::from(goal.geometry_tolerance)
                        },
                    ) =>
            {
                return Ok(attempt.commands.len());
            }
            EdgeSelection::PositionsMatch => {
                tokio::time::sleep(Duration::from_millis(10)).await;
                let c = observer.current();
                model.observe(&c, now + 10)?;
            }
            EdgeSelection::Limited { reason } => return Err(reason),
            EdgeSelection::Step(step) => {
                if attempt.commands.len() >= goal.submission_limit {
                    return Err("submission budget".into());
                }
                let capture = observer.current();
                model.observe(&capture, now)?;
                let edge = model.compile(&capture, step.boundary, step.travel)?;
                let key = f::CommandKey {
                    attempt: id,
                    sequence: attempt.commands.len() as u64 + 1,
                };
                let motion = MotionId {
                    attempt: id,
                    sequence: key.sequence,
                };
                model.begin(motion, &edge, now)?;
                assert_eq!(
                    actuator.execute_frozen(edge.clone()).await,
                    Transmission::Handled
                );
                let mut evidence = f::EdgeEvidence {
                    key,
                    intent: f::EdgeIntent {
                        basis_version: capture.version.0,
                        boundary: step.boundary,
                        travel: edge.travel,
                    },
                    created_at_ms: now,
                    outcome: Some(CommandOutcome::Handled),
                    response: None,
                    response_estimates: None,
                };
                loop {
                    tokio::time::sleep(Duration::from_millis(4)).await;
                    let now = base + origin.elapsed().as_millis() as u64;
                    if origin.elapsed() >= Duration::from_secs(4) {
                        return Err("position deadline".into());
                    }
                    let c = observer.current();
                    let assessment = model.observe(&c, now)?;
                    if let Some(PositionResponse::Progress { version, .. }) = assessment.response {
                        evidence.response = Some(f::EdgeResponse {
                            observed_version: version.0,
                            kind: f::ResponseKind::Progress,
                        });
                        break;
                    }
                    match model.on_timer(now)? {
                        TimerAction::RefreshWindows(_) => {
                            // Install through the native/Tracker owner, never by
                            // synthesizing a late query-to-subscription ordering.
                            refresh
                                .refresh(&c.scope, Duration::from_millis(500))
                                .await?;
                        }
                        TimerAction::Blocked(_) => {
                            evidence.response = Some(f::EdgeResponse {
                                observed_version: model.frame().version.0,
                                kind: f::ResponseKind::Blocked {
                                    reason: "silent native boundary after refresh".into(),
                                },
                            });
                            break;
                        }
                        _ => {}
                    }
                }
                evidence.response_estimates = Some([
                    model.frame().target_extents[step.boundary],
                    model.frame().target_extents[step.boundary + 1],
                ]);
                attempt.commands.push(evidence);
            }
        }
    }
}

async fn quiet(observer: &mut Observer, count: usize) -> Arc<DesktopCapture> {
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut previous = None;
        let mut since = tokio::time::Instant::now();
        loop {
            let c = observer.current();
            let group = c.groups.values().find(|g| g.members.len() == count && g.edge_eligibility == EdgeEligibility::Eligible);
            let signature = group.map(|g| g.members.iter().map(|id| c.windows[id].geometry.clone()).collect::<Vec<_>>());
            if signature.is_some() && signature == previous && since.elapsed() >= Duration::from_millis(300) { return c; }
            if signature != previous { previous = signature; since = tokio::time::Instant::now(); }
            tokio::select! { _ = observer.recv() => {}, _ = tokio::time::sleep(Duration::from_millis(10)) => {} }
        }
    }).await.unwrap()
}

#[tokio::test]
#[ignore = "isolated native compositor only; run tests/native/check.sh"]
async fn native_taffy_internal_edges_strips_constraints_struts_and_slow_commits() {
    assert_eq!(std::env::var("ZK_HEADLESS_GEOMETRY").as_deref(), Ok("1"));
    assert!(std::env::var_os("UMBRIEL_HARNESS_LIB").is_some());
    let count: usize = std::env::var("ZK_GEOMETRY_COUNT").unwrap().parse().unwrap();
    let constraint = std::env::var("ZK_GEOMETRY_CONSTRAINT").unwrap();
    let endpoint = zk_umbriel::Endpoint::new(std::env::var_os("UMBRIEL_SOCKET").unwrap());
    let (source, mut observer) = zk_umbriel::subscribe(
        endpoint,
        SourceId::new(Backend::Umbriel, "native-geometry"),
        zk_umbriel::Options::default(),
    )
    .await
    .unwrap();
    let capture = quiet(&mut observer, count).await;
    let group = capture
        .groups
        .values()
        .find(|g| g.members.len() == count)
        .unwrap();
    let anchor = capture.active_window.clone().unwrap();
    let anchor_geometry = capture.windows[&anchor].geometry.clone();
    assert_eq!(
        capture.windows[&anchor].title.as_deref(),
        Some("zk-geometry-anchor")
    );
    let mut model = FrozenGeometry::acquire(&capture, &group.key, 2).unwrap();
    let expected_axis = if std::env::var("ZK_GEOMETRY_AXIS").as_deref() == Ok("horizontal") {
        Axis::Vertical
    } else {
        Axis::Horizontal
    };
    assert_eq!(model.axis(), expected_axis);
    let envelope = model.envelope();
    let strips = model.strips().to_vec();
    let budget = model.content_budget();
    assert!(strips.iter().all(|s| *s > 0.));
    if expected_axis == Axis::Vertical {
        assert!(
            strips.iter().any(|s| *s != strips[0]),
            "mixed bars must be measured, not guessed"
        );
    }
    let mut last = vec![100; count];
    last[count - 1] = budget as u32 - 100 * (count as u32 - 1);
    let mut first = vec![100; count];
    first[0] = last[count - 1];
    if constraint == "none" {
        for (index, desired) in [
            last,
            first,
            vec![1, 2, 3, 4][..count].iter().map(|v| v * 100).collect(),
        ]
        .iter()
        .enumerate()
        {
            let commands = drive(
                &source,
                &mut observer,
                &source.edge_actuator().await.unwrap(),
                &mut model,
                desired,
                index as u64 + 1,
            )
            .await
            .unwrap();
            println!(
                "native goal {} converged in {commands} commands: {:?}",
                index + 1,
                model.frame().committed
            );
            assert!(model.frame().committed.iter().all(|extent| *extent >= 100.));
            // In addition to tolerance-based fixtures, retain a genuinely exact
            // native round trip in both donor-chain directions and heterogeneous
            // allocation. This is not merely a handled action or position match.
            if count == 3
                && expected_axis == Axis::Vertical
                && std::env::var("ZK_GEOMETRY_STRUT").as_deref() == Ok("20")
            {
                let expected = goal(&model, desired, index as u64 + 1);
                assert_eq!(
                    model.frame().committed,
                    expected
                        .targets
                        .iter()
                        .map(|t| f64::from(t.extent))
                        .collect::<Vec<_>>()
                );
            }
            assert_eq!(model.content_budget(), budget);
            assert_eq!(model.envelope(), envelope);
            assert_eq!(model.strips(), strips);
            assert_eq!(observer.current().active_window.as_ref(), Some(&anchor));
            assert_eq!(
                observer.current().windows[&anchor].geometry,
                anchor_geometry
            );
        }
    } else {
        let result = drive(
            &source,
            &mut observer,
            &source.edge_actuator().await.unwrap(),
            &mut model,
            &first,
            1,
        )
        .await;
        assert!(
            result.is_err(),
            "native {constraint} must limit unreachable target"
        );
        println!(
            "native {constraint} bounded limitation: {}",
            result.unwrap_err()
        );
    }
    source.shutdown().await.unwrap();
}
