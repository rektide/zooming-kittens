//! Pure Longcat episode decisions; native observation and execution stay outside.
//!
//! [`Episode::preview`] is non-mutating dry-run planning. [`Episode::decide`]
//! records intent, never fabricated execution. A runner records actual command
//! evidence, continues observing, and schedules [`Episode::next_reconsideration`]
//! even if no further native event arrives.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};
pub use zk_layout::{Allocation, Gaps};
use zk_layout::{Emphasis, Member, Request, allocate};

/// Caller-assigned source identity and non-resumable acquisition epoch.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub epoch: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GroupKey {
    pub source: Source,
    pub workspace: String,
    /// Adapter-owned topology token, not a promise that a column ordinal is a
    /// persistent identity. Ordered member identity is checked independently.
    pub topology: String,
    pub axis: Axis,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowKey {
    pub source: Source,
    pub workspace: String,
    pub id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Focus {
    /// Acquisition has not established seat focus; do not treat this as blur.
    Unknown,
    /// Explicitly observed no active window (for example a layer surface).
    None,
    Window(WindowKey),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObservedMember {
    pub id: String,
    /// Observed committed secondary-axis extent in the adapter's declared
    /// content/tile coordinate space. Never substitute our last request here.
    pub extent: f64,
    pub minimum: u32,
    pub maximum: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub group: GroupKey,
    /// Monotonic local capture order, not a compositor revision or resume token.
    pub version: u64,
    pub available_extent: f64,
    pub gaps: Gaps,
    /// Secondary-axis order; changing order invalidates this episode.
    pub members: Vec<ObservedMember>,
    pub focus: Focus,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Policy {
    pub active_share: f64,
    pub geometry_tolerance: u32,
    /// Clock inputs are caller-owned monotonic milliseconds, never wall time.
    pub convergence_timeout_ms: u64,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            active_share: 0.7,
            geometry_tolerance: 2,
            convergence_timeout_ms: 1_000,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Baseline {
    pub observation: Observation,
    /// Conserving, bounded integer restoration representation. This is not a
    /// native restore token and makes no claim to restore hidden split weights.
    pub allocation: Allocation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanKind {
    Emphasize,
    Restore,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    pub member: String,
    pub extent: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    /// Episode-local sequence number; pair with the episode/group identity.
    pub id: u64,
    pub kind: PlanKind,
    pub basis: Observation,
    pub active: Option<WindowKey>,
    pub targets: Vec<Target>,
    pub created_at_ms: u64,
    pub deadline_ms: u64,
    pub geometry_tolerance: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanRejection {
    StaleObservation,
    IdentityChanged,
    LayoutContextChanged,
    FocusChanged,
    GeometryChanged,
}

impl Plan {
    /// Local admission check before sending (and again between sequential
    /// commands). This cannot close a server-side focus/topology race.
    ///
    /// After any command may have been sent, geometry can legitimately differ
    /// from `basis`; the runner should validate identity/focus between commands,
    /// not demand the original geometry for the remainder of that same batch.
    pub fn validate(&self, current: &Observation) -> Result<(), PlanRejection> {
        self.validate_identity(current)?;
        if self
            .basis
            .members
            .iter()
            .zip(&current.members)
            .any(|(a, b)| {
                !b.extent.is_finite()
                    || (a.extent - b.extent).abs() > f64::from(self.geometry_tolerance)
            })
        {
            return Err(PlanRejection::GeometryChanged);
        }
        Ok(())
    }

    pub fn validate_identity(&self, current: &Observation) -> Result<(), PlanRejection> {
        if current.version < self.basis.version {
            return Err(PlanRejection::StaleObservation);
        }
        if !same_identity(&self.basis, current) {
            return Err(PlanRejection::IdentityChanged);
        }
        if !same_context(&self.basis, current) {
            return Err(PlanRejection::LayoutContextChanged);
        }
        if current.focus != self.basis.focus {
            return Err(PlanRejection::FocusChanged);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandOutcome {
    /// Definitely not submitted, including the unexecuted initial intent.
    Unsent,
    /// Native reply accepted handling; it does not prove visible convergence.
    Handled,
    Rejected {
        message: String,
    },
    /// Transport failed after submission may have occurred. Never blind-retry.
    PossiblySent {
        message: String,
    },
}

impl CommandOutcome {
    pub fn may_have_effect(&self) -> bool {
        matches!(self, Self::Handled | Self::PossiblySent { .. })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Convergence {
    Waiting,
    Compatible { observed_version: u64 },
    NoEffect,
    TimedOut,
    Abandoned,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Effect {
    pub plan: Plan,
    /// Exactly one entry per ordered plan target, retained through supersession,
    /// conflicts and loss. A native sequence is not a transaction.
    pub commands: Vec<CommandOutcome>,
    pub execution_finished: bool,
    pub convergence: Convergence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Ready,
    Applying,
    Settled,
    Restoring,
    Restored,
    Conflicted,
    Lost,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Suspension {
    FocusUnknown,
    StaleObservation,
    GroupIdentityChanged,
    LayoutContextChanged,
    SourceLost { reason: String },
    InvalidObservation(InputError),
    ClockRegressed,
    UnattributedGeometry,
    CommandRejected,
    ExecutionTimedOut,
    ConvergenceTimedOut,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Noop {
    FocusOutside,
    AlreadyCompatible,
    AwaitingExecution,
    AwaitingConvergence,
    Stable,
    Restored,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Decision {
    Plan(Box<Plan>),
    Noop(Noop),
    Suspend(Suspension),
    Infeasible(zk_layout::Error),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputError {
    EmptyIdentity,
    EmptyMembers,
    DuplicateMember { id: String },
    InvalidGeometry { id: String },
    InvalidPolicy,
    Allocation(zk_layout::Error),
}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid Longcat input: {self:?}")
    }
}

impl std::error::Error for InputError {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceError {
    UnknownPlan,
    InvalidCommandIndex,
    InvalidCommandCount,
    ContradictoryEvidence,
}

/// One topology-bound, volatile control episode. The runner owns a collection
/// only if it needs several groups. Reacquisition is explicit: construct a new
/// episode after loss/conflict; do not transfer the old baseline to new members.
#[derive(Clone, Debug, Serialize)]
pub struct Episode {
    baseline: Baseline,
    observed: Observation,
    policy: Policy,
    phase: Phase,
    suspension: Option<Suspension>,
    effects: Vec<Effect>,
    current_effect: Option<usize>,
    last_now_ms: Option<u64>,
    restoration_requested: bool,
}

impl Episode {
    pub fn new(observation: Observation, policy: Policy) -> Result<Self, InputError> {
        if !policy.active_share.is_finite()
            || policy.active_share <= 0.0
            || policy.active_share >= 1.0
            || policy.convergence_timeout_ms == 0
        {
            return Err(InputError::InvalidPolicy);
        }
        validate_observation(&observation)?;
        let allocation = allocation(&observation, None).map_err(InputError::Allocation)?;
        Ok(Self {
            baseline: Baseline {
                observation: observation.clone(),
                allocation,
            },
            observed: observation,
            policy,
            phase: Phase::Ready,
            suspension: None,
            effects: Vec::new(),
            current_effect: None,
            last_now_ms: None,
            restoration_requested: false,
        })
    }

    pub fn baseline(&self) -> &Baseline {
        &self.baseline
    }

    pub fn observed(&self) -> &Observation {
        &self.observed
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn suspension(&self) -> Option<&Suspension> {
        self.suspension.as_ref()
    }

    pub fn effects(&self) -> &[Effect] {
        &self.effects
    }

    pub fn requested(&self) -> Option<&Effect> {
        self.current_effect
            .and_then(|index| self.effects.get(index))
    }

    /// Schedule this deadline with the runner's real timer, including after the
    /// final native event. No polling or background task is created here.
    pub fn next_reconsideration(&self) -> Option<u64> {
        if matches!(self.phase, Phase::Applying | Phase::Restoring) {
            self.requested().map(|effect| effect.plan.deadline_ms)
        } else {
            None
        }
    }

    /// Dry-run: calculate from the retained baseline without recording intent,
    /// execution, an imaginary observed effect, or a convergence deadline.
    pub fn preview(&self, observation: Observation, now_ms: u64) -> Decision {
        self.clone().decide(observation, now_ms)
    }

    /// Record a real requested intent. Native sending is the caller's job.
    pub fn decide(&mut self, observation: Observation, now_ms: u64) -> Decision {
        if matches!(self.phase, Phase::Lost | Phase::Conflicted) {
            return Decision::Suspend(self.suspension.clone().expect("terminal reason"));
        }
        if let Err(error) = validate_observation(&observation) {
            return self.conflict(Suspension::InvalidObservation(error));
        }
        if self.last_now_ms.is_some_and(|previous| now_ms < previous) {
            return self.conflict(Suspension::ClockRegressed);
        }
        self.last_now_ms = Some(now_ms);
        if observation.version < self.observed.version {
            return Decision::Suspend(Suspension::StaleObservation);
        }
        if !same_identity(&self.baseline.observation, &observation) {
            self.observed = observation;
            return self.invalidate(Suspension::GroupIdentityChanged);
        }
        if !same_context(&self.baseline.observation, &observation) {
            self.observed = observation;
            return self.invalidate(Suspension::LayoutContextChanged);
        }
        self.observed = observation;

        if self.requested().is_some_and(|effect| {
            effect
                .commands
                .iter()
                .any(|outcome| matches!(outcome, CommandOutcome::Rejected { .. }))
        }) {
            return self.conflict(Suspension::CommandRejected);
        }
        if matches!(self.phase, Phase::Applying | Phase::Restoring) {
            let index = self.current_effect.expect("pending effect index");
            let effect = &mut self.effects[index];
            if !effect.execution_finished {
                if now_ms >= effect.plan.deadline_ms {
                    effect.convergence = Convergence::TimedOut;
                    return self.conflict(Suspension::ExecutionTimedOut);
                }
                return Decision::Noop(Noop::AwaitingExecution);
            }
            if !effect.commands.iter().any(CommandOutcome::may_have_effect) {
                effect.convergence = Convergence::NoEffect;
                // A refused, definitely-unsent replacement does not erase the
                // prior settled effect or the duty to conditionally restore it.
                self.current_effect = self.effects[..index].iter().rposition(|effect| {
                    matches!(effect.convergence, Convergence::Compatible { .. })
                });
                self.phase = match self.requested().map(|effect| effect.plan.kind) {
                    Some(PlanKind::Emphasize) => Phase::Settled,
                    Some(PlanKind::Restore) => Phase::Restored,
                    None => Phase::Ready,
                };
            } else if compatible(
                &self.observed,
                &effect.plan.targets,
                self.policy.geometry_tolerance,
            ) {
                effect.convergence = Convergence::Compatible {
                    observed_version: self.observed.version,
                };
                self.phase = match effect.plan.kind {
                    PlanKind::Emphasize => Phase::Settled,
                    PlanKind::Restore => Phase::Restored,
                };
            } else {
                if now_ms >= effect.plan.deadline_ms {
                    effect.convergence = Convergence::TimedOut;
                    return self.conflict(Suspension::ConvergenceTimedOut);
                }
                return Decision::Noop(Noop::AwaitingConvergence);
            }
        }

        // Self-induced geometry is compared with the last requested effect,
        // never mistaken for a change to the immutable baseline.
        let expected = match self.phase {
            Phase::Settled | Phase::Restored => self
                .requested()
                .map(|effect| effect.plan.targets.clone())
                .expect("settled effect"),
            _ => targets(&self.baseline.observation, &self.baseline.allocation),
        };
        if !compatible(&self.observed, &expected, self.policy.geometry_tolerance) {
            return self.conflict(Suspension::UnattributedGeometry);
        }
        if self.observed.focus == Focus::Unknown && !self.restoration_requested {
            return Decision::Suspend(Suspension::FocusUnknown);
        }

        let active = if self.restoration_requested {
            None
        } else {
            focused_member(&self.observed)
        };
        if let Some(index) = active {
            let window = match &self.observed.focus {
                Focus::Window(window) => window.clone(),
                _ => unreachable!("focused member has a window"),
            };
            if self.phase == Phase::Settled
                && self
                    .requested()
                    .is_some_and(|effect| effect.plan.active.as_ref() == Some(&window))
            {
                return Decision::Noop(Noop::Stable);
            }
            let desired = match allocation(
                &self.baseline.observation,
                Some(Emphasis {
                    index,
                    share: self.policy.active_share,
                }),
            ) {
                Ok(allocation) => allocation,
                Err(error) => return Decision::Infeasible(error),
            };
            let desired = targets(&self.baseline.observation, &desired);
            if compatible(&self.observed, &desired, self.policy.geometry_tolerance) {
                return Decision::Noop(Noop::AlreadyCompatible);
            }
            self.plan(PlanKind::Emphasize, Some(window), desired, now_ms)
        } else if self.phase == Phase::Settled {
            // Restoration authority requires unchanged identity/context AND
            // observed compatibility with a possibly-sent requested effect.
            let desired = targets(&self.baseline.observation, &self.baseline.allocation);
            self.plan(PlanKind::Restore, None, desired, now_ms)
        } else if self.phase == Phase::Restored {
            Decision::Noop(Noop::Restored)
        } else {
            Decision::Noop(Noop::FocusOutside)
        }
    }

    /// Bounded shutdown uses the same convergence and restoration preconditions.
    /// Once requested, later decisions never start a new emphasis. The runner
    /// supplies its own shutdown deadline and may abandon without writing.
    pub fn request_restore(&mut self, observation: Observation, now_ms: u64) -> Decision {
        self.restoration_requested = true;
        self.decide(observation, now_ms)
    }

    /// Loss revokes restoration authority immediately, retaining diagnostics and
    /// all execution evidence. There is deliberately no automatic retry/reset.
    pub fn lose(&mut self, reason: impl Into<String>) -> Decision {
        self.invalidate(Suspension::SourceLost {
            reason: reason.into(),
        })
    }

    /// Record one sequential native command. Indexing is the plan's target order.
    /// Evidence may still arrive after loss or timeout and remains inspectable.
    pub fn record_command(
        &mut self,
        plan_id: u64,
        index: usize,
        outcome: CommandOutcome,
    ) -> Result<(), EvidenceError> {
        let effect = self
            .effects
            .iter_mut()
            .find(|effect| effect.plan.id == plan_id)
            .ok_or(EvidenceError::UnknownPlan)?;
        let previous = effect
            .commands
            .get_mut(index)
            .ok_or(EvidenceError::InvalidCommandIndex)?;
        if !can_refine(previous, &outcome, effect.execution_finished) {
            return Err(EvidenceError::ContradictoryEvidence);
        }
        *previous = outcome;
        Ok(())
    }

    /// Mark the mutation sequence stopped, including its definitely-unsent tail.
    /// Do not finish while an executor may still submit more commands.
    pub fn finish_execution(&mut self, plan_id: u64) -> Result<(), EvidenceError> {
        let effect = self
            .effects
            .iter_mut()
            .find(|effect| effect.plan.id == plan_id)
            .ok_or(EvidenceError::UnknownPlan)?;
        effect.execution_finished = true;
        Ok(())
    }

    /// Convenience for a completed native sequence, preserving a partial prefix
    /// and unsent tail. Wrong-sized or contradictory reports change nothing.
    pub fn record_execution(
        &mut self,
        plan_id: u64,
        outcomes: Vec<CommandOutcome>,
    ) -> Result<(), EvidenceError> {
        let effect = self
            .effects
            .iter_mut()
            .find(|effect| effect.plan.id == plan_id)
            .ok_or(EvidenceError::UnknownPlan)?;
        if outcomes.len() != effect.commands.len() {
            return Err(EvidenceError::InvalidCommandCount);
        }
        if effect
            .commands
            .iter()
            .zip(&outcomes)
            .any(|(a, b)| !can_refine(a, b, effect.execution_finished))
        {
            return Err(EvidenceError::ContradictoryEvidence);
        }
        effect.commands = outcomes;
        effect.execution_finished = true;
        Ok(())
    }

    fn plan(
        &mut self,
        kind: PlanKind,
        active: Option<WindowKey>,
        targets: Vec<Target>,
        now_ms: u64,
    ) -> Decision {
        let plan = Plan {
            id: self.effects.len() as u64 + 1,
            kind,
            basis: self.observed.clone(),
            active,
            targets,
            created_at_ms: now_ms,
            deadline_ms: now_ms.saturating_add(self.policy.convergence_timeout_ms),
            geometry_tolerance: self.policy.geometry_tolerance,
        };
        self.phase = match kind {
            PlanKind::Emphasize => Phase::Applying,
            PlanKind::Restore => Phase::Restoring,
        };
        self.effects.push(Effect {
            commands: vec![CommandOutcome::Unsent; plan.targets.len()],
            plan: plan.clone(),
            execution_finished: false,
            convergence: Convergence::Waiting,
        });
        self.current_effect = Some(self.effects.len() - 1);
        Decision::Plan(Box::new(plan))
    }

    fn conflict(&mut self, reason: Suspension) -> Decision {
        self.phase = Phase::Conflicted;
        self.suspension = Some(reason.clone());
        if let Some(effect) = self
            .current_effect
            .and_then(|index| self.effects.get_mut(index))
            && effect.convergence == Convergence::Waiting
        {
            effect.convergence = Convergence::Abandoned;
        }
        Decision::Suspend(reason)
    }

    fn invalidate(&mut self, reason: Suspension) -> Decision {
        self.phase = Phase::Lost;
        self.suspension = Some(reason.clone());
        if let Some(effect) = self
            .current_effect
            .and_then(|index| self.effects.get_mut(index))
            && effect.convergence == Convergence::Waiting
        {
            effect.convergence = Convergence::Abandoned;
        }
        Decision::Suspend(reason)
    }
}

fn validate_observation(observation: &Observation) -> Result<(), InputError> {
    if observation.group.source.id.is_empty()
        || observation.group.workspace.is_empty()
        || observation.group.topology.is_empty()
    {
        return Err(InputError::EmptyIdentity);
    }
    if observation.members.is_empty() {
        return Err(InputError::EmptyMembers);
    }
    let mut ids = BTreeSet::new();
    for member in &observation.members {
        if member.id.is_empty() {
            return Err(InputError::EmptyIdentity);
        }
        if !ids.insert(&member.id) {
            return Err(InputError::DuplicateMember {
                id: member.id.clone(),
            });
        }
        if !member.extent.is_finite() || member.extent < 0.0 {
            return Err(InputError::InvalidGeometry {
                id: member.id.clone(),
            });
        }
    }
    if let Focus::Window(window) = &observation.focus
        && (window.id.is_empty() || window.workspace.is_empty() || window.source.id.is_empty())
    {
        return Err(InputError::EmptyIdentity);
    }
    // Validate budget/bounds without deriving new weights from observed effects.
    // Zero observed extents can be legitimate pending/clamped geometry, but a
    // newly acquired baseline must have positive weights (checked by `new`).
    let members: Vec<Member> = observation
        .members
        .iter()
        .map(|member| Member {
            weight: 1.0,
            minimum: member.minimum,
            maximum: member.maximum,
        })
        .collect();
    allocate(Request {
        extent: observation.available_extent,
        gaps: observation.gaps,
        members: &members,
        emphasis: None,
    })
    .map_err(InputError::Allocation)?;
    Ok(())
}

fn allocation(
    observation: &Observation,
    emphasis: Option<Emphasis>,
) -> Result<Allocation, zk_layout::Error> {
    let members: Vec<Member> = observation
        .members
        .iter()
        .map(|member| Member {
            weight: member.extent,
            minimum: member.minimum,
            maximum: member.maximum,
        })
        .collect();
    allocate(Request {
        extent: observation.available_extent,
        gaps: observation.gaps,
        members: &members,
        emphasis,
    })
}

fn targets(observation: &Observation, allocation: &Allocation) -> Vec<Target> {
    observation
        .members
        .iter()
        .zip(&allocation.extents)
        .map(|(member, &extent)| Target {
            member: member.id.clone(),
            extent,
        })
        .collect()
}

fn same_identity(a: &Observation, b: &Observation) -> bool {
    a.group == b.group
        && a.members.len() == b.members.len()
        && a.members.iter().zip(&b.members).all(|(a, b)| a.id == b.id)
}

fn same_context(a: &Observation, b: &Observation) -> bool {
    a.available_extent == b.available_extent
        && a.gaps == b.gaps
        && a.members
            .iter()
            .zip(&b.members)
            .all(|(a, b)| a.minimum == b.minimum && a.maximum == b.maximum)
}

fn compatible(observation: &Observation, targets: &[Target], tolerance: u32) -> bool {
    observation.members.len() == targets.len()
        && observation
            .members
            .iter()
            .zip(targets)
            .all(|(member, target)| {
                member.id == target.member
                    && (member.extent - f64::from(target.extent)).abs() <= f64::from(tolerance)
            })
}

fn focused_member(observation: &Observation) -> Option<usize> {
    let Focus::Window(window) = &observation.focus else {
        return None;
    };
    if window.source != observation.group.source || window.workspace != observation.group.workspace
    {
        return None;
    }
    observation
        .members
        .iter()
        .position(|member| member.id == window.id)
}

fn can_refine(previous: &CommandOutcome, next: &CommandOutcome, finished: bool) -> bool {
    previous == next
        || (!finished && matches!(previous, CommandOutcome::Unsent))
        || matches!(
            (previous, next),
            (
                CommandOutcome::PossiblySent { .. },
                CommandOutcome::Handled | CommandOutcome::Rejected { .. }
            )
        )
}
