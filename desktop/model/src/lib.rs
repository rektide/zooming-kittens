//! Semantic desktop captures. Native adapters own protocol state and lifecycle.
#![forbid(unsafe_code)]

use std::{collections::{BTreeMap, BTreeSet}, sync::Arc};
use tokio::sync::{broadcast, watch};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Backend { Niri, Umbriel }

/// Caller-chosen source identity; reconnects use a new epoch, not a new name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceId { pub backend: Backend, pub name: String }

impl SourceId {
    pub fn new(backend: Backend, name: impl Into<String>) -> Self {
        Self { backend, name: name.into() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Epoch(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Scope { pub source: SourceId, pub epoch: Epoch }

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NativeId { Number(u64), Text(String) }

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowKey { pub scope: Scope, pub native: NativeId }

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkspaceKey { pub scope: Scope, pub native: NativeId }

/// A location hint within one capture, NOT persistent topology identity.
/// Bind a control episode to its ordered members as well as this location.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupKey { pub workspace: WorkspaceKey, pub position: NativeId }

/// Adapter-local observation order, never a server revision or resume token.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObservationVersion(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FamilyStamp { pub epoch: Epoch, pub position: u64 }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LossReason {
    Disconnected, Shutdown, EpochChanged, InvalidSnapshot, ObserverLag,
    Protocol(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Health { Unseen, Current, Stale(LossReason) }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamilyStatus { pub stamp: Option<FamilyStamp>, pub health: Health }

impl Default for FamilyStatus {
    fn default() -> Self { Self { stamp: None, health: Health::Unseen } }
}

impl FamilyStatus {
    pub fn is_current(&self) -> bool { self.health == Health::Current }
    pub fn stale(&mut self, reason: LossReason) {
        if self.stamp.is_some() { self.health = Health::Stale(reason); }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Evidence { Reported, Inferred, Unavailable }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis { Horizontal, Vertical }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point { pub x: f64, pub y: f64 }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size { pub width: f64, pub height: f64 }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect { pub origin: Point, pub size: Size }

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
pub enum Placement { Tiled, Floating, Scratchpad(String), Unknown }

#[derive(Debug, Clone, PartialEq)]
pub struct WindowView {
    pub key: WindowKey,
    pub workspace: Option<WorkspaceKey>,
    pub group: Option<GroupKey>,
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
        self.key == other.key && self.workspace == other.workspace && self.group == other.group
            && self.placement == other.placement && self.active == other.active
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
    pub secondary_axis: Axis,
    pub membership: Evidence,
    pub bounds: Option<Rect>,
    pub geometry_evidence: Evidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    pub reported_groups: bool,
    pub height_by_id: bool,
    pub atomic_multiwindow: bool,
    pub native_revision_precondition: bool,
    pub exact_restore: bool,
}

impl Capabilities {
    pub const READ_ONLY: Self = Self {
        reported_groups: false, height_by_id: false, atomic_multiwindow: false,
        native_revision_precondition: false, exact_restore: false,
    };
}

#[derive(Debug, Clone, PartialEq)]
pub struct DesktopCapture {
    pub scope: Scope,
    pub version: ObservationVersion,
    pub connected: bool,
    pub windows_status: FamilyStatus,
    pub workspaces_status: FamilyStatus,
    pub windows: BTreeMap<WindowKey, WindowView>,
    pub workspaces: BTreeMap<WorkspaceKey, WorkspaceView>,
    pub groups: BTreeMap<GroupKey, GroupView>,
    pub active_window: Option<WindowKey>,
    pub capabilities: Capabilities,
}

impl DesktopCapture {
    pub fn empty(scope: Scope, capabilities: Capabilities) -> Self {
        Self { scope, version: ObservationVersion(0), connected: true,
            windows_status: FamilyStatus::default(), workspaces_status: FamilyStatus::default(),
            windows: BTreeMap::new(), workspaces: BTreeMap::new(), groups: BTreeMap::new(),
            active_window: None, capabilities }
    }

    /// Local joined readiness, NOT an atomic joint compositor revision.
    pub fn is_ready(&self) -> bool {
        self.connected && self.windows_status.is_current() && self.workspaces_status.is_current()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Impact {
    pub windows: BTreeSet<WindowKey>,
    pub workspaces: BTreeSet<WorkspaceKey>,
    pub groups: BTreeSet<GroupKey>,
    pub focus_changed: bool,
    pub health_changed: bool,
}

impl Impact {
    /// Semantic projection retaining BOTH former and current regions.
    pub fn between(before: &DesktopCapture, after: &DesktopCapture) -> Self {
        let mut impact = Self {
            focus_changed: before.active_window != after.active_window,
            health_changed: before.connected != after.connected || before.scope != after.scope
                || before.windows_status.health != after.windows_status.health
                || before.workspaces_status.health != after.workspaces_status.health,
            ..Self::default()
        };
        for key in before.windows.keys().chain(after.windows.keys()) {
            let old = before.windows.get(key);
            let new = after.windows.get(key);
            if matches!((old, new), (Some(a), Some(b)) if a.layout_eq(b)) { continue; }
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
            if before.groups.get(key) != after.groups.get(key) { impact.groups.insert(key.clone()); }
        }
        impact
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Continuity { Initial, Continuous, Discontinuous(LossReason) }

#[derive(Debug, Clone)]
pub struct DesktopUpdate {
    pub before: Arc<DesktopCapture>,
    pub after: Arc<DesktopCapture>,
    pub impact: Impact,
    pub continuity: Continuity,
}

impl DesktopUpdate {
    pub fn new(before: Arc<DesktopCapture>, after: Arc<DesktopCapture>, continuity: Continuity) -> Self {
        let impact = Impact::between(&before, &after);
        Self { before, after, impact, continuity }
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
pub enum ObserveError { Lagged { skipped: u64 }, Closed }

impl std::fmt::Display for ObserveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "desktop observation: {self:?}") }
}
impl std::error::Error for ObserveError {}

impl Observer {
    pub fn current(&self) -> Arc<DesktopCapture> { self.current.borrow().clone() }
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
            Err(broadcast::error::TryRecvError::Lagged(skipped)) => Err(ObserveError::Lagged { skipped }),
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
        (Self { events, current }, Observer { events: receiver, current: watch })
    }
    pub fn watch(&self) -> watch::Receiver<Arc<DesktopCapture>> { self.current.subscribe() }
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
        let scope = Scope { source: SourceId::new(Backend::Niri, "test"), epoch: Epoch(1) };
        let initial = Arc::new(DesktopCapture::empty(scope, Capabilities::READ_ONLY));
        let (publisher, mut observer) = Publisher::new(initial.clone(), 1);
        for version in 1..=4 {
            let mut after = (*initial).clone();
            after.version = ObservationVersion(version);
            publisher.publish(DesktopUpdate::new(initial.clone(), Arc::new(after), Continuity::Continuous));
        }
        assert!(matches!(observer.recv().await, Err(ObserveError::Lagged { .. })));
        assert_eq!(observer.current().version.0, 4);
        assert_eq!(observer.recv().await.unwrap().after.version.0, 4);
        drop(publisher);
        assert_eq!(observer.recv().await.unwrap_err(), ObserveError::Closed);
        assert_eq!(observer.current().version.0, 4);
    }
}
