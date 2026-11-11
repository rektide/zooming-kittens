use kubeless_view_impact::affected_roots_with;
use std::{sync::Arc, time::Duration};
use umbriel_client::Observation;
use umbriel_protocol::envelope::Family;
use umbriel_state::inventory::Status;
use umbriel_tracking::{Tracker, Update};
use zk_desktop::*;

pub(crate) const STOCK_FAMILIES: &[Family] =
    &[Family::Windows, Family::Workspaces, Family::Outputs];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReduceError(pub String);
impl std::fmt::Display for ReduceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ReduceError {}

/// Tracker/Inventory are the sole owners of typed native rows and receipts.
/// The three latest full families are a local join, never an atomic snapshot.
pub struct Reducer {
    tracker: Tracker,
    current: Arc<DesktopCapture>,
}
impl Reducer {
    pub fn new(source: SourceId) -> Self {
        assert_eq!(source.backend, Backend::Umbriel);
        let mut initial = DesktopCapture::empty(
            Scope {
                source,
                epoch: Epoch(0),
            },
            Capabilities::READ_ONLY,
        );
        initial.outputs_status = Some(FamilyStatus::default());
        Self {
            tracker: Tracker::new(STOCK_FAMILIES, true),
            current: Arc::new(initial),
        }
    }
    pub fn capture(&self) -> Arc<DesktopCapture> {
        self.current.clone()
    }

    pub fn loss(&mut self, reason: LossReason) -> DesktopUpdate {
        self.tracker.lost();
        let mut update = self.commit(Continuity::Discontinuous(reason.clone()));
        let mut after = (*update.after).clone();
        after.windows_status.stale(reason.clone());
        after.workspaces_status.stale(reason.clone());
        after
            .outputs_status
            .as_mut()
            .expect("stock outputs")
            .stale(reason);
        update.after = Arc::new(after);
        update.impact = Impact::between(&update.before, &update.after);
        self.current = update.after.clone();
        update
    }

    pub fn apply(
        &mut self,
        now: Duration,
        observation: Observation,
    ) -> Result<Option<DesktopUpdate>, ReduceError> {
        let mut roots = Vec::new();
        let continuity = match self.tracker.observe(now, observation) {
            Update::Ignored => return Ok(None),
            Update::Epoch { .. } => Continuity::Discontinuous(LossReason::EpochChanged),
            Update::Gap { .. } => return Ok(Some(self.loss(LossReason::Disconnected))),
            Update::Rejected { .. } => Continuity::Discontinuous(LossReason::InvalidSnapshot),
            Update::Installed {
                before,
                after,
                mut windows,
                ..
            } => {
                if let Some(changes) = windows.as_mut() {
                    changes.retain_layout_changes();
                    let old_scope = Scope {
                        source: self.current.scope.source.clone(),
                        epoch: before
                            .stamp
                            .map_or(self.current.scope.epoch, |s| Epoch(s.epoch)),
                    };
                    let new_scope = Scope {
                        epoch: Epoch(after.stamp.expect("installed stamp").epoch),
                        ..old_scope.clone()
                    };
                    roots.extend(
                        affected_roots_with(
                            &changes.commit.evidence,
                            |_, w| workspace(&old_scope, w.workspace.as_str()),
                            |_, w| workspace(&new_scope, w.workspace.as_str()),
                        )
                        .keys()
                        .iter()
                        .cloned(),
                    );
                }
                continuity(before)
            }
        };
        let mut update = self.commit(continuity);
        update.impact.workspaces.extend(roots);
        Ok(Some(update))
    }

    fn commit(&mut self, continuity: Continuity) -> DesktopUpdate {
        let native = self.tracker.inventory().snapshot();
        let scope = Scope {
            source: self.current.scope.source.clone(),
            epoch: Epoch(native.epoch),
        };
        let mut after = DesktopCapture::empty(scope.clone(), Capabilities::READ_ONLY);
        after.version = ObservationVersion(
            self.current
                .version
                .0
                .checked_add(1)
                .expect("observation version exhausted"),
        );
        after.connected = native.connected;
        after.windows_status = status(native.windows.status);
        after.workspaces_status = status(native.workspaces.status);
        after.outputs_status = Some(status(native.outputs.status));
        let window_scope = family_scope(&scope, native.windows.status);
        let workspace_scope = family_scope(&scope, native.workspaces.status);
        let output_scope = family_scope(&scope, native.outputs.status);
        after.windows =
            native
                .windows
                .entries()
                .values()
                .map(|w| {
                    let key = window(&window_scope, w.id.as_str());
                    let workspace = workspace(&window_scope, w.workspace.as_str());
                    let placement = if !w.scratchpad.is_empty() {
                        Placement::Scratchpad(w.scratchpad.clone())
                    } else if w.floating {
                        Placement::Floating
                    } else if workspace.is_some() {
                        Placement::Tiled
                    } else {
                        Placement::Unknown
                    };
                    let group = workspace
                        .clone()
                        .zip(w.column)
                        .zip(w.row)
                        .filter(|((_, column), row)| *column >= 0 && *row >= 0)
                        .map(|((workspace, column), _)| GroupKey {
                            workspace,
                            position: NativeId::Number(column as u64),
                        });
                    (
                        key.clone(),
                        WindowView {
                            key,
                            workspace,
                            group,
                            placement,
                            output: w.output.as_ref().filter(|s| !s.is_empty()).map(|name| {
                                OutputKey {
                                    scope: window_scope.clone(),
                                    name: name.clone(),
                                }
                            }),
                            presentation: Presentation {
                                title_bar: w.title_bar,
                                tabbed: w.tabbed,
                                tab_index: w.tab_index,
                                tab_hidden: w.tab_hidden,
                                fullscreen: w.fullscreen,
                                maximized: w.maximized,
                                maximized_to_edges: w.maximized_to_edges,
                                pinned: w.pinned,
                                minimized: w.minimized,
                                visible: w.visible,
                                parent: w
                                    .parent
                                    .as_ref()
                                    .filter(|id| !id.as_str().is_empty())
                                    .map(|id| window(&window_scope, id.as_str())),
                                region: w.region.clone(),
                            },
                            title: Some(w.title.clone()),
                            app_id: Some(w.app_id.clone()),
                            pid: (w.pid >= 0).then_some(w.pid),
                            active: w.active,
                            geometry: Geometry {
                                content_size: Some(Size {
                                    width: w.w as f64,
                                    height: w.h as f64,
                                }),
                                target_position: Some(Point {
                                    x: w.x as f64,
                                    y: w.y as f64,
                                }),
                                ..Geometry::default()
                            },
                        },
                    )
                })
                .collect();
        after.active_window = after
            .windows
            .values()
            .find(|w| w.active)
            .map(|w| w.key.clone());
        after.workspaces = native
            .workspaces
            .entries()
            .values()
            .map(|w| {
                let key =
                    workspace(&workspace_scope, w.id.as_str()).expect("validated workspace ID");
                (
                    key.clone(),
                    WorkspaceView {
                        key,
                        name: Some(w.name.clone()),
                        output: Some(w.output.clone()),
                        active: w.active,
                        focused: w.focused,
                        layout: Some(w.layout.as_str().into()),
                        geometry: None,
                        grouping: Evidence::Unavailable,
                    },
                )
            })
            .collect();
        after.outputs = native
            .outputs
            .entries()
            .values()
            .map(|o| {
                let key = OutputKey {
                    scope: output_scope.clone(),
                    name: o.name.clone(),
                };
                (
                    key.clone(),
                    OutputView {
                        key,
                        enabled: o.enabled,
                        powered: o.powered,
                        position: o.position.map(|p| Point {
                            x: p.x as f64,
                            y: p.y as f64,
                        }),
                        logical_size: o.logical_size.map(|s| Size {
                            width: s.width as f64,
                            height: s.height as f64,
                        }),
                        usable_area: o.usable_area.map(|r| Rect {
                            origin: Point {
                                x: r.x as f64,
                                y: r.y as f64,
                            },
                            size: Size {
                                width: r.width as f64,
                                height: r.height as f64,
                            },
                        }),
                        scale: o.scale,
                        transform: o.transform.clone(),
                    },
                )
            })
            .collect();
        crate::stock::project_groups(&mut after, &native.windows);
        let after = Arc::new(after);
        let update = DesktopUpdate::new(self.current.clone(), after.clone(), continuity);
        self.current = after;
        update
    }
}

fn window(scope: &Scope, id: &str) -> WindowKey {
    WindowKey {
        scope: scope.clone(),
        native: NativeId::Text(id.into()),
    }
}
fn workspace(scope: &Scope, id: &str) -> Option<WorkspaceKey> {
    (!id.is_empty()).then(|| WorkspaceKey {
        scope: scope.clone(),
        native: NativeId::Text(id.into()),
    })
}
fn family_scope(scope: &Scope, status: Status) -> Scope {
    Scope {
        epoch: status.stamp.map_or(scope.epoch, |s| Epoch(s.epoch)),
        ..scope.clone()
    }
}
fn status(native: Status) -> FamilyStatus {
    use umbriel_state::inventory::{Health as H, StaleReason as R};
    FamilyStatus {
        stamp: native.stamp.map(|s| FamilyStamp {
            epoch: Epoch(s.epoch),
            position: s.position,
        }),
        health: match native.health {
            H::Unseen => Health::Unseen,
            H::Current => Health::Current,
            H::Stale(reason) => Health::Stale(match reason {
                R::EpochChanged => LossReason::EpochChanged,
                R::Gap => LossReason::Disconnected,
                R::InvalidSnapshot => LossReason::InvalidSnapshot,
            }),
        },
    }
}
fn continuity(prior: Status) -> Continuity {
    if prior.is_ready() {
        return Continuity::Continuous;
    }
    if prior.stamp.is_none() {
        return Continuity::Initial;
    }
    match status(prior).health {
        Health::Stale(reason) => Continuity::Discontinuous(reason),
        _ => Continuity::Discontinuous(LossReason::InvalidSnapshot),
    }
}
