//! Stock projection. Native placement is reported; physical axis is inferred
//! from separated content rectangles, NOT from coincident x coordinates.
use std::collections::{BTreeMap, BTreeSet};
use umbriel_protocol::model::{Window, WindowId};
use umbriel_state::inventory::FamilySnapshot;
use zk_desktop::*;

pub(crate) fn project_groups(
    capture: &mut DesktopCapture,
    native: &FamilySnapshot<WindowId, Window>,
) {
    // Reported (-1,-1) means outside the scrolling strip, not a missing sibling.
    // An old payload or half-located row cannot establish complete membership.
    let mut incomplete = BTreeSet::new();
    let mut groups: BTreeMap<GroupKey, Vec<(i32, WindowKey)>> = BTreeMap::new();
    for w in capture.windows.values() {
        let NativeId::Text(id) = &w.key.native else {
            unreachable!()
        };
        let row = &native.entries()[&WindowId::from(id.clone())];
        if w.placement == Placement::Tiled
            && w.group.is_none()
            && (row.column, row.row) != (Some(-1), Some(-1))
        {
            incomplete.extend(w.workspace.clone());
        }
        if let Some(group) = &w.group {
            groups
                .entry(group.clone())
                .or_default()
                .push((row.row.expect("located row"), w.key.clone()));
        }
    }
    for workspace in capture.workspaces.values_mut() {
        let complete = !incomplete.contains(&workspace.key);
        if complete
            && workspace.layout.as_deref() == Some("scrolling")
            && capture.windows_status.is_current()
            && capture.workspaces_status.is_current()
        {
            workspace.grouping = Evidence::Reported;
        }
    }
    for (key, mut rows) in groups {
        rows.sort();
        let complete = rows
            .iter()
            .enumerate()
            .all(|(index, (row, _))| *row as usize == index)
            && !incomplete.contains(&key.workspace);
        let members: Vec<_> = rows.into_iter().map(|(_, id)| id).collect();
        let windows: Vec<_> = members.iter().map(|id| &capture.windows[id]).collect();
        let axis = infer_axis(&windows);
        let eligible = eligibility(capture, &key, &windows, complete, axis);
        capture.groups.insert(
            key.clone(),
            GroupView {
                key,
                members,
                secondary_axis: axis,
                membership: if complete {
                    Evidence::Reported
                } else {
                    Evidence::Unavailable
                },
                bounds: None,
                geometry_evidence: if axis.is_some() {
                    Evidence::Inferred
                } else {
                    Evidence::Unavailable
                },
                edge_eligibility: eligible,
            },
        );
    }
    capture.capabilities.reported_groups = capture
        .groups
        .values()
        .any(|g| g.membership == Evidence::Reported);
    capture.capabilities.relative_edge_by_id = capture
        .groups
        .values()
        .any(|g| g.edge_eligibility == EdgeEligibility::Eligible);
}

fn eligibility(
    c: &DesktopCapture,
    key: &GroupKey,
    windows: &[&WindowView],
    complete: bool,
    axis: Option<Axis>,
) -> EdgeEligibility {
    if !c.is_ready() {
        return EdgeEligibility::FamilyUnavailable;
    }
    if !complete {
        return EdgeEligibility::InconsistentMembership;
    }
    let Some(ws) = c.workspaces.get(&key.workspace) else {
        return EdgeEligibility::InconsistentMembership;
    };
    if ws.layout.as_deref() != Some("scrolling") {
        return EdgeEligibility::UnsupportedLayout;
    }
    let Some(name) = &ws.output else {
        return EdgeEligibility::OutputUnavailable;
    };
    let output_key = OutputKey {
        scope: key.workspace.scope.clone(),
        name: name.clone(),
    };
    let Some(output) = c.outputs.get(&output_key) else {
        return EdgeEligibility::OutputUnavailable;
    };
    if output.enabled != Some(true)
        || output.powered != Some(true)
        || !output
            .usable_area
            .is_some_and(|r| r.size.width > 0. && r.size.height > 0.)
        || !output
            .logical_size
            .is_some_and(|s| s.width > 0. && s.height > 0.)
        || output.position.is_none()
    {
        return EdgeEligibility::OutputUnavailable;
    }
    if windows
        .iter()
        .any(|w| w.output.as_ref() != Some(&output_key))
    {
        return EdgeEligibility::InconsistentMembership;
    }
    if windows
        .iter()
        .any(|w| w.placement != Placement::Tiled || !w.presentation.ordinary_untabbed())
    {
        return EdgeEligibility::UnsupportedPresentation;
    }
    if windows.len() < 2 {
        return EdgeEligibility::TooFewMembers;
    }
    if axis.is_none() {
        return EdgeEligibility::AmbiguousAxis;
    }
    EdgeEligibility::Eligible
}

/// Candidate evidence only: the controller must also wait for relevant geometry
/// quietness. Mixed target-position/committed-size frames are not allocations.
pub fn infer_axis(windows: &[&WindowView]) -> Option<Axis> {
    if windows.len() < 2 {
        return None;
    }
    let rects = windows
        .iter()
        .map(|w| Some((w.geometry.target_position?, w.geometry.content_size?)))
        .collect::<Option<Vec<_>>>()?;
    if rects.iter().any(|(p, s)| {
        ![p.x, p.y, s.width, s.height]
            .into_iter()
            .all(f64::is_finite)
            || s.width <= 0.
            || s.height <= 0.
    }) {
        return None;
    }
    let separated = |vertical: bool| {
        let along = |(p, s): &(Point, Size)| {
            if vertical {
                (p.y, s.height)
            } else {
                (p.x, s.width)
            }
        };
        let across = |(p, s): &(Point, Size)| {
            if vertical {
                (p.x, s.width)
            } else {
                (p.y, s.height)
            }
        };
        let start = rects
            .iter()
            .map(|r| across(r).0)
            .fold(f64::NEG_INFINITY, f64::max);
        let end = rects
            .iter()
            .map(|r| {
                let (p, h) = across(r);
                p + h
            })
            .fold(f64::INFINITY, f64::min);
        start < end
            && rects.windows(2).all(|pair| {
                let (p, h) = along(&pair[0]);
                p + h <= along(&pair[1]).0
            })
    };
    match (separated(false), separated(true)) {
        (true, false) => Some(Axis::Horizontal),
        (false, true) => Some(Axis::Vertical),
        _ => None,
    }
}
