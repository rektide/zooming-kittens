//! Revisable stock feedback control beside the immutable [`crate::Episode`].
//!
//! One instance owns a full source census, live scoped preferences, current group
//! references and at most one bounded attempt plus one completion mailbox. It is
//! pure: native geometry/gain/attribution are adapter facts; clocks and futures
//! belong to the application. See [`Feedback::observe`] and [`Feedback::begin_command`].

mod preferences;
mod types;

pub use types::*;

use serde::Serialize;
use zk_layout::{Emphasis, Gaps, Member, Request, allocate};

use crate::{CommandOutcome, Focus, GroupKey, Target};

#[derive(Clone, Debug, Serialize)]
struct Reference {
    geometry: FeedbackGroup,
    emphasized: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct Yield {
    expected: Reference,
    reason: GuidanceReason,
}

#[derive(Clone, Debug, Serialize)]
struct GroupState {
    observed: FeedbackGroup,
    last_motion_ms: u64,
    acquired: bool,
    reference: Option<Reference>,
    yielding: Option<Yield>,
    limited: Option<GoalInputs>,
    /// Frozen prediction identity retained for delayed owned client commits,
    /// including after completion evidence has been handed to the application.
    owned: Option<CommandKey>,
    owned_reference_pending: bool,
    last_guidance: Option<Guidance>,
}

/// Preference lifetime is the full census, not a positional group identity.
/// Completed attempts must be taken/persisted; no lifetime attempt-count veto,
/// unbounded audit vector, background task or imaginary native receipt exists.
#[derive(Clone, Debug, Serialize)]
pub struct Feedback {
    policy: FeedbackPolicy,
    input: Option<FeedbackInput>,
    preferences: Vec<Preference>,
    preference_generation: u64,
    groups: Vec<GroupState>,
    current: Option<FeedbackAttempt>,
    completed: Option<FeedbackAttempt>,
    counters: FeedbackCounters,
    next_attempt: u64,
    last_now_ms: Option<u64>,
    phase: FeedbackPhase,
    restore_requested: bool,
    lost: bool,
}

impl Feedback {
    pub fn new(policy: FeedbackPolicy) -> Result<Self, FeedbackError> {
        if !policy.active_share.is_finite()
            || policy.active_share <= 0.0
            || policy.active_share >= 1.0
            || policy.convergence_timeout_ms == 0
        {
            return Err(FeedbackError::InvalidPolicy);
        }
        Ok(Self {
            policy,
            input: None,
            preferences: Vec::new(),
            preference_generation: 0,
            groups: Vec::new(),
            current: None,
            completed: None,
            counters: FeedbackCounters::default(),
            next_attempt: 1,
            last_now_ms: None,
            phase: FeedbackPhase::Unavailable,
            restore_requested: false,
            lost: false,
        })
    }

    pub fn preferences(&self) -> &[Preference] {
        &self.preferences
    }
    pub fn preference_generation(&self) -> u64 {
        self.preference_generation
    }
    pub fn current(&self) -> Option<&FeedbackAttempt> {
        self.current.as_ref()
    }
    pub fn completed(&self) -> Option<&FeedbackAttempt> {
        self.completed.as_ref()
    }
    pub fn counters(&self) -> FeedbackCounters {
        self.counters
    }
    pub fn phase(&self) -> FeedbackPhase {
        self.phase
    }
    pub fn input(&self) -> Option<&FeedbackInput> {
        self.input.as_ref()
    }
    pub fn restoration_requested(&self) -> bool {
        self.restore_requested
    }

    /// Per-group quiet deadline for geometry reacquisition in C. A past deadline
    /// does not itself prove coherence or drain a pending command. Inspect those
    /// independent facts before rebuilding a frozen native model.
    pub fn quiet_deadline_ms(&self, group: &GroupKey) -> Option<u64> {
        self.groups
            .iter()
            .find(|state| state.observed.key == *group)
            .map(|state| self.quiet_at(state))
    }

    pub fn awaiting_guidance(&self, group: &GroupKey) -> bool {
        self.groups
            .iter()
            .any(|state| state.observed.key == *group && state.yielding.is_some())
    }

    pub fn guidance(&self) -> impl Iterator<Item = &Guidance> {
        self.groups.iter().filter_map(|g| g.last_guidance.as_ref())
    }

    /// Explicit diagnostic backpressure. Take the finished record before the
    /// next decision can create an attempt; persist it in the existing bounded
    /// output path. Unfinished native receipts are never silently discarded.
    pub fn take_completed(&mut self) -> Option<FeedbackAttempt> {
        self.completed.take()
    }

    /// Hypothetical calculation only. The original preferences, attempts,
    /// counters and timer obligations remain unchanged; no native handling is
    /// invented, and no command may be admitted using the returned preview ID.
    pub fn preview(
        &self,
        input: FeedbackInput,
        now_ms: u64,
    ) -> Result<FeedbackDecision, FeedbackError> {
        self.clone().observe(input, now_ms)
    }

    /// Install a full latest semantic capture. Only content geometry changes
    /// restart quietness; focus and unrelated metadata do not. Geometry compilers
    /// must label owned progress/late commits against their frozen prediction.
    /// On unexplained motion the unsent tail is revoked immediately, even if the
    /// application must still drain its one native command future.
    pub fn observe(
        &mut self,
        input: FeedbackInput,
        now_ms: u64,
    ) -> Result<FeedbackDecision, FeedbackError> {
        self.check_clock(now_ms)?;
        validate(&input)?;
        if let Some(previous) = &self.input {
            if previous.source.id == input.source.id
                && (input.source.epoch < previous.source.epoch
                    || (input.source == previous.source && input.version < previous.version))
            {
                return Err(FeedbackError::StaleObservation);
            }
            // Loss can resume only in a genuinely new acquisition epoch.
            if self.lost && input.source == previous.source {
                return Err(FeedbackError::StaleObservation);
            }
        }
        for group in &input.groups {
            if let Motion::Owned(key) | Motion::Unattributed(key) = group.motion {
                let known = self.current.as_ref().is_some_and(|a| {
                    a.goal.inputs.group == group.key
                        && a.goal.inputs.context == group.context
                        && same_members(group, &a.goal.inputs)
                        && a.commands.iter().any(|e| e.key == key)
                }) || self.groups.iter().any(|g| {
                    g.observed.key == group.key
                        && same_topology(&g.observed, group)
                        && same_context(&g.observed, group)
                        && g.owned == Some(key)
                });
                if !known || self.input.as_ref().is_none_or(|i| i.source != input.source) {
                    return Err(FeedbackError::UnknownCommand);
                }
            }
        }
        self.last_now_ms = Some(now_ms);
        if self
            .input
            .as_ref()
            .is_some_and(|old| old.source != input.source)
        {
            self.stop(AttemptEnd::Retired(RetirementReason::SourceChanged));
            self.preferences.clear();
            self.groups.clear();
            self.preference_generation += 1;
            self.counters.continuity_losses += 1;
        }
        self.lost = false;
        if preferences::reconcile(&mut self.preferences, &input) {
            self.preference_generation += 1;
        }
        let mut previous = std::mem::take(&mut self.groups);
        for group in &input.groups {
            let old = previous
                .iter()
                .position(|g| g.observed.key == group.key)
                .map(|index| previous.remove(index));
            let state = if let Some(mut state) = old {
                let member_context_changed = self.input.as_ref().is_some_and(|old| {
                    group.members.iter().any(|member| {
                        let previous = old.live_members.iter().find(|m| m.id == member.id);
                        let current = input.live_members.iter().find(|m| m.id == member.id);
                        previous != current
                    })
                });
                if !same_topology(&state.observed, group)
                    || !same_context(&state.observed, group)
                    || member_context_changed
                {
                    GroupState::new(group.clone(), now_ms)
                } else {
                    if !same_geometry(&state.observed, group, 0) {
                        state.last_motion_ms = now_ms;
                        let own = matches!(group.motion, Motion::Owned(_));
                        if own {
                            state.owned_reference_pending = true;
                        }
                        let compatible = state.reference.as_ref().is_some_and(|r| {
                            same_geometry(&r.geometry, group, self.policy.geometry_tolerance)
                        });
                        if state.acquired
                            && !own
                            && (!compatible || matches!(group.motion, Motion::Unattributed(_)))
                        {
                            let mut reference =
                                state.reference.clone().expect("acquired reference");
                            // If an effect was submitted, its active policy is
                            // visible during ambiguous overlapping motion.
                            if let Some(attempt) = &self.current
                                && attempt.goal.inputs.group == group.key
                                && attempt.commands.iter().any(|edge| {
                                    edge.outcome
                                        .as_ref()
                                        .is_none_or(CommandOutcome::may_have_effect)
                                })
                            {
                                reference.emphasized.clone_from(&attempt.goal.inputs.active);
                            }
                            let reason = if matches!(group.motion, Motion::Unattributed(_))
                                || self.current.as_ref().is_some_and(|a| {
                                    a.goal.inputs.group == group.key && a.pending().is_some()
                                }) {
                                GuidanceReason::UnattributedOverlap
                            } else {
                                GuidanceReason::ExternalGeometry
                            };
                            if let Some(yielding) = &mut state.yielding {
                                if reason == GuidanceReason::UnattributedOverlap {
                                    yielding.reason = reason;
                                }
                            } else {
                                state.yielding = Some(Yield {
                                    expected: reference,
                                    reason,
                                });
                            }
                        }
                    }
                    state.observed = group.clone();
                    state
                }
            } else {
                GroupState::new(group.clone(), now_ms)
            };
            self.groups.push(state);
        }
        self.input = Some(input);
        self.reconsider(now_ms)
    }

    /// Timer path over a still-current capture. Deduplicated stock streams need
    /// not publish a second identical snapshot to finish acquisition or guidance.
    pub fn reconsider(&mut self, now_ms: u64) -> Result<FeedbackDecision, FeedbackError> {
        self.check_clock(now_ms)?;
        self.last_now_ms = Some(now_ms);
        if self.completed.is_some() {
            return Ok(self.wait(self.phase, WaitReason::Completion));
        }
        self.assess_current(now_ms);
        if self.current.as_ref().is_some_and(|a| a.stopping.is_some()) {
            if self.current.as_ref().is_some_and(|a| a.pending().is_some()) {
                let phase = if self.groups.iter().any(|g| g.yielding.is_some()) {
                    FeedbackPhase::Yielding
                } else {
                    FeedbackPhase::Unavailable
                };
                return Ok(self.wait(phase, WaitReason::PendingCommand));
            }
            self.finalize(now_ms);
            return Ok(self.wait(self.phase, WaitReason::Completion));
        }
        if self.lost || self.input.is_none() {
            return Ok(self.wait(FeedbackPhase::Unavailable, WaitReason::NoEligibleGroup));
        }
        if let Some(attempt) = &self.current {
            let group = self
                .groups
                .iter()
                .find(|g| g.observed.key == attempt.goal.inputs.group)
                .expect("assessed current scope");
            if attempt.pending().is_some() {
                return Ok(self.wait(FeedbackPhase::Driving, WaitReason::PendingCommand));
            }
            if converged(
                &group.observed,
                &attempt.goal.targets,
                self.policy.geometry_tolerance,
            ) {
                if now_ms < self.quiet_at(group) {
                    return Ok(self.wait(FeedbackPhase::Driving, WaitReason::Quiet));
                }
                self.stop(AttemptEnd::Settled);
                self.finalize(now_ms);
                return Ok(self.wait(FeedbackPhase::Ready, WaitReason::Completion));
            }
            if let Some(edge) = attempt.commands.last() {
                let failure = match &edge.outcome {
                    Some(CommandOutcome::Unsent) => Some(LimitReason::Unsent),
                    Some(CommandOutcome::Rejected { message }) => Some(LimitReason::Rejected {
                        message: message.clone(),
                    }),
                    Some(CommandOutcome::PossiblySent { message }) => {
                        Some(LimitReason::PossiblySent {
                            message: message.clone(),
                        })
                    }
                    _ => None,
                };
                if let Some(reason) = failure {
                    self.stop(AttemptEnd::Limited(reason));
                    self.finalize(now_ms);
                    return Ok(self.wait(FeedbackPhase::Limited, WaitReason::Completion));
                }
                if edge.response.is_none() {
                    return Ok(self.wait(FeedbackPhase::Driving, WaitReason::PositionResponse));
                }
            }
            if positions_match(
                &group.observed,
                &attempt.goal.targets,
                self.policy.geometry_tolerance,
            ) {
                return Ok(self.wait(FeedbackPhase::Driving, WaitReason::ClientCommit));
            }
            if attempt.commands.len() >= attempt.goal.submission_limit {
                self.stop(AttemptEnd::Limited(LimitReason::SubmissionBudget));
                self.finalize(now_ms);
                return Ok(self.wait(FeedbackPhase::Limited, WaitReason::Completion));
            }
            self.phase = FeedbackPhase::Driving;
            return Ok(FeedbackDecision::Drive(Box::new(attempt.goal.clone())));
        }
        self.acquire_and_learn(now_ms);
        self.clear_changed_limitations();
        let input = self.input.as_ref().expect("current input");
        if input.focus == Focus::Unknown && !self.restore_requested {
            return Ok(self.wait(FeedbackPhase::Unavailable, WaitReason::FocusUnknown));
        }
        // Restore previous emphasis before driving a newly focused group.
        let mut order: Vec<usize> = (0..self.groups.len()).collect();
        order.sort_by_key(|&i| {
            let g = &self.groups[i];
            !(g.reference.as_ref().is_some_and(|r| r.emphasized.is_some())
                && active(input, &g.observed, self.restore_requested).is_none())
        });
        let mut waiting = None;
        for index in order {
            let state = &self.groups[index];
            if !eligible(&state.observed) {
                continue;
            }
            if state.yielding.is_some() {
                waiting = Some((FeedbackPhase::Yielding, WaitReason::Quiet));
                continue;
            }
            if !state.acquired || now_ms < self.quiet_at(state) {
                waiting.get_or_insert((
                    FeedbackPhase::Acquiring,
                    if state.observed.coherent {
                        WaitReason::Quiet
                    } else {
                        WaitReason::CoherentGeometry
                    },
                ));
                continue;
            }
            let inputs = self.goal_inputs(index);
            if state.limited.as_ref() == Some(&inputs) {
                waiting.get_or_insert((FeedbackPhase::Limited, WaitReason::Limited));
                continue;
            }
            let targets = match solve(&inputs, self.policy.active_share) {
                Ok(targets) => targets,
                Err(error) => {
                    self.phase = FeedbackPhase::Limited;
                    return Ok(FeedbackDecision::Infeasible {
                        group: inputs.group,
                        error,
                    });
                }
            };
            if converged(&state.observed, &targets, self.policy.geometry_tolerance) {
                self.groups[index].reference = Some(Reference {
                    geometry: state.observed.clone(),
                    emphasized: inputs.active,
                });
                continue;
            }
            // An uncalibrated projection can acquire preferences, but it cannot
            // drive without target estimates and a coherent starting geometry.
            if !state.observed.coherent || state.observed.target_extents.is_none() {
                waiting.get_or_insert((FeedbackPhase::Acquiring, WaitReason::CoherentGeometry));
                continue;
            }
            let goal = FeedbackGoal {
                id: self.next_attempt,
                submission_limit: 8 * (inputs.members.len() - 1) + 2,
                inputs,
                preference_generation: self.preference_generation,
                basis_version: input.version,
                targets,
                created_at_ms: now_ms,
                deadline_ms: now_ms.saturating_add(self.policy.convergence_timeout_ms),
                geometry_tolerance: self.policy.geometry_tolerance,
            };
            self.next_attempt += 1;
            self.counters.attempts += 1;
            self.current = Some(FeedbackAttempt {
                goal: goal.clone(),
                commands: Vec::new(),
                stopping: None,
                result: None,
            });
            self.phase = FeedbackPhase::Driving;
            return Ok(FeedbackDecision::Drive(Box::new(goal)));
        }
        if let Some((phase, reason)) = waiting {
            return Ok(self.wait(phase, reason));
        }
        let any = self.groups.iter().any(|g| eligible(&g.observed));
        Ok(self.wait(
            if any {
                FeedbackPhase::Ready
            } else {
                FeedbackPhase::Unavailable
            },
            if any {
                WaitReason::Stable
            } else {
                WaitReason::NoEligibleGroup
            },
        ))
    }

    /// Admit one real native intent BEFORE awaiting its future. Reconsider first
    /// after every observation/reply/response. The adapter still must validate its
    /// native basis and capability; this pure gate is not remote compare-and-swap.
    pub fn begin_command(
        &mut self,
        attempt_id: u64,
        intent: EdgeIntent,
        now_ms: u64,
    ) -> Result<CommandKey, FeedbackError> {
        self.check_clock(now_ms)?;
        if self.completed.is_some() || self.phase != FeedbackPhase::Driving {
            return Err(FeedbackError::NotDriving);
        }
        let Some(attempt) = &self.current else {
            return Err(FeedbackError::NotDriving);
        };
        if attempt.goal.id != attempt_id
            || attempt.stopping.is_some()
            || attempt.pending().is_some()
            || attempt.commands.last().is_some_and(|e| {
                e.response.is_none() || !matches!(e.outcome, Some(CommandOutcome::Handled))
            })
            || now_ms >= attempt.goal.deadline_ms
            || attempt.commands.len() >= attempt.goal.submission_limit
        {
            return Err(FeedbackError::NotDriving);
        }
        if self
            .input
            .as_ref()
            .is_none_or(|i| i.version != intent.basis_version)
        {
            return Err(FeedbackError::StaleBasis);
        }
        if intent.boundary >= attempt.goal.inputs.members.len().saturating_sub(1)
            || !intent.travel.is_finite()
            || intent.travel == 0.0
        {
            return Err(FeedbackError::InvalidEdge);
        }
        let group = self
            .groups
            .iter()
            .find(|g| g.observed.key == attempt.goal.inputs.group)
            .expect("current group");
        if group.yielding.is_some()
            || positions_match(
                &group.observed,
                &attempt.goal.targets,
                self.policy.geometry_tolerance,
            )
        {
            return Err(FeedbackError::NotDriving);
        }
        self.last_now_ms = Some(now_ms);
        let attempt = self.current.as_mut().expect("current attempt");
        let key = CommandKey {
            attempt: attempt_id,
            sequence: attempt.commands.len() as u64 + 1,
        };
        attempt.commands.push(EdgeEvidence {
            key,
            intent,
            created_at_ms: now_ms,
            outcome: None,
            response: None,
            response_estimates: None,
        });
        self.counters.submissions += 1;
        Ok(key)
    }

    /// A stopped/lost attempt still accepts its outstanding receipt. A later
    /// command is never admitted until this future is drained and evidence taken.
    pub fn record_outcome(
        &mut self,
        key: CommandKey,
        outcome: CommandOutcome,
    ) -> Result<(), FeedbackError> {
        let edge = self.edge_mut(key)?;
        if edge.outcome.as_ref().is_some_and(|old| *old != outcome) {
            return Err(FeedbackError::ContradictoryEvidence);
        }
        edge.outcome = Some(outcome);
        Ok(())
    }

    /// Record adapter-assessed position response separately from native handling.
    /// A blocked result is scoped to this command's adjacent geometry, not a
    /// universal client bound. C may choose another donor-ready edge afterwards.
    pub fn record_response(
        &mut self,
        key: CommandKey,
        response: EdgeResponse,
    ) -> Result<(), FeedbackError> {
        let version = self.input.as_ref().map(|i| i.version);
        let estimates = self
            .current
            .as_ref()
            .filter(|a| a.goal.id == key.attempt)
            .and_then(|attempt| {
                let edge = attempt.commands.iter().find(|e| e.key == key)?;
                let group = self.groups.iter().find(|g| {
                    g.observed.key == attempt.goal.inputs.group
                        && same_members(&g.observed, &attempt.goal.inputs)
                })?;
                let extents = group.observed.target_extents.as_ref()?;
                Some([
                    extents[edge.intent.boundary],
                    extents[edge.intent.boundary + 1],
                ])
            });
        let edge = self.edge_mut(key)?;
        if version != Some(response.observed_version)
            || response.observed_version < edge.intent.basis_version
            || (matches!(response.kind, ResponseKind::Progress)
                && response.observed_version == edge.intent.basis_version)
        {
            return Err(FeedbackError::StaleBasis);
        }
        if edge.response.as_ref().is_some_and(|old| *old != response) {
            return Err(FeedbackError::ContradictoryEvidence);
        }
        edge.response = Some(response);
        edge.response_estimates = estimates;
        Ok(())
    }

    /// No donor-ready route / native geometry limitation. Stop without promoting
    /// failed achieved geometry to a neutral weight or active-share preference.
    pub fn limit_current(
        &mut self,
        attempt_id: u64,
        reason: LimitReason,
    ) -> Result<(), FeedbackError> {
        if self
            .current
            .as_ref()
            .is_none_or(|a| a.goal.id != attempt_id)
        {
            return Err(FeedbackError::NotDriving);
        }
        self.stop(AttemptEnd::Limited(reason));
        Ok(())
    }

    /// Latches neutral intent. Active user motion still wins over restoration.
    /// The application owns the bounded shutdown deadline and may leave achieved
    /// geometry in place instead of forcing a write against a drag or stale facts.
    pub fn request_restore(&mut self, now_ms: u64) -> Result<FeedbackDecision, FeedbackError> {
        self.check_clock(now_ms)?;
        if !self.restore_requested {
            self.restore_requested = true;
            self.stop(AttemptEnd::Retired(RetirementReason::RestoreRequested));
        }
        self.reconsider(now_ms)
    }

    /// Revoke unsent work now; never replay a possibly-sent edge. Recovery needs
    /// a fresh source epoch, and cannot bypass an outstanding receipt/mailbox.
    pub fn lose(
        &mut self,
        reason: impl Into<String>,
        now_ms: u64,
    ) -> Result<FeedbackDecision, FeedbackError> {
        self.check_clock(now_ms)?;
        self.lost = true;
        self.stop(AttemptEnd::Retired(RetirementReason::SourceLost {
            reason: reason.into(),
        }));
        self.reconsider(now_ms)
    }

    /// Schedule one real timer, even on a silent stream. Command-position query
    /// deadlines/gain/clamp handling are additional C/D responsibilities.
    pub fn next_reconsideration(&self) -> Option<u64> {
        if self.completed.is_some() || self.lost {
            return None;
        }
        let now = self.last_now_ms.unwrap_or(0);
        let goal_deadline = self
            .current
            .as_ref()
            .filter(|a| a.stopping.is_none())
            .map(|a| a.goal.deadline_ms);
        let quiet = self
            .groups
            .iter()
            .filter(|g| {
                eligible(&g.observed)
                    && (g.yielding.is_some()
                        || !g.acquired
                        || self
                            .current
                            .as_ref()
                            .is_some_and(|a| a.goal.inputs.group == g.observed.key))
            })
            .map(|g| self.quiet_at(g))
            .filter(|&at| at > now)
            .min();
        goal_deadline.into_iter().chain(quiet).min()
    }

    fn check_clock(&self, now_ms: u64) -> Result<(), FeedbackError> {
        if self.last_now_ms.is_some_and(|last| now_ms < last) {
            Err(FeedbackError::ClockRegressed)
        } else {
            Ok(())
        }
    }

    fn wait(&mut self, phase: FeedbackPhase, reason: WaitReason) -> FeedbackDecision {
        self.phase = phase;
        FeedbackDecision::Wait { phase, reason }
    }

    fn quiet_at(&self, group: &GroupState) -> u64 {
        group
            .last_motion_ms
            .saturating_add(self.policy.resize_debounce_ms)
    }

    fn stop(&mut self, reason: AttemptEnd) {
        if let Some(attempt) = &mut self.current
            && attempt.stopping.is_none()
        {
            attempt.stopping = Some(reason);
        }
    }

    fn edge_mut(&mut self, key: CommandKey) -> Result<&mut EdgeEvidence, FeedbackError> {
        self.current
            .as_mut()
            .filter(|a| a.goal.id == key.attempt)
            .and_then(|a| a.commands.iter_mut().find(|e| e.key == key))
            .ok_or(FeedbackError::UnknownCommand)
    }

    fn goal_inputs(&self, index: usize) -> GoalInputs {
        let input = self.input.as_ref().expect("current input");
        let group = &self.groups[index].observed;
        GoalInputs {
            group: group.key.clone(),
            context: group.context.clone(),
            content_budget: group.content_budget,
            active: active(input, group, self.restore_requested),
            members: group
                .members
                .iter()
                .map(|m| {
                    let preference = preferences::find(
                        &self.preferences,
                        &input.source,
                        &group.key.workspace,
                        &m.id,
                    )
                    .expect("acquired preference");
                    GoalMember {
                        id: m.id.clone(),
                        neutral_weight: preference.neutral_weight,
                        learned_share: preference.active_share,
                        maximum_extent: m.maximum_extent,
                        minimum_extent: preference
                            .manual_floor
                            .map_or(m.minimum_extent, |floor| floor.min(m.minimum_extent)),
                    }
                })
                .collect(),
        }
    }

    fn assess_current(&mut self, now_ms: u64) {
        let Some(attempt) = &self.current else {
            return;
        };
        if attempt.stopping.is_some() {
            return;
        }
        let Some(index) = self
            .groups
            .iter()
            .position(|g| g.observed.key == attempt.goal.inputs.group)
        else {
            self.stop(AttemptEnd::Retired(RetirementReason::TopologyChanged));
            return;
        };
        let state = &self.groups[index];
        let reason = if !eligible(&state.observed) || state.observed.target_extents.is_none() {
            Some(RetirementReason::Unavailable)
        } else if !same_members(&state.observed, &attempt.goal.inputs) {
            Some(RetirementReason::TopologyChanged)
        } else if !state.acquired || state.observed.context != attempt.goal.inputs.context {
            Some(RetirementReason::ContextChanged)
        } else if state.yielding.is_some() {
            Some(RetirementReason::Yielded)
        } else if self.input.as_ref().is_some_and(|input| {
            input.focus == Focus::Unknown && !self.restore_requested
                || active(input, &state.observed, self.restore_requested)
                    != attempt.goal.inputs.active
        }) {
            Some(RetirementReason::FocusChanged)
        } else if self.goal_inputs(index) != attempt.goal.inputs {
            Some(RetirementReason::ContextChanged)
        } else {
            None
        };
        if let Some(reason) = reason {
            self.stop(AttemptEnd::Retired(reason));
        } else if now_ms >= attempt.goal.deadline_ms {
            self.stop(AttemptEnd::Limited(LimitReason::Deadline));
        }
    }

    fn acquire_and_learn(&mut self, now_ms: u64) {
        let input = self.input.as_ref().expect("current input");
        for state in &mut self.groups {
            let group = &state.observed;
            if !eligible(group)
                || !group.coherent
                || group.members.iter().any(|m| m.extent <= 0.0)
                || now_ms
                    < state
                        .last_motion_ms
                        .saturating_add(self.policy.resize_debounce_ms)
            {
                continue;
            }
            if !state.acquired {
                if preferences::acquire(&mut self.preferences, input, group) {
                    self.preference_generation += 1;
                }
                state.acquired = true;
                state.reference = Some(Reference {
                    geometry: group.clone(),
                    emphasized: None,
                });
                self.counters.acquisitions += 1;
            } else if let Some(yielding) = state.yielding.take() {
                let changed = preferences::learn(
                    &mut self.preferences,
                    group,
                    yielding.expected.emphasized.as_deref(),
                    &yielding.expected.geometry,
                    self.policy.geometry_tolerance,
                );
                if changed {
                    self.preference_generation += 1;
                }
                state.last_guidance = Some(Guidance {
                    group: group.key.clone(),
                    reason: yielding.reason,
                    emphasized: yielding.expected.emphasized.clone(),
                    observed_version: input.version,
                    preference_generation: self.preference_generation,
                    changed,
                });
                state.reference = Some(Reference {
                    geometry: group.clone(),
                    emphasized: yielding.expected.emphasized,
                });
                state.limited = None;
                state.owned_reference_pending = false;
                self.counters.guidance += 1;
            } else if state.owned_reference_pending {
                // Final/limited/retired commands may have slow client commits.
                // Refresh expected geometry only after coherent quiet settlement;
                // retain the preference parameters and actual emphasized member.
                if let Some(reference) = &mut state.reference {
                    reference.geometry = group.clone();
                }
                state.owned_reference_pending = false;
            }
        }
    }

    fn clear_changed_limitations(&mut self) {
        for index in 0..self.groups.len() {
            if self.groups[index].acquired
                && self.groups[index]
                    .limited
                    .as_ref()
                    .is_some_and(|limited| *limited != self.goal_inputs(index))
            {
                // A → B → A is three focus generations, not revival of the old
                // A limitation merely because the input vector happens to recur.
                self.groups[index].limited = None;
            }
        }
    }

    fn finalize(&mut self, now_ms: u64) {
        let mut attempt = self.current.take().expect("stopping attempt");
        let end = attempt.stopping.clone().expect("classified completion");
        let state = self.groups.iter_mut().find(|g| {
            g.observed.key == attempt.goal.inputs.group
                && same_members(&g.observed, &attempt.goal.inputs)
                && g.observed.context == attempt.goal.inputs.context
                && g.acquired
        });
        let achieved = state.as_ref().map(|g| g.observed.clone());
        if let Some(state) = state {
            state.owned = attempt.commands.last().map(|e| e.key);
            if state.yielding.is_none() {
                let affected = matches!(end, AttemptEnd::Settled)
                    || attempt.commands.iter().any(|e| {
                        e.outcome
                            .as_ref()
                            .is_some_and(CommandOutcome::may_have_effect)
                    });
                let emphasized = if affected {
                    attempt.goal.inputs.active.clone()
                } else {
                    state.reference.as_ref().and_then(|r| r.emphasized.clone())
                };
                state.reference = Some(Reference {
                    geometry: state.observed.clone(),
                    emphasized,
                });
            }
            if matches!(end, AttemptEnd::Limited(_)) {
                state.limited = Some(attempt.goal.inputs.clone());
            }
        }
        self.phase = match end {
            AttemptEnd::Settled => {
                self.counters.settled += 1;
                FeedbackPhase::Ready
            }
            AttemptEnd::Limited(_) => {
                self.counters.limited += 1;
                FeedbackPhase::Limited
            }
            AttemptEnd::Retired(_) => {
                self.counters.retired += 1;
                if self.groups.iter().any(|g| g.yielding.is_some()) {
                    FeedbackPhase::Yielding
                } else {
                    FeedbackPhase::Acquiring
                }
            }
        };
        attempt.result = Some(AttemptResult {
            end,
            achieved,
            blocked_edges: attempt
                .commands
                .iter()
                .filter_map(|edge| {
                    let response = edge.response.as_ref()?;
                    let ResponseKind::Blocked { reason } = &response.kind else {
                        return None;
                    };
                    Some(BlockedEdge {
                        command: edge.key,
                        boundary: edge.intent.boundary,
                        positive_travel: edge.intent.travel > 0.0,
                        adjacent_estimates: edge.response_estimates,
                        observed_version: response.observed_version,
                        reason: reason.clone(),
                    })
                })
                .collect(),
            finished_at_ms: now_ms,
        });
        self.completed = Some(attempt);
    }
}

impl GroupState {
    fn new(observed: FeedbackGroup, now_ms: u64) -> Self {
        Self {
            observed,
            last_motion_ms: now_ms,
            acquired: false,
            reference: None,
            yielding: None,
            limited: None,
            owned: None,
            owned_reference_pending: false,
            last_guidance: None,
        }
    }
}

fn eligible(group: &FeedbackGroup) -> bool {
    group.unavailable.is_none() && group.members.len() >= 2
}

fn active(input: &FeedbackInput, group: &FeedbackGroup, restore: bool) -> Option<String> {
    if restore {
        return None;
    }
    let Focus::Window(window) = &input.focus else {
        return None;
    };
    (window.source == input.source
        && window.workspace == group.key.workspace
        && group.members.iter().any(|m| m.id == window.id))
    .then(|| window.id.clone())
}

fn same_topology(a: &FeedbackGroup, b: &FeedbackGroup) -> bool {
    a.members.len() == b.members.len()
        && a.members.iter().zip(&b.members).all(|(a, b)| a.id == b.id)
}

fn same_context(a: &FeedbackGroup, b: &FeedbackGroup) -> bool {
    a.context == b.context
        && a.unavailable == b.unavailable
        && a.members.iter().zip(&b.members).all(|(a, b)| {
            a.minimum_extent == b.minimum_extent && a.maximum_extent == b.maximum_extent
        })
}

fn same_members(group: &FeedbackGroup, inputs: &GoalInputs) -> bool {
    group.members.len() == inputs.members.len()
        && group
            .members
            .iter()
            .zip(&inputs.members)
            .all(|(a, b)| a.id == b.id)
}

fn same_geometry(a: &FeedbackGroup, b: &FeedbackGroup, tolerance: u32) -> bool {
    same_topology(a, b)
        && a.members.iter().zip(&b.members).all(|(a, b)| {
            (a.start - b.start).abs() <= f64::from(tolerance)
                && (a.extent - b.extent).abs() <= f64::from(tolerance)
        })
}

fn positions_match(group: &FeedbackGroup, targets: &[Target], tolerance: u32) -> bool {
    group.target_extents.as_ref().is_some_and(|extents| {
        extents.len() == targets.len()
            && extents.iter().zip(targets).all(|(extent, target)| {
                (extent - f64::from(target.extent)).abs() <= f64::from(tolerance)
            })
    })
}

fn converged(group: &FeedbackGroup, targets: &[Target], tolerance: u32) -> bool {
    group.coherent
        && positions_match(group, targets, tolerance)
        && group.members.iter().zip(targets).all(|(m, t)| {
            m.id == t.member && (m.extent - f64::from(t.extent)).abs() <= f64::from(tolerance)
        })
}

fn solve(inputs: &GoalInputs, default_share: f64) -> Result<Vec<Target>, zk_layout::Error> {
    let members: Vec<_> = inputs
        .members
        .iter()
        .map(|m| Member {
            weight: m.neutral_weight,
            minimum: m.minimum_extent,
            maximum: m.maximum_extent,
        })
        .collect();
    let emphasis = inputs
        .active
        .as_ref()
        .and_then(|id| inputs.members.iter().position(|m| m.id == *id))
        .map(|index| Emphasis {
            index,
            share: inputs.members[index].learned_share.unwrap_or(default_share),
        });
    let allocation = allocate(Request {
        extent: inputs.content_budget,
        gaps: Gaps::AlreadyExcluded,
        members: &members,
        emphasis,
    })?;
    Ok(inputs
        .members
        .iter()
        .zip(allocation.extents)
        .map(|(member, extent)| Target {
            member: member.id.clone(),
            extent,
        })
        .collect())
}

fn validate(input: &FeedbackInput) -> Result<(), FeedbackError> {
    let invalid = |reason: &str| FeedbackError::InvalidInput {
        reason: reason.into(),
    };
    if input.source.id.is_empty() {
        return Err(invalid("empty source"));
    }
    let mut live = std::collections::BTreeSet::new();
    for member in &input.live_members {
        if member.id.is_empty()
            || member.workspace.is_empty()
            || member.context.is_empty()
            || !live.insert(member.id.as_str())
        {
            return Err(invalid("invalid or duplicate live identity"));
        }
    }
    let mut assigned = std::collections::BTreeSet::new();
    for (i, group) in input.groups.iter().enumerate() {
        if group.key.source != input.source
            || group.key.workspace.is_empty()
            || group.key.topology.is_empty()
            || group.context.is_empty()
            || input.groups[..i].iter().any(|g| g.key == group.key)
        {
            return Err(invalid("invalid or duplicate group identity"));
        }
        if !group.content_budget.is_finite()
            || group.content_budget <= 0.0
            || group.content_budget > f64::from(zk_layout::MAX_EXTENT)
        {
            return Err(invalid("invalid content budget"));
        }
        if let Some(targets) = &group.target_extents
            && (targets.len() != group.members.len()
                || targets.iter().any(|extent| {
                    !extent.is_finite()
                        || *extent < 0.0
                        || *extent > f64::from(zk_layout::MAX_EXTENT)
                }))
        {
            return Err(invalid("invalid target estimates"));
        }
        for member in &group.members {
            if !assigned.insert(member.id.as_str())
                || !input
                    .live_members
                    .iter()
                    .any(|live| live.id == member.id && live.workspace == group.key.workspace)
            {
                return Err(invalid(
                    "group member missing, duplicated or in another workspace",
                ));
            }
            if !member.start.is_finite()
                || !member.extent.is_finite()
                || member.extent < 0.0
                || member.extent > f64::from(zk_layout::MAX_EXTENT)
                || member
                    .maximum_extent
                    .is_some_and(|max| max < member.minimum_extent)
            {
                return Err(invalid("invalid content geometry or bounds"));
            }
        }
    }
    if let Focus::Window(window) = &input.focus
        && (window.id.is_empty() || window.workspace.is_empty() || window.source.id.is_empty())
    {
        return Err(invalid("invalid focus identity"));
    }
    Ok(())
}
