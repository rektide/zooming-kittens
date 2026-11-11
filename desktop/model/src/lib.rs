//! Semantic desktop captures. Native adapters own protocol state and lifecycle.
#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use tokio::sync::{broadcast, watch};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Backend {
    Niri,
    Umbriel,
}

/// Caller-chosen source identity; reconnects use a new epoch, not a new name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceId {
    pub backend: Backend,
    pub name: String,
}

impl SourceId {
    pub fn new(backend: Backend, name: impl Into<String>) -> Self {
        Self {
            backend,
            name: name.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Epoch(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Scope {
    pub source: SourceId,
    pub epoch: Epoch,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NativeId {
    Number(u64),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowKey {
    pub scope: Scope,
    pub native: NativeId,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkspaceKey {
    pub scope: Scope,
    pub native: NativeId,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OutputKey {
    pub scope: Scope,
    pub name: String,
}

/// A location hint within one capture, NOT persistent topology identity.
/// Bind a control episode to its ordered members as well as this location.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupKey {
    pub workspace: WorkspaceKey,
    pub position: NativeId,
}

/// Adapter-local observation order, never a server revision or resume token.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObservationVersion(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FamilyStamp {
    pub epoch: Epoch,
    pub position: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LossReason {
    Disconnected,
    Shutdown,
    EpochChanged,
    InvalidSnapshot,
    ObserverLag,
    Protocol(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Health {
    Unseen,
    Current,
    Stale(LossReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamilyStatus {
    pub stamp: Option<FamilyStamp>,
    pub health: Health,
}

impl Default for FamilyStatus {
    fn default() -> Self {
        Self {
            stamp: None,
            health: Health::Unseen,
        }
    }
}

impl FamilyStatus {
    pub fn is_current(&self) -> bool {
        self.health == Health::Current
    }
    pub fn stale(&mut self, reason: LossReason) {
        if self.stamp.is_some() {
            self.health = Health::Stale(reason);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Evidence {
    Reported,
    Inferred,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

/// Geometry meanings remain separate even when the values happen to agree.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Geometry {
    pub content_size: Option<Size>,
    pub tile_size: Option<Size>,
    pub tile_position: Option<Point>,
    pub window_offset_in_tile: Option<Point>,
    pub target_position: Option<Point>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placement {
    Tiled,
    Floating,
    Scratchpad(String),
    Unknown,
}

/// Unreported flags stay None. Native presentation is not a policy preference.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Presentation {
    pub title_bar: Option<bool>,
    pub tabbed: Option<bool>,
    pub tab_index: Option<i32>,
    pub tab_hidden: Option<bool>,
    pub fullscreen: Option<bool>,
    pub maximized: Option<bool>,
    pub maximized_to_edges: Option<bool>,
    pub pinned: Option<bool>,
    pub minimized: Option<bool>,
    pub visible: Option<bool>,
    pub parent: Option<WindowKey>,
    pub region: Option<String>,
}

impl Presentation {
    pub fn ordinary_untabbed(&self) -> bool {
        [
            self.tabbed,
            self.tab_hidden,
            self.fullscreen,
            self.maximized,
            self.maximized_to_edges,
            self.pinned,
            self.minimized,
        ]
        .into_iter()
        .all(|v| v == Some(false))
            && self.title_bar.is_some()
            && self.tab_index == Some(-1)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowView {
    pub key: WindowKey,
    pub workspace: Option<WorkspaceKey>,
    pub group: Option<GroupKey>,
    pub output: Option<OutputKey>,
    pub presentation: Presentation,
    pub placement: Placement,
    pub title: Option<String>,
    pub app_id: Option<String>,
    pub pid: Option<i32>,
    /// Seat-global focus/activation; not workspace-local remembered focus.
    pub active: bool,
    pub geometry: Geometry,
}

impl WindowView {
    pub fn layout_eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.workspace == other.workspace
            && self.group == other.group
            && self.output == other.output
            && self.presentation == other.presentation
            && self.placement == other.placement
            && self.active == other.active
            && self.geometry == other.geometry
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceView {
    pub key: WorkspaceKey,
    pub name: Option<String>,
    pub output: Option<String>,
    pub active: bool,
    pub focused: bool,
    pub layout: Option<String>,
    pub geometry: Option<Rect>,
    pub grouping: Evidence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupView {
    pub key: GroupKey,
    pub members: Vec<WindowKey>,
    /// Stock membership can be reported while its physical axis is ambiguous.
    pub secondary_axis: Option<Axis>,
    pub membership: Evidence,
    pub bounds: Option<Rect>,
    pub geometry_evidence: Evidence,
    /// Local geometry/presentation admission only; not a native atomic predicate
    /// or proof that the server understands qualified action grammar.
    pub edge_eligibility: EdgeEligibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeEligibility {
    Eligible,
    NotSupported,
    FamilyUnavailable,
    InconsistentMembership,
    UnsupportedLayout,
    UnsupportedPresentation,
    OutputUnavailable,
    AmbiguousAxis,
    TooFewMembers,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutputView {
    pub key: OutputKey,
    pub enabled: Option<bool>,
    pub powered: Option<bool>,
    pub position: Option<Point>,
    pub logical_size: Option<Size>,
    /// Reported output workarea, NOT exact native tiledArea or content budget.
    pub usable_area: Option<Rect>,
    pub scale: Option<f64>,
    pub transform: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    pub reported_groups: bool,
    pub height_by_id: bool,
    /// Adapter offers qualified relative edges; runtime server grammar admission
    /// is additionally required. Does NOT imply fixed-pixel assignment.
    pub relative_edge_by_id: bool,
    pub atomic_multiwindow: bool,
    pub native_revision_precondition: bool,
    pub exact_restore: bool,
}

/// Adapter-owned native evidence with a solver-facing projection. Opaque payloads
/// are compared byte-for-byte and interpreted only by their originating adapter.
/// They are never reconstructed from geometry or refreshed after an external edit.
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalAllocation {
    pub basis: Arc<[u8]>,
    /// All conditional sizing context except the assigned native weights.
    pub context: Arc<[u8]>,
    pub restore_payload: Arc<[u8]>,
    pub content_budget: u32,
    pub members: Vec<AllocationMember>,
    pub edge_gaps: (u32, u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllocationMember {
    pub target_content: u32,
    pub decoration: u32,
    pub minimum: u32,
    pub maximum: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct GroupAllocationRequest {
    pub group: GroupKey,
    pub expected: Arc<[u8]>,
    /// Some restores the original native state; None compiles pixel emphasis.
    pub restore_payload: Option<Arc<[u8]>>,
    pub targets: Vec<u32>,
    pub edge_gaps: (u32, u32),
    pub expected_active: Option<WindowKey>,
}

/// One signed motion of the cross-end edge of target. `following` must be its
/// actual next row; never use the group's last outer edge. Fraction is of native
/// tiledArea, not a pixel count, workarea fraction guarantee, or solver share.
#[derive(Debug, Clone, PartialEq)]
pub struct RelativeEdgeRequest {
    pub group: GroupKey,
    pub target: WindowKey,
    pub following: WindowKey,
    pub basis: ObservationVersion,
    pub axis: Axis,
    pub fraction: f64,
}

impl Capabilities {
    pub const READ_ONLY: Self = Self {
        reported_groups: false,
        height_by_id: false,
        relative_edge_by_id: false,
        atomic_multiwindow: false,
        native_revision_precondition: false,
        exact_restore: false,
    };
}

#[derive(Debug, Clone, PartialEq)]
pub struct DesktopCapture {
    pub scope: Scope,
    pub version: ObservationVersion,
    pub connected: bool,
    pub windows_status: FamilyStatus,
    pub workspaces_status: FamilyStatus,
    /// Stock requires all three independent families. None for adapters whose
    /// readiness contract does not use a separately observed outputs family.
    pub outputs_status: Option<FamilyStatus>,
    pub windows: BTreeMap<WindowKey, WindowView>,
    pub workspaces: BTreeMap<WorkspaceKey, WorkspaceView>,
    pub outputs: BTreeMap<OutputKey, OutputView>,
    pub groups: BTreeMap<GroupKey, GroupView>,
    pub active_window: Option<WindowKey>,
    pub capabilities: Capabilities,
    /// When present, this coherent family alone establishes control readiness.
    pub control_status: Option<FamilyStatus>,
    pub allocations: BTreeMap<GroupKey, ConditionalAllocation>,
}

impl DesktopCapture {
    pub fn empty(scope: Scope, capabilities: Capabilities) -> Self {
        Self {
            scope,
            version: ObservationVersion(0),
            connected: true,
            windows_status: FamilyStatus::default(),
            workspaces_status: FamilyStatus::default(),
            outputs_status: None,
            windows: BTreeMap::new(),
            workspaces: BTreeMap::new(),
            outputs: BTreeMap::new(),
            groups: BTreeMap::new(),
            active_window: None,
            capabilities,
            control_status: None,
            allocations: BTreeMap::new(),
        }
    }

    /// Local joined readiness, NOT an atomic joint compositor revision.
    pub fn is_ready(&self) -> bool {
        self.connected
            && self.control_status.as_ref().map_or_else(
                || {
                    self.windows_status.is_current()
                        && self.workspaces_status.is_current()
                        && self
                            .outputs_status
                            .as_ref()
                            .is_none_or(FamilyStatus::is_current)
                        && [&self.windows_status, &self.workspaces_status]
                            .into_iter()
                            .chain(self.outputs_status.as_ref())
                            .all(|s| s.stamp.is_some_and(|s| s.epoch == self.scope.epoch))
                },
                FamilyStatus::is_current,
            )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Impact {
    pub windows: BTreeSet<WindowKey>,
    pub workspaces: BTreeSet<WorkspaceKey>,
    pub groups: BTreeSet<GroupKey>,
    pub outputs: BTreeSet<OutputKey>,
    pub focus_changed: bool,
    pub health_changed: bool,
}

impl Impact {
    /// Semantic projection retaining BOTH former and current regions.
    pub fn between(before: &DesktopCapture, after: &DesktopCapture) -> Self {
        let mut impact = Self {
            focus_changed: before.active_window != after.active_window,
            health_changed: before.connected != after.connected
                || before.scope != after.scope
                || before.windows_status.health != after.windows_status.health
                || before.workspaces_status.health != after.workspaces_status.health
                || before.outputs_status.as_ref().map(|s| &s.health)
                    != after.outputs_status.as_ref().map(|s| &s.health)
                || before.control_status.as_ref().map(|s| &s.health)
                    != after.control_status.as_ref().map(|s| &s.health),
            // Native basis edits may leave every rounded pixel unchanged.
            ..Self::default()
        };
        for key in before.windows.keys().chain(after.windows.keys()) {
            let old = before.windows.get(key);
            let new = after.windows.get(key);
            if matches!((old, new), (Some(a), Some(b)) if a.layout_eq(b)) {
                continue;
            }
            impact.windows.insert(key.clone());
            for window in old.into_iter().chain(new) {
                impact.workspaces.extend(window.workspace.clone());
                impact.groups.extend(window.group.clone());
            }
        }
        for key in before.workspaces.keys().chain(after.workspaces.keys()) {
            if before.workspaces.get(key) != after.workspaces.get(key) {
                impact.workspaces.insert(key.clone());
            }
        }
        for key in before.groups.keys().chain(after.groups.keys()) {
            if before.groups.get(key) != after.groups.get(key)
                || before.allocations.get(key) != after.allocations.get(key)
            {
                impact.groups.insert(key.clone());
            }
        }
        for key in before.outputs.keys().chain(after.outputs.keys()) {
            if before.outputs.get(key) == after.outputs.get(key) {
                continue;
            }
            impact.outputs.insert(key.clone());
            for capture in [before, after] {
                for group in capture.groups.values() {
                    if capture
                        .workspaces
                        .get(&group.key.workspace)
                        .is_some_and(|w| {
                            w.key.scope == key.scope && w.output.as_ref() == Some(&key.name)
                        })
                    {
                        impact.groups.insert(group.key.clone());
                        impact.workspaces.insert(group.key.workspace.clone());
                    }
                }
            }
        }
        impact
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Continuity {
    Initial,
    Continuous,
    Discontinuous(LossReason),
}

#[derive(Debug, Clone)]
pub struct DesktopUpdate {
    pub before: Arc<DesktopCapture>,
    pub after: Arc<DesktopCapture>,
    pub impact: Impact,
    pub continuity: Continuity,
}

impl DesktopUpdate {
    pub fn new(
        before: Arc<DesktopCapture>,
        after: Arc<DesktopCapture>,
        continuity: Continuity,
    ) -> Self {
        let impact = Impact::between(&before, &after);
        Self {
            before,
            after,
            impact,
            continuity,
        }
    }
}

/// Bounded item delivery; slow consumers must invalidate episodes on Lagged.
/// Queue capacity is an item count, NOT a byte budget. Native adapters separately
/// bound frames and inventory cardinality. Optional observers cannot block I/O.
pub struct Observer {
    events: broadcast::Receiver<Arc<DesktopUpdate>>,
    current: watch::Receiver<Arc<DesktopCapture>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObserveError {
    Lagged { skipped: u64 },
    Closed,
}

impl std::fmt::Display for ObserveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "desktop observation: {self:?}")
    }
}
impl std::error::Error for ObserveError {}

impl Observer {
    pub fn current(&self) -> Arc<DesktopCapture> {
        self.current.borrow().clone()
    }
    pub async fn recv(&mut self) -> Result<Arc<DesktopUpdate>, ObserveError> {
        self.events.recv().await.map_err(|e| match e {
            broadcast::error::RecvError::Lagged(skipped) => ObserveError::Lagged { skipped },
            broadcast::error::RecvError::Closed => ObserveError::Closed,
        })
    }
    pub fn try_recv(&mut self) -> Result<Option<Arc<DesktopUpdate>>, ObserveError> {
        match self.events.try_recv() {
            Ok(update) => Ok(Some(update)),
            Err(broadcast::error::TryRecvError::Empty) => Ok(None),
            Err(broadcast::error::TryRecvError::Lagged(skipped)) => {
                Err(ObserveError::Lagged { skipped })
            }
            Err(broadcast::error::TryRecvError::Closed) => Err(ObserveError::Closed),
        }
    }
}

/// Adapter-facing publisher, not a source owner or transport abstraction.
pub struct Publisher {
    events: broadcast::Sender<Arc<DesktopUpdate>>,
    current: watch::Sender<Arc<DesktopCapture>>,
}

impl Publisher {
    pub fn new(initial: Arc<DesktopCapture>, capacity: usize) -> (Self, Observer) {
        assert!(capacity > 0, "desktop event capacity must be nonzero");
        let (events, receiver) = broadcast::channel(capacity);
        let (current, watch) = watch::channel(initial);
        (
            Self { events, current },
            Observer {
                events: receiver,
                current: watch,
            },
        )
    }
    pub fn watch(&self) -> watch::Receiver<Arc<DesktopCapture>> {
        self.current.subscribe()
    }
    pub fn publish(&self, update: DesktopUpdate) {
        self.current.send_replace(update.after.clone());
        let _ = self.events.send(Arc::new(update));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn bounded_lag_is_explicit_and_current_survives_close() {
        let scope = Scope {
            source: SourceId::new(Backend::Niri, "test"),
            epoch: Epoch(1),
        };
        let initial = Arc::new(DesktopCapture::empty(scope, Capabilities::READ_ONLY));
        let (publisher, mut observer) = Publisher::new(initial.clone(), 1);
        for version in 1..=4 {
            let mut after = (*initial).clone();
            after.version = ObservationVersion(version);
            publisher.publish(DesktopUpdate::new(
                initial.clone(),
                Arc::new(after),
                Continuity::Continuous,
            ));
        }
        assert!(matches!(
            observer.recv().await,
            Err(ObserveError::Lagged { .. })
        ));
        assert_eq!(observer.current().version.0, 4);
        assert_eq!(observer.recv().await.unwrap().after.version.0, 4);
        drop(publisher);
        assert_eq!(observer.recv().await.unwrap_err(), ObserveError::Closed);
        assert_eq!(observer.current().version.0, 4);
    }
}
