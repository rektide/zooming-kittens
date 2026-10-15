use zk_layout::{Emphasis, Error, Gaps, Infeasible, Member, Request, allocate};

fn solve(members: &[Member], extent: f64, emphasis: Option<Emphasis>) -> Vec<u32> {
    allocate(Request {
        extent,
        gaps: Gaps::AlreadyExcluded,
        members,
        emphasis,
    })
    .unwrap()
    .extents
}

fn emphasis(index: usize) -> Option<Emphasis> {
    Some(Emphasis { index, share: 0.7 })
}

#[test]
fn canonical_seventy_thirty_uses_baseline_relative_siblings() {
    let members = [Member::new(400.0), Member::new(300.0), Member::new(200.0)];
    assert_eq!(solve(&members, 900.0, emphasis(0)), [630, 162, 108]);
    assert_eq!(solve(&members, 900.0, emphasis(1)), [180, 630, 90]);
    assert_eq!(solve(&members, 900.0, None), [400, 300, 200]);
}

#[test]
fn minimum_saturation_redistributes_repeatedly() {
    let members = [
        Member::new(400.0),
        Member {
            minimum: 200,
            ..Member::new(300.0)
        },
        Member {
            minimum: 150,
            ..Member::new(200.0)
        },
    ];
    assert_eq!(solve(&members, 900.0, emphasis(0)), [550, 200, 150]);
}

#[test]
fn maximum_saturation_cascades() {
    let members = [
        Member {
            maximum: Some(300),
            ..Member::new(400.0)
        },
        Member {
            maximum: Some(320),
            ..Member::new(300.0)
        },
        Member::new(200.0),
    ];
    assert_eq!(solve(&members, 900.0, emphasis(0)), [300, 320, 280]);
}

#[test]
fn mixed_bounds_are_conserved() {
    let members = [
        Member {
            maximum: Some(500),
            ..Member::new(400.0)
        },
        Member {
            minimum: 250,
            ..Member::new(300.0)
        },
        Member {
            minimum: 100,
            maximum: Some(150),
            ..Member::new(200.0)
        },
    ];
    assert_eq!(solve(&members, 900.0, emphasis(0)), [500, 250, 150]);
}

#[test]
fn infeasible_constraints_are_not_overflowing_layouts() {
    let members = [Member {
        minimum: 60,
        ..Member::new(1.0)
    }; 2];
    assert_eq!(
        allocate(Request {
            extent: 100.0,
            gaps: Gaps::AlreadyExcluded,
            members: &members,
            emphasis: None
        }),
        Err(Error::Infeasible(Infeasible::MinimaExceedBudget {
            required: 120,
            available: 100
        }))
    );
    let members = [Member {
        maximum: Some(40),
        ..Member::new(1.0)
    }; 2];
    assert_eq!(
        allocate(Request {
            extent: 100.0,
            gaps: Gaps::AlreadyExcluded,
            members: &members,
            emphasis: None
        }),
        Err(Error::Infeasible(Infeasible::MaximaBelowBudget {
            capacity: 80,
            available: 100
        }))
    );
}

#[test]
fn gaps_are_explicit_and_never_double_counted() {
    let members = [Member::new(400.0), Member::new(300.0), Member::new(200.0)];
    let allocation = allocate(Request {
        extent: 920.0,
        gaps: Gaps::Between { pixels: 10 },
        members: &members,
        emphasis: emphasis(0),
    })
    .unwrap();
    assert_eq!(allocation.extents, [630, 162, 108]);
    assert_eq!(allocation.content_budget, 900);
    assert_eq!(allocation.gap_budget, 20);
    assert_eq!(allocation.total_budget, 920);
    assert_eq!(solve(&members, 900.0, emphasis(0)), allocation.extents);
    assert!(matches!(
        allocate(Request {
            extent: 10.0,
            gaps: Gaps::Between { pixels: 10 },
            members: &members,
            emphasis: None,
        }),
        Err(Error::Infeasible(Infeasible::GapsExceedBudget))
    ));
}

#[test]
fn integer_rounding_is_conserving_and_ordered() {
    let members = [Member::new(1.0); 3];
    assert_eq!(solve(&members, 10.9, None), [4, 3, 3]);
    let members = [Member::new(2.0), Member::new(1.0), Member::new(3.0)];
    assert_eq!(solve(&members, 600.0, emphasis(1)), [72, 420, 108]);
    for budget in 0..300 {
        let output = solve(&members, f64::from(budget), emphasis(1));
        assert_eq!(output.iter().sum::<u32>(), budget);
        assert_eq!(solve(&members, f64::from(budget), emphasis(1)), output);
    }
}

#[test]
fn zero_budget_is_valid_only_if_minima_allow_it() {
    assert_eq!(solve(&[Member::new(1.0); 3], 0.0, emphasis(0)), [0, 0, 0]);
    let member = Member {
        minimum: 1,
        ..Member::new(1.0)
    };
    assert!(matches!(
        allocate(Request {
            extent: 0.0,
            gaps: Gaps::AlreadyExcluded,
            members: &[member],
            emphasis: None,
        }),
        Err(Error::Infeasible(Infeasible::MinimaExceedBudget { .. }))
    ));
}

#[test]
fn single_member_receives_entire_budget() {
    assert_eq!(solve(&[Member::new(42.0)], 900.0, emphasis(0)), [900]);
}

#[test]
fn invalid_finite_ranges_do_not_enter_taffy() {
    for extent in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, f64::MAX] {
        assert_eq!(
            allocate(Request {
                extent,
                gaps: Gaps::AlreadyExcluded,
                members: &[Member::new(1.0)],
                emphasis: None,
            }),
            Err(Error::InvalidExtent)
        );
    }
    for weight in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(
            allocate(Request {
                extent: 900.0,
                gaps: Gaps::AlreadyExcluded,
                members: &[Member::new(weight)],
                emphasis: None,
            }),
            Err(Error::InvalidWeight { index: 0 })
        );
    }
    for share in [0.0, 1.0, -0.1, f64::NAN, f64::INFINITY] {
        assert_eq!(
            allocate(Request {
                extent: 900.0,
                gaps: Gaps::AlreadyExcluded,
                members: &[Member::new(1.0)],
                emphasis: Some(Emphasis { index: 0, share }),
            }),
            Err(Error::InvalidEmphasis)
        );
    }
    assert_eq!(
        allocate(Request {
            extent: 900.0,
            gaps: Gaps::AlreadyExcluded,
            members: &[Member {
                minimum: 100,
                maximum: Some(99),
                ..Member::new(1.0)
            }],
            emphasis: None,
        }),
        Err(Error::InvalidBounds { index: 0 })
    );
}

#[test]
fn large_finite_weights_are_normalized_without_overflow() {
    assert_eq!(
        solve(&[Member::new(f64::MAX); 3], 900.0, emphasis(0)),
        [630, 135, 135]
    );
}

#[test]
fn bounded_grid_preserves_every_integer_and_bound() {
    for budget in 1..120 {
        for minimum in [0, 5, 20] {
            let members = [
                Member {
                    minimum,
                    maximum: Some(60),
                    ..Member::new(7.0)
                },
                Member {
                    minimum,
                    maximum: Some(50),
                    ..Member::new(2.0)
                },
                Member {
                    minimum,
                    maximum: Some(40),
                    ..Member::new(1.0)
                },
            ];
            if budget < minimum * 3 {
                continue;
            }
            let output = solve(&members, f64::from(budget), emphasis(0));
            assert_eq!(output.iter().sum::<u32>(), budget);
            for (value, member) in output.iter().zip(members) {
                assert!(*value >= member.minimum);
                assert!(*value <= member.maximum.unwrap());
            }
        }
    }
}
