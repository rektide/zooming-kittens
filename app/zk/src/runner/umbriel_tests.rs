use super::*;
use zk_desktop::*;
use zk_longcat::{Phase, PlanKind};

fn capture() -> DesktopCapture {
    let scope = Scope {
        source: SourceId::new(Backend::Umbriel, "test"),
        epoch: Epoch(1),
    };
    let workspace = WorkspaceKey {
        scope: scope.clone(),
        native: NativeId::Text("ws".into()),
    };
    let group = GroupKey {
        workspace: workspace.clone(),
        position: NativeId::Number(0),
    };
    let mut capture = DesktopCapture::empty(
        scope.clone(),
        Capabilities {
            reported_groups: true,
            height_by_id: false,
            atomic_multiwindow: true,
            native_revision_precondition: false,
            exact_restore: true,
        },
    );
    capture.control_status = Some(FamilyStatus {
        stamp: Some(FamilyStamp {
            epoch: Epoch(1),
            position: 1,
        }),
        health: Health::Current,
    });
    capture.version.0 = 1;
    let members = ["a", "b"]
        .into_iter()
        .map(|id| WindowKey {
            scope: scope.clone(),
            native: NativeId::Text(id.into()),
        })
        .collect::<Vec<_>>();
    // Native constraints can produce a baseline whose pixel total differs from
    // the conserving content budget. It must still be restored as native state.
    for (id, height) in members.iter().zip([400.0, 220.0]) {
        capture.windows.insert(
            id.clone(),
            WindowView {
                key: id.clone(),
                workspace: Some(workspace.clone()),
                group: Some(group.clone()),
                placement: Placement::Tiled,
                title: None,
                app_id: None,
                pid: None,
                active: id == &members[0],
                geometry: Geometry {
                    content_size: Some(Size {
                        width: 600.0,
                        height,
                    }),
                    tile_size: Some(Size {
                        width: 600.0,
                        height: height + 20.0,
                    }),
                    ..Geometry::default()
                },
            },
        );
    }
    capture.active_window = Some(members[0].clone());
    capture.workspaces.insert(
        workspace.clone(),
        WorkspaceView {
            key: workspace,
            name: None,
            output: Some("OUT".into()),
            active: true,
            focused: true,
            layout: Some("scrolling".into()),
            geometry: None,
            grouping: Evidence::Reported,
        },
    );
    capture.groups.insert(
        group.clone(),
        GroupView {
            key: group.clone(),
            members,
            secondary_axis: Axis::Vertical,
            membership: Evidence::Reported,
            bounds: None,
            geometry_evidence: Evidence::Reported,
        },
    );
    capture.allocations.insert(
        group,
        ConditionalAllocation {
            basis: Arc::from(&b"original-basis"[..]),
            context: Arc::from(&b"context"[..]),
            restore_payload: Arc::from(&b"raw-original-weights"[..]),
            content_budget: 618,
            members: vec![
                AllocationMember {
                    target_content: 400,
                    decoration: 20,
                    minimum: 50,
                    maximum: None,
                },
                AllocationMember {
                    target_content: 220,
                    decoration: 20,
                    minimum: 50,
                    maximum: None,
                },
            ],
            edge_gaps: (10, 20),
        },
    );
    capture
}

fn changed(
    capture: &DesktopCapture,
    sequence: &Sequence,
    basis: &'static [u8],
) -> (DesktopCapture, ConditionalAllocation) {
    let mut next = capture.clone();
    next.version.0 += 1;
    let allocation = next.allocations.get_mut(&sequence.key).unwrap();
    allocation.basis = Arc::from(basis);
    for (member, target) in allocation.members.iter_mut().zip(&sequence.plan.targets) {
        member.target_content = target.extent;
    }
    let allocation = allocation.clone();
    for (id, target) in next.groups[&sequence.key]
        .members
        .iter()
        .zip(&sequence.plan.targets)
    {
        next.windows
            .get_mut(id)
            .unwrap()
            .geometry
            .content_size
            .as_mut()
            .unwrap()
            .height = f64::from(target.extent);
    }
    (next, allocation)
}

#[test]
fn group_receipt_restores_native_targets_and_original_weights_not_conserving_pixels() {
    let initial = capture();
    let mut control = Control::new(&LongcatConfig::default(), false, 1500);
    let (_, some) = control.evaluate(&initial, 0, None, "initial").unwrap();
    let emphasize = some.unwrap();
    assert!(emphasize.group_submission);
    assert_eq!(
        emphasize.plan.targets.iter().map(|t| t.extent).sum::<u32>(),
        618
    );
    let native::Request::Group(request) = control.request(&initial, &emphasize).unwrap() else {
        panic!("not group")
    };
    assert!(request.restore_payload.is_none());
    assert_eq!(request.edge_gaps, (10, 20));
    let (applied, receipt) = changed(&initial, &emphasize, b"accepted-emphasis");
    control.accept_group(&emphasize, receipt).unwrap();
    control.finish(&emphasize).unwrap();
    // Reply alone cannot settle before its basis is observed.
    assert!(
        control
            .evaluate(&initial, 1, None, "old capture")
            .unwrap()
            .1
            .is_none()
    );
    control.evaluate(&applied, 2, None, "applied").unwrap();
    let slot = &control.slots[&emphasize.key];
    assert_eq!(slot.episode.phase(), Phase::Settled);
    let effect = slot.episode.requested().unwrap();
    assert!(effect.commands.is_empty());
    assert_eq!(effect.group_command, Some(CommandOutcome::Handled));
    assert_eq!(
        slot.episode
            .baseline()
            .allocation
            .extents
            .iter()
            .sum::<u32>(),
        618
    );
    let mut blurred = applied.clone();
    blurred.version.0 += 1;
    blurred.active_window = None;
    let (_, restore) = control.evaluate(&blurred, 3, None, "blur").unwrap();
    let restore = restore.unwrap();
    assert_eq!(restore.plan.kind, PlanKind::Restore);
    assert_eq!(
        restore
            .plan
            .targets
            .iter()
            .map(|t| t.extent)
            .collect::<Vec<_>>(),
        [400, 220]
    );
    let native::Request::Group(request) = control.request(&blurred, &restore).unwrap() else {
        panic!("not group")
    };
    assert_eq!(&*request.expected, b"accepted-emphasis");
    assert_eq!(&*request.restore_payload.unwrap(), b"raw-original-weights");
    assert!(request.expected_active.is_none());
    let (restored, receipt) = changed(&blurred, &restore, b"accepted-restore");
    control.accept_group(&restore, receipt).unwrap();
    control.finish(&restore).unwrap();
    control.evaluate(&restored, 4, None, "restore").unwrap();
    assert_eq!(control.slots[&restore.key].episode.phase(), Phase::Restored);
}

#[test]
fn same_pixel_external_basis_change_and_uncertain_send_revoke_native_authority() {
    for uncertain in [false, true] {
        let initial = capture();
        let mut control = Control::new(&LongcatConfig::default(), false, 1500);
        let sequence = control
            .evaluate(&initial, 0, None, "initial")
            .unwrap()
            .1
            .unwrap();
        let (mut applied, receipt) = changed(&initial, &sequence, b"accepted");
        if uncertain {
            control
                .record(
                    &sequence,
                    CommandOutcome::PossiblySent {
                        message: "lost reply".into(),
                    },
                )
                .unwrap();
            control.finish(&sequence).unwrap();
        } else {
            control.accept_group(&sequence, receipt).unwrap();
            control.finish(&sequence).unwrap();
            control.evaluate(&applied, 1, None, "settled").unwrap();
            applied.allocations.get_mut(&sequence.key).unwrap().basis =
                Arc::from(&b"external-same-pixels"[..]);
        }
        applied.active_window = None;
        applied.version.0 += 1;
        assert!(
            control
                .evaluate(&applied, 2, None, "external/lost")
                .unwrap()
                .1
                .is_none()
        );
        assert_eq!(control.slots[&sequence.key].episode.phase(), Phase::Lost);
        assert!(control.request(&applied, &sequence).is_err());
    }
}
