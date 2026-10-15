use kubeless_view_impact::affected_roots_with;
use std::{sync::Arc, time::Duration};
use umbriel_client::Observation;
use umbriel_protocol::{
    envelope::{EventName, Family, Message},
    model::{Event, EventDecode, decode_event},
};
use umbriel_state::inventory::{Inventory, Stamp, Status};
use zk_desktop::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReduceError(pub String);
impl std::fmt::Display for ReduceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ReduceError {}

/// Native Inventory remains the ONLY owner of Umbriel rows/change receipts.
pub struct Reducer {
    inventory: Inventory,
    current: Arc<DesktopCapture>,
}
impl Reducer {
    pub fn new(source: SourceId) -> Self {
        assert_eq!(source.backend, Backend::Umbriel);
        Self {
            inventory: Inventory::new(0),
            current: Arc::new(DesktopCapture::empty(
                Scope {
                    source,
                    epoch: Epoch(0),
                },
                Capabilities::READ_ONLY,
            )),
        }
    }
    pub fn capture(&self) -> Arc<DesktopCapture> {
        self.current.clone()
    }

    pub fn loss(&mut self, reason: LossReason) -> DesktopUpdate {
        self.inventory.gap();
        let mut update = self.commit(Continuity::Discontinuous(reason.clone()));
        // Native inventory records Gap; retain the more precise local cause too.
        let mut after = (*update.after).clone();
        after.windows_status.stale(reason.clone());
        after.workspaces_status.stale(reason);
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
        match observation {
            Observation::Epoch { epoch } => {
                self.inventory
                    .begin_epoch(epoch)
                    .map_err(|e| ReduceError(e.to_string()))?;
                Ok(Some(self.commit(Continuity::Discontinuous(
                    LossReason::EpochChanged,
                ))))
            }
            Observation::Gap { .. } => Ok(Some(self.loss(LossReason::Disconnected))),
            Observation::Frame {
                epoch,
                position,
                bytes,
            } => {
                let message = match Message::decode(&bytes) {
                    Ok(message) => message,
                    Err(_) => return Ok(Some(self.invalid(None))),
                };
                let family = match &message {
                    Message::Event {
                        name: EventName::Known(family),
                        ..
                    } if matches!(family, Family::Windows | Family::Workspaces) => *family,
                    _ => return Ok(None),
                };
                let event = match decode_event(&message) {
                    Ok(EventDecode::Known(event)) => event,
                    _ => return Ok(Some(self.invalid(Some(family)))),
                };
                let stamp = Stamp {
                    epoch,
                    position,
                    observed_at: now,
                };
                match event {
                    Event::Windows(windows) => {
                        if windows.iter().filter(|w| w.active).count() > 1
                            || windows.iter().any(|w| w.w < 0 || w.h < 0)
                        {
                            return Ok(Some(self.invalid(Some(Family::Windows))));
                        }
                        let mut changes =
                            match self.inventory.install_windows_changes(stamp, windows) {
                                Ok(changes) => changes,
                                Err(_) => return Ok(Some(self.invalid(Some(Family::Windows)))),
                            };
                        let continuity = continuity(changes.before, changes.is_continuous());
                        changes
                            .commit
                            .evidence
                            .retain(|row| match (&row.before, &row.after) {
                                (Some(a), Some(b)) => {
                                    a.workspace != b.workspace
                                        || a.scratchpad != b.scratchpad
                                        || a.active != b.active
                                        || a.floating != b.floating
                                        || a.x != b.x
                                        || a.y != b.y
                                        || a.w != b.w
                                        || a.h != b.h
                                }
                                (None, None) => false,
                                _ => true,
                            });
                        let mut old_scope = self.current.scope.clone();
                        if let Some(stamp) = changes.before.stamp {
                            old_scope.epoch = Epoch(stamp.epoch);
                        }
                        let new_scope = Scope {
                            source: self.current.scope.source.clone(),
                            epoch: Epoch(epoch),
                        };
                        let roots = affected_roots_with(
                            &changes.commit.evidence,
                            |_, w| {
                                (!w.workspace.as_str().is_empty()).then(|| WorkspaceKey {
                                    scope: old_scope.clone(),
                                    native: NativeId::Text(w.workspace.as_str().to_owned()),
                                })
                            },
                            |_, w| {
                                (!w.workspace.as_str().is_empty()).then(|| WorkspaceKey {
                                    scope: new_scope.clone(),
                                    native: NativeId::Text(w.workspace.as_str().to_owned()),
                                })
                            },
                        );
                        let mut update = self.commit(continuity);
                        // Before/after semantic comparison already preserves old-epoch keys.
                        update
                            .impact
                            .workspaces
                            .extend(roots.keys().iter().cloned());
                        Ok(Some(update))
                    }
                    Event::Workspaces(workspaces) => {
                        let prior = self.inventory.status(Family::Workspaces);
                        if self
                            .inventory
                            .install(stamp, Event::Workspaces(workspaces))
                            .is_err()
                        {
                            return Ok(Some(self.invalid(Some(Family::Workspaces))));
                        }
                        let continuity = continuity(prior, prior.is_ready());
                        Ok(Some(self.commit(continuity)))
                    }
                    _ => Ok(None),
                }
            }
        }
    }

    fn invalid(&mut self, family: Option<Family>) -> DesktopUpdate {
        for candidate in [Family::Windows, Family::Workspaces] {
            if family.is_none_or(|family| family == candidate) {
                self.inventory.invalid_snapshot(candidate);
            }
        }
        self.commit(Continuity::Discontinuous(LossReason::InvalidSnapshot))
    }
    fn commit(&mut self, continuity: Continuity) -> DesktopUpdate {
        let native = self.inventory.snapshot();
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
        let window_scope = Scope {
            epoch: native
                .windows
                .status
                .stamp
                .map_or(scope.epoch, |s| Epoch(s.epoch)),
            ..scope.clone()
        };
        let workspace_scope = Scope {
            epoch: native
                .workspaces
                .status
                .stamp
                .map_or(scope.epoch, |s| Epoch(s.epoch)),
            ..scope
        };
        after.windows = native
            .windows
            .entries()
            .values()
            .map(|w| {
                let key = WindowKey {
                    scope: window_scope.clone(),
                    native: NativeId::Text(w.id.as_str().to_owned()),
                };
                let workspace = (!w.workspace.as_str().is_empty()).then(|| WorkspaceKey {
                    scope: window_scope.clone(),
                    native: NativeId::Text(w.workspace.as_str().to_owned()),
                });
                let placement = if !w.scratchpad.is_empty() {
                    Placement::Scratchpad(w.scratchpad.clone())
                } else if w.floating {
                    Placement::Floating
                } else if workspace.is_some() {
                    Placement::Tiled
                } else {
                    Placement::Unknown
                };
                (
                    key.clone(),
                    WindowView {
                        key,
                        workspace,
                        group: None,
                        placement,
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
                let key = WorkspaceKey {
                    scope: workspace_scope.clone(),
                    native: NativeId::Text(w.id.as_str().to_owned()),
                };
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
        let after = Arc::new(after);
        let update = DesktopUpdate::new(self.current.clone(), after.clone(), continuity);
        self.current = after;
        update
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

fn continuity(prior: Status, continuous: bool) -> Continuity {
    if continuous {
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
