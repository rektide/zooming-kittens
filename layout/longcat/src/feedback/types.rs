use serde::{Deserialize, Serialize};

use crate::{CommandOutcome, Focus, GroupKey, Source, Target, WindowKey};

/// Policy clocks are caller-owned monotonic milliseconds. The application owns
/// timers, transport futures, geometry compilation and diagnostic persistence.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeedbackPolicy {
    pub active_share: f64,
    pub geometry_tolerance: u32,
    pub resize_debounce_ms: u64,
    pub convergence_timeout_ms: u64,
}

impl Default for FeedbackPolicy {
    fn default() -> Self {
        Self {
            active_share: 0.7,
            geometry_tolerance: 2,
            resize_debounce_ms: 500,
            convergence_timeout_ms: 1_000,
        }
    }
}

/// Full source census, including temporarily unsupported/tabbed windows. This
/// keeps preferences alive while a window is not in an eligible control group.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveMember {
    pub id: String,
    pub workspace: String,
    /// Adapter-owned presentation/output context. Excludes column/row ordinals,
    /// focus, title and ordinary internal-boundary movement. A change clears the
    /// manual floor allowance, but not the neutral weight or learned share.
    pub context: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeedbackMember {
    pub id: String,
    /// Actual content start and committed extent on the physical secondary axis.
    pub start: f64,
    pub extent: f64,
    pub minimum_extent: u32,
    /// Reported bounds only. Do not promote an empirical clamp to this field.
    pub maximum_extent: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandKey {
    pub attempt: u64,
    pub sequence: u64,
}

/// Geometry attribution is assessed by the adapter's frozen prediction, not by
/// action replies or by pretending stock IPC contains a user-action tag.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Motion {
    #[default]
    Unclassified,
    /// Compatible boundary progress or clients catching up to a frozen target.
    /// A clamp known to be execution-related can also use this classification;
    /// report its blocked response separately, never teach it as user guidance.
    Owned(CommandKey),
    /// Command and independent motion overlap. Drain the command and accept the
    /// final coherent quiet vector as explicitly unattributed guidance.
    Unattributed(CommandKey),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeedbackGroup {
    pub key: GroupKey,
    /// Independent output/presentation context, NOT a geometry revision. Include
    /// changes invalidating native geometry authority, but exclude user outer-
    /// envelope edits if those should teach preferences after reacquisition.
    pub context: String,
    /// An unavailable group remains represented and automatically reacquires.
    pub unavailable: Option<String>,
    pub members: Vec<FeedbackMember>,
    /// Content-only budget from the adapter's geometry model. Hold it through
    /// owned internal corrections, including partial client commits. Reacquire
    /// on external envelope/context changes; never substitute output workarea.
    pub content_budget: f64,
    /// Model-derived, NOT reported, target extents. Keep actual committed sizes
    /// above. None permits acquisition but cannot establish goal convergence.
    pub target_extents: Option<Vec<f64>>,
    /// Adapter assessment: positions, committed sizes, strips and final endpoint
    /// form a coherent settled geometry. Quietness alone does not establish this.
    pub coherent: bool,
    pub motion: Motion,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeedbackInput {
    pub source: Source,
    /// Monotonic local capture order, not a server revision or causal token.
    pub version: u64,
    pub live_members: Vec<LiveMember>,
    pub groups: Vec<FeedbackGroup>,
    pub focus: Focus,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Preference {
    pub window: WindowKey,
    pub neutral_weight: f64,
    pub active_share: Option<f64>,
    pub manual_floor: Option<u32>,
    pub context: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeedbackPhase {
    Acquiring,
    Ready,
    Driving,
    Yielding,
    Limited,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuidanceReason {
    ExternalGeometry,
    UnattributedOverlap,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Guidance {
    pub group: GroupKey,
    pub reason: GuidanceReason,
    /// Member actually emphasized in the expected geometry, not necessarily
    /// the latest focus after focus changes during a drag.
    pub emphasized: Option<String>,
    pub observed_version: u64,
    pub preference_generation: u64,
    pub changed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GoalMember {
    pub id: String,
    pub neutral_weight: f64,
    pub learned_share: Option<f64>,
    pub minimum_extent: u32,
    pub maximum_extent: Option<u32>,
}

/// Inputs defining one finite goal. Capture order and transient committed sizes
/// are deliberately absent; stable mismatch cannot create a new input generation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GoalInputs {
    pub group: GroupKey,
    pub context: String,
    pub content_budget: f64,
    pub members: Vec<GoalMember>,
    pub active: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeedbackGoal {
    pub id: u64,
    pub inputs: GoalInputs,
    pub preference_generation: u64,
    pub basis_version: u64,
    pub targets: Vec<Target>,
    pub created_at_ms: u64,
    pub deadline_ms: u64,
    pub geometry_tolerance: u32,
    pub submission_limit: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EdgeIntent {
    pub basis_version: u64,
    /// Cross-end edge of this member, always with an actual following member.
    pub boundary: usize,
    /// Signed secondary-axis travel, not a native fraction. C owns donor-ready
    /// selection, scale/gain, protocol range and qualified request compilation.
    pub travel: f64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseKind {
    Progress,
    Blocked { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeResponse {
    pub observed_version: u64,
    pub kind: ResponseKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EdgeEvidence {
    pub key: CommandKey,
    pub intent: EdgeIntent,
    pub created_at_ms: u64,
    /// None means the application still owns an outstanding command future.
    pub outcome: Option<CommandOutcome>,
    /// A reply is not a position response. Both are required before another edge.
    pub response: Option<EdgeResponse>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LimitReason {
    Deadline,
    SubmissionBudget,
    Unsent,
    Rejected { message: String },
    PossiblySent { message: String },
    Executor { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetirementReason {
    FocusChanged,
    TopologyChanged,
    ContextChanged,
    Unavailable,
    Yielded,
    SourceLost { reason: String },
    SourceChanged,
    RestoreRequested,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttemptEnd {
    Settled,
    Limited(LimitReason),
    Retired(RetirementReason),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AttemptResult {
    pub end: AttemptEnd,
    /// Actual last geometry in this attempt's scope, never the desired vector.
    pub achieved: Option<FeedbackGroup>,
    pub finished_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeedbackAttempt {
    pub goal: FeedbackGoal,
    pub commands: Vec<EdgeEvidence>,
    /// Set immediately to revoke unsent work, even while a future is draining.
    pub stopping: Option<AttemptEnd>,
    pub result: Option<AttemptResult>,
}

impl FeedbackAttempt {
    pub fn pending(&self) -> Option<&EdgeEvidence> {
        self.commands.last().filter(|edge| edge.outcome.is_none())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedbackCounters {
    pub attempts: u64,
    pub settled: u64,
    pub limited: u64,
    pub retired: u64,
    pub submissions: u64,
    pub guidance: u64,
    pub acquisitions: u64,
    pub continuity_losses: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaitReason {
    Quiet,
    CoherentGeometry,
    PendingCommand,
    PositionResponse,
    ClientCommit,
    FocusUnknown,
    NoEligibleGroup,
    Stable,
    Limited,
    /// One-slot completion mailbox: persist/take this evidence before advancing.
    /// It is backpressure, not an ever-growing historical admission lock.
    Completion,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FeedbackDecision {
    /// One current goal, not a queued native batch. C must call begin_command
    /// against the latest basis before sending a single selected correction.
    Drive(Box<FeedbackGoal>),
    Wait {
        phase: FeedbackPhase,
        reason: WaitReason,
    },
    Infeasible {
        group: GroupKey,
        error: zk_layout::Error,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeedbackError {
    InvalidPolicy,
    InvalidInput { reason: String },
    ClockRegressed,
    StaleObservation,
    UnknownCommand,
    ContradictoryEvidence,
    NotDriving,
    StaleBasis,
    InvalidEdge,
}

impl std::fmt::Display for FeedbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "feedback control error: {self:?}")
    }
}

impl std::error::Error for FeedbackError {}
