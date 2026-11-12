use super::types::{FeedbackGroup, FeedbackInput, Preference};
use crate::{Source, WindowKey};

pub(super) fn find<'a>(
    preferences: &'a [Preference],
    source: &Source,
    workspace: &str,
    id: &str,
) -> Option<&'a Preference> {
    preferences.iter().find(|p| {
        p.window.source == *source && p.window.workspace == workspace && p.window.id == id
    })
}

/// Full-census reconciliation, not group membership reconciliation. A cross-group
/// move carries the weight/share; a workspace move carries only neutral weight.
pub(super) fn reconcile(preferences: &mut Vec<Preference>, input: &FeedbackInput) -> bool {
    let before = preferences.clone();
    preferences.retain_mut(|p| {
        let Some(live) = input.live_members.iter().find(|m| m.id == p.window.id) else {
            return false;
        };
        if p.window.workspace != live.workspace {
            p.window.workspace.clone_from(&live.workspace);
            p.active_share = None;
            p.manual_floor = None;
        }
        if p.context != live.context {
            p.context.clone_from(&live.context);
            p.manual_floor = None;
        }
        true
    });
    *preferences != before
}

/// Seed only from quiet coherent content, never from an in-flight effect. Known
/// identities keep their existing weight even when acquired in another group.
pub(super) fn acquire(
    preferences: &mut Vec<Preference>,
    input: &FeedbackInput,
    group: &FeedbackGroup,
) -> bool {
    let mut changed = false;
    for member in &group.members {
        if find(preferences, &input.source, &group.key.workspace, &member.id).is_some() {
            continue;
        }
        let live = input
            .live_members
            .iter()
            .find(|m| m.id == member.id)
            .expect("validated census");
        preferences.push(Preference {
            window: WindowKey {
                source: input.source.clone(),
                workspace: group.key.workspace.clone(),
                id: member.id.clone(),
            },
            neutral_weight: member.extent,
            active_share: None,
            manual_floor: None,
            context: live.context.clone(),
        });
        changed = true;
    }
    changed
}

pub(super) fn learn(
    preferences: &mut [Preference],
    group: &FeedbackGroup,
    emphasized: Option<&str>,
    expected: &FeedbackGroup,
    tolerance: u32,
) -> bool {
    let before = preferences.to_vec();
    let budget: f64 = group.members.iter().map(|m| m.extent).sum();
    let other_extent: f64 = group
        .members
        .iter()
        .filter(|m| Some(m.id.as_str()) != emphasized)
        .map(|m| m.extent)
        .sum();
    let other_weight: f64 = group
        .members
        .iter()
        .filter(|m| Some(m.id.as_str()) != emphasized)
        .map(|m| {
            find(preferences, &group.key.source, &group.key.workspace, &m.id)
                .expect("acquired preference")
                .neutral_weight
        })
        .sum();
    let expected_budget: f64 = expected.members.iter().map(|m| m.extent).sum();
    for member in &group.members {
        let preference = preferences
            .iter_mut()
            .find(|p| {
                p.window.source == group.key.source
                    && p.window.workspace == group.key.workspace
                    && p.window.id == member.id
            })
            .expect("acquired preference");
        if Some(member.id.as_str()) == emphasized {
            let old_extent = expected
                .members
                .iter()
                .find(|m| m.id == member.id)
                .expect("same topology")
                .extent;
            // Compare normalized content shares in pixel units. An outer-envelope
            // edit that preserves the share must not teach a new focus preference.
            if (member.extent - old_extent / expected_budget * budget).abs() > f64::from(tolerance)
            {
                preference.active_share = Some(member.extent / budget);
            }
        } else {
            let weight = other_weight * member.extent / other_extent;
            if (weight - preference.neutral_weight).abs() > 1e-9 {
                preference.neutral_weight = weight;
            }
        }
        if member.extent < f64::from(member.minimum_extent) {
            // Integer solver floors must not round a manual fractional size up.
            preference.manual_floor = Some(member.extent.floor() as u32);
        }
    }
    preferences != before
}
