//! Adapter-neutral, latest-target donor-ready selection. This does not allocate:
//! the desired vector is still Taffy's answer. No transport or native scale lives here.
use super::{FeedbackAttempt, FeedbackGoal, ResponseKind};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EdgeStep {
    pub boundary: usize,
    pub travel: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EdgeSelection {
    Step(EdgeStep),
    PositionsMatch,
    Limited { reason: String },
}

/// Prefer the largest realizable prefix correction, tie by row index. A blocked
/// direction is retried only after either adjacent *model estimate* changes.
/// `attempt` must belong to this goal; old attempt evidence is not a new bound.
pub fn select_edge(
    goal: &FeedbackGoal,
    estimates: &[f64],
    attempt: Option<&FeedbackAttempt>,
) -> EdgeSelection {
    let n = goal.targets.len();
    let limited = |reason: &str| EdgeSelection::Limited {
        reason: reason.into(),
    };
    if n < 2
        || goal.inputs.members.len() != n
        || estimates.len() != n
        || estimates.iter().any(|v| !v.is_finite() || *v <= 0.0)
        || (estimates.iter().sum::<f64>() - goal.inputs.content_budget).abs() > 0.01
        || attempt.is_some_and(|a| a.goal != *goal || a.stopping.is_some())
        || goal
            .targets
            .iter()
            .zip(&goal.inputs.members)
            .any(|(t, m)| t.member != m.id)
    {
        return limited("invalid or retired donor basis");
    }
    let tolerance = f64::from(goal.geometry_tolerance);
    if estimates
        .iter()
        .zip(&goal.targets)
        .all(|(t, d)| (t - f64::from(d.extent)).abs() <= tolerance)
    {
        return EdgeSelection::PositionsMatch;
    }
    let mut residual: f64 = 0.0;
    let mut best: Option<EdgeStep> = None;
    for i in 0..n - 1 {
        residual += f64::from(goal.targets[i].extent) - estimates[i];
        if residual.abs() <= tolerance {
            continue;
        }
        let positive = residual > 0.0;
        let blocked = attempt.is_some_and(|a| {
            a.commands.iter().any(|e| {
                e.intent.boundary == i
                    && (e.intent.travel > 0.0) == positive
                    && e.response
                        .as_ref()
                        .is_some_and(|r| matches!(r.kind, ResponseKind::Blocked { .. }))
                    && e.response_estimates.is_some_and(|pair| {
                        (pair[0] - estimates[i]).abs() < 0.5
                            && (pair[1] - estimates[i + 1]).abs() < 0.5
                    })
            })
        });
        if blocked {
            continue;
        }
        let (donor, recipient) = if positive { (i + 1, i) } else { (i, i + 1) };
        let slack =
            (estimates[donor] - f64::from(goal.inputs.members[donor].minimum_extent)).max(0.0);
        let capacity = goal.inputs.members[recipient]
            .maximum_extent
            .map_or(f64::INFINITY, |max| {
                (f64::from(max) - estimates[recipient]).max(0.0)
            });
        let travel = residual.abs().min(slack).min(capacity);
        // Fractional but positive slack is useful; native integer rounding is
        // handled by the response deadline, not by assuming a client bound.
        if travel > 0.01 && best.is_none_or(|b| travel > b.travel.abs() + 0.01) {
            best = Some(EdgeStep {
                boundary: i,
                travel: travel.copysign(residual),
            });
        }
    }
    best.map_or_else(
        || limited("no donor-ready unblocked internal edge"),
        EdgeSelection::Step,
    )
}
