use kubeless_view_impact::affected_roots_with;
use kubeless_view_memory::{MutationBatch, Reader, Replacement, Writer, memory_view};
use niri_ipc::{Event, Window, WindowLayout, Workspace};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use zk_desktop::*;

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub windows: usize,
    pub workspaces: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            windows: 16_384,
            workspaces: 4_096,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReduceError(pub String);
impl std::fmt::Display for ReduceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ReduceError {}

/// Serialized native owner. Each window event commits one complete local batch.
pub struct Reducer {
    reader: Reader<u64, Window>,
    writer: Writer<u64, Window>,
    workspaces: BTreeMap<u64, Workspace>,
    current: Arc<DesktopCapture>,
    limits: Limits,
}

impl Reducer {
    pub fn new(source: SourceId, epoch: Epoch, limits: Limits) -> Self {
        assert_eq!(source.backend, Backend::Niri);
        let (reader, writer) = memory_view();
        let capture = DesktopCapture::empty(
            Scope { source, epoch },
            Capabilities {
                reported_groups: true,
                height_by_id: true,
                ..Capabilities::READ_ONLY
            },
        );
        Self {
            reader,
            writer,
            workspaces: BTreeMap::new(),
            current: Arc::new(capture),
            limits,
        }
    }

    pub fn capture(&self) -> Arc<DesktopCapture> {
        self.current.clone()
    }

    pub fn begin_epoch(&mut self, epoch: Epoch) -> Result<DesktopUpdate, ReduceError> {
        if epoch <= self.current.scope.epoch {
            return Err(error("epoch must increase"));
        }
        let mut after = (*self.current).clone();
        after.scope.epoch = epoch;
        after.connected = true;
        after.windows_status.stale(LossReason::EpochChanged);
        after.workspaces_status.stale(LossReason::EpochChanged);
        Ok(self.commit_capture(after, Continuity::Discontinuous(LossReason::EpochChanged)))
    }

    pub fn loss(&mut self, reason: LossReason) -> DesktopUpdate {
        let mut after = (*self.current).clone();
        after.connected = false;
        after.windows_status.stale(reason.clone());
        after.workspaces_status.stale(reason.clone());
        self.commit_capture(after, Continuity::Discontinuous(reason))
    }

    pub fn apply(&mut self, event: Event) -> Result<DesktopUpdate, ReduceError> {
        let result = self.apply_inner(event);
        if result.is_err() {
            // Retain diagnostic rows, but never leave rejected native evidence ready.
            self.loss(LossReason::InvalidSnapshot);
        }
        result
    }

    fn apply_inner(&mut self, event: Event) -> Result<DesktopUpdate, ReduceError> {
        if !self.current.connected {
            return Err(error("source lost; begin a new epoch before events"));
        }
        let mut after = (*self.current).clone();
        let stamp = Some(FamilyStamp {
            epoch: after.scope.epoch,
            position: after.version.0 + 1,
        });
        let mut receipt = None;
        match event {
            Event::WindowsChanged { windows } => {
                if windows.len() > self.limits.windows {
                    return Err(error("window census exceeds limit"));
                }
                validate_windows(&windows)?;
                let replacement =
                    Replacement::try_from_entries(windows.into_iter().map(|w| (w.id, w)))
                        .map_err(|_| error("duplicate window ID"))?;
                receipt = Some(self.writer.replace_changes(replacement));
                after.windows_status = FamilyStatus {
                    stamp,
                    health: Health::Current,
                };
            }
            Event::WindowOpenedOrChanged { window } => {
                self.require_windows()?;
                validate_layout(&window.layout)?;
                if self.reader.get(&window.id).is_none() && self.reader.len() >= self.limits.windows
                {
                    return Err(error("window inventory exceeds limit"));
                }
                let mut batch = MutationBatch::new();
                if window.is_focused {
                    for (id, old) in self.reader.snapshot().entries {
                        if id != window.id && old.is_focused {
                            let mut old = (*old).clone();
                            old.is_focused = false;
                            batch.upsert(id, old);
                        }
                    }
                }
                batch.upsert(window.id, window);
                receipt = Some(self.writer.apply_changes(batch));
                after.windows_status.stamp = stamp;
            }
            Event::WindowClosed { id } => {
                self.require_windows()?;
                let mut batch = MutationBatch::new();
                batch.remove(id);
                receipt = Some(self.writer.apply_changes(batch));
                after.windows_status.stamp = stamp;
            }
            Event::WindowFocusChanged { id } => {
                self.require_windows()?;
                if id.is_some_and(|id| self.reader.get(&id).is_none()) {
                    return Err(error("focus names unknown window"));
                }
                let mut batch = MutationBatch::new();
                for (key, window) in self.reader.snapshot().entries {
                    let focused = Some(key) == id;
                    if focused != window.is_focused {
                        let mut window = (*window).clone();
                        window.is_focused = focused;
                        batch.upsert(key, window);
                    }
                }
                receipt = Some(self.writer.apply_changes(batch));
                after.windows_status.stamp = stamp;
            }
            Event::WindowLayoutsChanged { changes } => {
                self.require_windows()?;
                let mut seen = BTreeSet::new();
                let mut batch = MutationBatch::new();
                for (id, layout) in changes {
                    validate_layout(&layout)?;
                    if !seen.insert(id) {
                        return Err(error("duplicate layout-change ID"));
                    }
                    let window = self
                        .reader
                        .get(&id)
                        .ok_or_else(|| error("layout names unknown window"))?;
                    let mut window = (*window).clone();
                    window.layout = layout;
                    batch.upsert(id, window);
                }
                receipt = Some(self.writer.apply_changes(batch));
                after.windows_status.stamp = stamp;
            }
            Event::WindowFocusTimestampChanged {
                id,
                focus_timestamp,
            } => {
                self.require_windows()?;
                let window = self
                    .reader
                    .get(&id)
                    .ok_or_else(|| error("timestamp names unknown window"))?;
                let mut window = (*window).clone();
                window.focus_timestamp = focus_timestamp;
                let mut batch = MutationBatch::new();
                batch.upsert(id, window);
                receipt = Some(self.writer.apply_changes(batch));
                after.windows_status.stamp = stamp;
            }
            Event::WindowUrgencyChanged { id, urgent } => {
                self.require_windows()?;
                let window = self
                    .reader
                    .get(&id)
                    .ok_or_else(|| error("urgency names unknown window"))?;
                let mut window = (*window).clone();
                window.is_urgent = urgent;
                let mut batch = MutationBatch::new();
                batch.upsert(id, window);
                receipt = Some(self.writer.apply_changes(batch));
                after.windows_status.stamp = stamp;
            }
            Event::WorkspacesChanged { workspaces } => {
                if workspaces.len() > self.limits.workspaces {
                    return Err(error("workspace census exceeds limit"));
                }
                let mut replacement = BTreeMap::new();
                for workspace in workspaces {
                    if replacement.insert(workspace.id, workspace).is_some() {
                        return Err(error("duplicate workspace ID"));
                    }
                }
                self.workspaces = replacement;
                after.workspaces_status = FamilyStatus {
                    stamp,
                    health: Health::Current,
                };
            }
            Event::WorkspaceActivated { id, focused } => {
                self.require_workspaces()?;
                let output = self
                    .workspaces
                    .get(&id)
                    .ok_or_else(|| error("activation names unknown workspace"))?
                    .output
                    .clone();
                for workspace in self.workspaces.values_mut() {
                    if workspace.output == output {
                        workspace.is_active = workspace.id == id;
                    }
                    if focused {
                        workspace.is_focused = workspace.id == id;
                    }
                }
                after.workspaces_status.stamp = stamp;
            }
            Event::WorkspaceActiveWindowChanged {
                workspace_id,
                active_window_id,
            } => {
                self.require_workspaces()?;
                self.workspaces
                    .get_mut(&workspace_id)
                    .ok_or_else(|| error("active window names unknown workspace"))?
                    .active_window_id = active_window_id;
                after.workspaces_status.stamp = stamp;
            }
            Event::WorkspaceUrgencyChanged { id, urgent } => {
                self.require_workspaces()?;
                self.workspaces
                    .get_mut(&id)
                    .ok_or_else(|| error("urgency names unknown workspace"))?
                    .is_urgent = urgent;
                after.workspaces_status.stamp = stamp;
            }
            _ => {}
        }
        self.project(&mut after);
        let continuity =
            if !self.current.windows_status.is_current() && after.windows_status.is_current() {
                Continuity::Initial
            } else {
                Continuity::Continuous
            };
        let mut update = self.commit_capture(after, continuity);
        if let Some(mut receipt) = receipt {
            // Filter native commit evidence by policy-relevant equality, not Arc identity.
            let scope = update.after.scope.clone();
            receipt
                .evidence
                .retain(|row| match (&row.before, &row.after) {
                    (Some(a), Some(b)) => {
                        !project_window(a, &scope).layout_eq(&project_window(b, &scope))
                    }
                    (None, None) => false,
                    _ => true,
                });
            let mut old_scope = update.before.scope.clone();
            if let Some(stamp) = update.before.windows_status.stamp {
                old_scope.epoch = stamp.epoch;
            }
            let roots = affected_roots_with(
                &receipt.evidence,
                |_, window| project_window(window, &old_scope).workspace,
                |_, window| project_window(window, &scope).workspace,
            );
            update
                .impact
                .workspaces
                .extend(roots.keys().iter().cloned());
        }
        Ok(update)
    }

    fn require_windows(&self) -> Result<(), ReduceError> {
        if self.current.windows_status.is_current() {
            Ok(())
        } else {
            Err(error("window delta before current census"))
        }
    }
    fn require_workspaces(&self) -> Result<(), ReduceError> {
        if self.current.workspaces_status.is_current() {
            Ok(())
        } else {
            Err(error("workspace delta before current census"))
        }
    }
    fn commit_capture(
        &mut self,
        mut after: DesktopCapture,
        continuity: Continuity,
    ) -> DesktopUpdate {
        after.version.0 = self
            .current
            .version
            .0
            .checked_add(1)
            .expect("observation version exhausted");
        let after = Arc::new(after);
        let update = DesktopUpdate::new(self.current.clone(), after.clone(), continuity);
        self.current = after;
        update
    }
    fn project(&self, after: &mut DesktopCapture) {
        let mut window_scope = after.scope.clone();
        if let Some(stamp) = after.windows_status.stamp {
            window_scope.epoch = stamp.epoch;
        }
        after.windows = self
            .reader
            .snapshot()
            .entries
            .into_iter()
            .map(|(_, w)| {
                let window = project_window(&w, &window_scope);
                (window.key.clone(), window)
            })
            .collect();
        after.active_window = after
            .windows
            .values()
            .find(|w| w.active)
            .map(|w| w.key.clone());
        let mut workspace_scope = after.scope.clone();
        if let Some(stamp) = after.workspaces_status.stamp {
            workspace_scope.epoch = stamp.epoch;
        }
        after.workspaces = self
            .workspaces
            .values()
            .map(|w| {
                let key = workspace_key(&workspace_scope, w.id);
                (
                    key.clone(),
                    WorkspaceView {
                        key,
                        name: w.name.clone(),
                        output: w.output.clone(),
                        active: w.is_active,
                        focused: w.is_focused,
                        layout: None,
                        geometry: None,
                        grouping: Evidence::Reported,
                    },
                )
            })
            .collect();
        let mut groups = BTreeMap::<GroupKey, Vec<(usize, WindowKey)>>::new();
        for (_, window) in self.reader.snapshot().entries {
            let projected = project_window(&window, &window_scope);
            if let (Some(group), Some((_, row))) =
                (projected.group, window.layout.pos_in_scrolling_layout)
            {
                groups.entry(group).or_default().push((row, projected.key));
            }
        }
        after.groups = groups
            .into_iter()
            .map(|(key, mut rows)| {
                rows.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
                let unambiguous = rows.windows(2).all(|pair| pair[0].0 != pair[1].0);
                let members: Vec<_> = rows.into_iter().map(|(_, id)| id).collect();
                let bounds = group_bounds(members.iter().map(|id| &after.windows[id]));
                (
                    key.clone(),
                    GroupView {
                        key,
                        members,
                        secondary_axis: Axis::Vertical,
                        membership: if unambiguous {
                            Evidence::Reported
                        } else {
                            Evidence::Unavailable
                        },
                        bounds,
                        geometry_evidence: Evidence::Inferred,
                    },
                )
            })
            .collect();
    }
}

fn error(message: &str) -> ReduceError {
    ReduceError(message.into())
}
fn validate_windows(windows: &[Window]) -> Result<(), ReduceError> {
    if windows.iter().filter(|w| w.is_focused).count() > 1 {
        return Err(error("multiple seat-focused windows"));
    }
    for window in windows {
        validate_layout(&window.layout)?;
    }
    Ok(())
}
fn validate_layout(layout: &WindowLayout) -> Result<(), ReduceError> {
    let finite = [
        layout.tile_size.0,
        layout.tile_size.1,
        layout.window_offset_in_tile.0,
        layout.window_offset_in_tile.1,
    ]
    .into_iter()
    .chain(
        layout
            .tile_pos_in_workspace_view
            .into_iter()
            .flat_map(|p| [p.0, p.1]),
    )
    .all(f64::is_finite);
    if !finite
        || layout.tile_size.0 < 0.0
        || layout.tile_size.1 < 0.0
        || layout.window_size.0 < 0
        || layout.window_size.1 < 0
        || layout
            .pos_in_scrolling_layout
            .is_some_and(|(col, row)| col == 0 || row == 0)
    {
        return Err(error("invalid native window geometry"));
    }
    Ok(())
}
fn workspace_key(scope: &Scope, id: u64) -> WorkspaceKey {
    WorkspaceKey {
        scope: scope.clone(),
        native: NativeId::Number(id),
    }
}
fn project_window(w: &Window, scope: &Scope) -> WindowView {
    let workspace = w.workspace_id.map(|id| workspace_key(scope, id));
    let group = if !w.is_floating {
        workspace
            .clone()
            .zip(w.layout.pos_in_scrolling_layout)
            .map(|(workspace, (col, _))| GroupKey {
                workspace,
                position: NativeId::Number(col as u64),
            })
    } else {
        None
    };
    WindowView {
        key: WindowKey {
            scope: scope.clone(),
            native: NativeId::Number(w.id),
        },
        workspace,
        group,
        placement: if w.is_floating {
            Placement::Floating
        } else {
            Placement::Tiled
        },
        title: w.title.clone(),
        app_id: w.app_id.clone(),
        pid: w.pid,
        active: w.is_focused,
        geometry: Geometry {
            content_size: Some(Size {
                width: w.layout.window_size.0 as f64,
                height: w.layout.window_size.1 as f64,
            }),
            tile_size: Some(Size {
                width: w.layout.tile_size.0,
                height: w.layout.tile_size.1,
            }),
            tile_position: w
                .layout
                .tile_pos_in_workspace_view
                .map(|(x, y)| Point { x, y }),
            window_offset_in_tile: Some(Point {
                x: w.layout.window_offset_in_tile.0,
                y: w.layout.window_offset_in_tile.1,
            }),
            target_position: None,
        },
    }
}
fn group_bounds<'a>(windows: impl Iterator<Item = &'a WindowView>) -> Option<Rect> {
    let boxes = windows
        .map(|w| w.geometry.tile_position.zip(w.geometry.tile_size))
        .collect::<Option<Vec<_>>>()?;
    let left = boxes.iter().map(|(p, _)| p.x).reduce(f64::min)?;
    let top = boxes.iter().map(|(p, _)| p.y).reduce(f64::min)?;
    let right = boxes.iter().map(|(p, s)| p.x + s.width).reduce(f64::max)?;
    let bottom = boxes.iter().map(|(p, s)| p.y + s.height).reduce(f64::max)?;
    Some(Rect {
        origin: Point { x: left, y: top },
        size: Size {
            width: right - left,
            height: bottom - top,
        },
    })
}
