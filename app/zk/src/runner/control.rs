use super::bridge::{self, Context};
use crate::config::LongcatConfig;
use serde_json::{Value, json};
use std::{collections::BTreeMap, io};
use zk_desktop::{ConditionalAllocation, DesktopCapture, GroupKey, WindowKey};
use zk_longcat::{CommandOutcome, Decision, Episode, Phase, Plan, Policy};

pub struct Slot {
    pub episode: Episode,
    context: Context,
    native: Option<NativeEpisode>,
}

struct NativeEpisode {
    baseline: ConditionalAllocation,
    accepted: ConditionalAllocation,
    previous_basis: Option<std::sync::Arc<[u8]>>,
    awaiting_basis: bool,
}

#[derive(Clone)]
pub struct Sequence {
    pub key: GroupKey,
    pub plan: Plan,
    pub next: usize,
    pub group_submission: bool,
}

pub struct Control {
    pub slots: BTreeMap<GroupKey, Slot>,
    pub dry_run: bool,
    pub stopping: bool,
    minimum: u32,
    resize_debounce_ms: u64,
    policy: Policy,
}

impl Control {
    pub fn new(config: &LongcatConfig, dry_run: bool, timeout: u64) -> Self {
        Self {
            slots: BTreeMap::new(),
            dry_run,
            stopping: false,
            minimum: config.minimum_height as u32,
            resize_debounce_ms: config.resize_debounce_ms,
            policy: Policy {
                active_share: config.active_height_proportion
                    / (config.active_height_proportion + config.other_height_proportion),
                convergence_timeout_ms: timeout,
                ..Policy::default()
            },
        }
    }

    pub fn lose(&mut self, reason: &str) {
        for slot in self.slots.values_mut() {
            slot.episode.lose(reason);
        }
    }

    /// One bounded pass over at most 32 retained episodes. Lost/conflicted episodes
    /// are retained, never silently rebaselined or used as restoration authority.
    pub fn evaluate(
        &mut self,
        capture: &DesktopCapture,
        now: u64,
        in_flight: Option<&Sequence>,
        reason: &str,
    ) -> io::Result<(Value, Option<Sequence>)> {
        let mut unsupported = None;
        if !capture.is_ready() {
            unsupported = Some("waiting for current windows and workspaces families".to_string());
        } else if !self.stopping {
            if let Some(active) = capture
                .active_window
                .as_ref()
                .and_then(|id| capture.windows.get(id))
            {
                if let Some(key) = &active.group {
                    if !self.slots.contains_key(key) {
                        match bridge::observation(capture, key, self.minimum, None) {
                            Ok((observation, context)) => {
                                let overlapping_effect = self.slots.values().any(|slot| {
                                    slot.episode.phase() != Phase::Restored
                                        && slot
                                            .episode
                                            .effects()
                                            .iter()
                                            .any(|effect| effect.may_have_effect())
                                        && slot.episode.baseline().observation.group.source
                                            == observation.group.source
                                        && slot.episode.baseline().observation.members.iter().any(
                                            |member| {
                                                observation
                                                    .members
                                                    .iter()
                                                    .any(|new| new.id == member.id)
                                            },
                                        )
                                });
                                if overlapping_effect {
                                    unsupported = Some("members overlap an episode with retained possible effects; restart/reacquire explicitly, no automatic rebaseline".into());
                                } else if self.slots.len() >= 32 {
                                    return Err(bridge::invalid(
                                        "episode retention limit (32) reached; restart explicitly",
                                    ));
                                } else {
                                    let native = capture.allocations.get(key).cloned();
                                    let created = if let Some(allocation) = &native {
                                        if observation.members.iter().zip(&allocation.members).any(
                                            |(m, a)| {
                                                (m.extent - f64::from(a.target_content)).abs()
                                                    > f64::from(self.policy.geometry_tolerance)
                                            },
                                        ) {
                                            unsupported = Some("waiting for native baseline target/committed convergence".into());
                                            None
                                        } else {
                                            let targets = observation
                                                .members
                                                .iter()
                                                .zip(&allocation.members)
                                                .map(|(m, a)| zk_longcat::Target {
                                                    member: m.id.clone(),
                                                    extent: a.target_content,
                                                })
                                                .collect();
                                            Some(Episode::new_group(
                                                observation,
                                                self.policy,
                                                targets,
                                            ))
                                        }
                                    } else {
                                        Some(Episode::new(observation, self.policy))
                                    };
                                    if let Some(created) = created {
                                        match created {
                                            Ok(episode) => {
                                                self.slots.insert(
                                                    key.clone(),
                                                    Slot {
                                                        episode,
                                                        context,
                                                        native: native.map(|baseline| {
                                                            NativeEpisode {
                                                                accepted: baseline.clone(),
                                                                baseline,
                                                                previous_basis: None,
                                                                awaiting_basis: false,
                                                            }
                                                        }),
                                                    },
                                                );
                                            }
                                            Err(error) => unsupported = Some(error.to_string()),
                                        }
                                    }
                                }
                            }
                            Err(error) => unsupported = Some(error),
                        }
                    }
                } else {
                    unsupported = Some("native grouping/secondary axis unavailable for active window; no geometry-based topology is fabricated".into());
                }
            } else if capture.active_window.is_some() {
                unsupported = Some("active window is not present in the current census".into());
            } else if !capture.capabilities.reported_groups {
                unsupported =
                    Some("native grouping/secondary axis unavailable; no active window".into());
            }
        }
        let mut rows = Vec::new();
        let mut selected = None;
        for (key, slot) in &mut self.slots {
            if let Some(native) = &mut slot.native {
                let pending = in_flight.is_some_and(|sequence| &sequence.key == key)
                    && slot
                        .episode
                        .requested()
                        .is_some_and(|effect| !effect.execution_finished);
                if !pending {
                    match capture.allocations.get(key) {
                        Some(current) if current.basis == native.accepted.basis => {
                            native.awaiting_basis = false;
                            native.previous_basis = None;
                        }
                        Some(current)
                            if native.awaiting_basis
                                && native.previous_basis.as_ref() == Some(&current.basis) =>
                        {
                            if slot
                                .episode
                                .next_reconsideration()
                                .is_some_and(|due| now >= due)
                            {
                                slot.episode
                                    .lose("accepted native basis was not observed before deadline");
                            } else {
                                rows.push(json!({"phase":slot.episode.phase(),"waiting":"accepted native basis observation"}));
                                continue;
                            }
                        }
                        _ => {
                            slot.episode.lose("native basis changed independently; no automatic rebasing or restoration");
                        }
                    }
                }
            }
            let budget = slot.episode.baseline().observation.available_extent;
            let input = bridge::observation(capture, key, self.minimum, Some(budget));
            let observed;
            let decision;
            let can_record = !self.dry_run
                && selected.is_none()
                && in_flight.is_none_or(|sequence| &sequence.key == key);
            let preview = !can_record;
            match input {
                Ok((input, context)) if slot.context.compatible(&context) => {
                    observed = Some(input.clone());
                    if can_record {
                        if slot.episode.effects().len() >= 64 {
                            return Err(bridge::invalid(
                                "episode effect retention limit (64) reached; stopping without automatic pruning/rebaseline",
                            ));
                        }
                        decision = if self.stopping {
                            slot.episode.request_restore(input, now)
                        } else {
                            slot.episode.decide(input, now)
                        };
                    } else {
                        decision = slot.episode.preview(input, now);
                    }
                }
                Ok((input, _)) => {
                    observed = Some(input);
                    decision = slot.episode.lose("independent layout context changed (output, reported span, content width or decorations)");
                }
                Err(error) => {
                    observed = None;
                    decision = slot.episode.lose(error);
                }
            }
            if !preview && let Decision::Plan(plan) = &decision {
                selected = Some(Sequence {
                    key: key.clone(),
                    plan: (**plan).clone(),
                    next: 0,
                    group_submission: slot.native.is_some(),
                });
            }
            rows.push(json!({ "observed": observed, "baseline": slot.episode.baseline(),
                "desired": decision, "preview": preview, "phase": slot.episode.phase(),
                "requested_effect": slot.episode.requested(), "retained_effect_count": slot.episode.effects().len() }));
        }
        if !self.dry_run
            && !capture.capabilities.height_by_id
            && !capture.capabilities.atomic_multiwindow
            && selected.is_some()
        {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "backend has no by-ID height capability",
            ));
        }
        let windows: Vec<_> = capture.windows.values().take(128).map(|window| json!({
            "id":bridge::native_id(&window.key.native),
            "workspace":window.workspace.as_ref().map(|workspace| bridge::native_id(&workspace.native)),
            "group_position":window.group.as_ref().map(|group| bridge::native_id(&group.position)),
            "placement":format!("{:?}", window.placement), "active":window.active,
            "committed_content":window.geometry.content_size.map(|size| json!({"width":size.width,"height":size.height})),
            "tile":window.geometry.tile_size.map(|size| json!({"width":size.width,"height":size.height}))
        })).collect();
        let record = json!({ "type": "longcat", "reason": reason, "version": capture.version.0,
            "source": format!("{:?}:{}", capture.scope.source.backend, capture.scope.source.name), "epoch": capture.scope.epoch.0,
            "ready": capture.is_ready(), "connected": capture.connected, "active_window": capture.active_window.as_ref().map(|w| bridge::native_id(&w.native)),
            "window_count": capture.windows.len(), "workspace_count": capture.workspaces.len(), "dry_run": self.dry_run,
            "policy":self.policy, "resize_learning":{"enabled":false,"configured_debounce_ms":self.resize_debounce_ms},
            "observed_windows":windows, "observed_windows_truncated":capture.windows.len() > 128,
            "family_health":{"windows":format!("{:?}",capture.windows_status.health), "workspaces":format!("{:?}",capture.workspaces_status.health)},
            "unsupported": unsupported, "groups": rows });
        Ok((record, selected))
    }

    /// Fresh capture for each command. Geometry is checked once, identity/context
    /// and focus between commands; self-induced intermediate extents are allowed.
    pub fn request(
        &self,
        capture: &DesktopCapture,
        sequence: &Sequence,
    ) -> Result<super::native::Request, String> {
        let slot = self.slots.get(&sequence.key).ok_or("episode missing")?;
        if matches!(slot.episode.phase(), Phase::Lost | Phase::Conflicted) {
            return Err("episode no longer owns mutation authority".into());
        }
        let (observation, context) = bridge::observation(
            capture,
            &sequence.key,
            self.minimum,
            Some(slot.episode.baseline().observation.available_extent),
        )?;
        if !slot.context.compatible(&context) {
            return Err("independent layout context changed".into());
        }
        let validation = if sequence.next == 0 {
            sequence.plan.validate(&observation)
        } else {
            sequence.plan.validate_identity(&observation)
        };
        validation.map_err(|e| format!("plan admission refused: {e:?}"))?;
        if let Some(native) = &slot.native {
            let current = capture
                .allocations
                .get(&sequence.key)
                .ok_or("native allocation disappeared")?;
            if native.awaiting_basis || current.basis != native.accepted.basis {
                return Err("native basis no longer matches last accepted authority".into());
            }
            let group = capture.groups.get(&sequence.key).ok_or("group missing")?;
            let expected_active = sequence
                .plan
                .active
                .as_ref()
                .map(|active| {
                    group
                        .members
                        .iter()
                        .find(|id| bridge::native_id(&id.native) == active.id)
                        .cloned()
                        .ok_or("active member disappeared")
                })
                .transpose()?;
            return Ok(super::native::Request::Group(
                zk_desktop::GroupAllocationRequest {
                    group: sequence.key.clone(),
                    expected: native.accepted.basis.clone(),
                    restore_payload: (sequence.plan.kind == zk_longcat::PlanKind::Restore)
                        .then(|| native.baseline.restore_payload.clone()),
                    targets: sequence
                        .plan
                        .targets
                        .iter()
                        .map(|target| target.extent)
                        .collect(),
                    edge_gaps: native.baseline.edge_gaps,
                    expected_active,
                },
            ));
        }
        let target = sequence
            .plan
            .targets
            .get(sequence.next)
            .ok_or("sequence finished")?;
        let key: WindowKey = capture
            .groups
            .get(&sequence.key)
            .ok_or("group missing")?
            .members
            .iter()
            .find(|id| bridge::native_id(&id.native) == target.member)
            .ok_or("target missing")?
            .clone();
        let height =
            i32::try_from(target.extent).map_err(|_| "height exceeds niri integer range")?;
        Ok(super::native::Request::Height(
            super::native::HeightRequest {
                target: key,
                basis: capture.version,
                height,
            },
        ))
    }

    pub fn record(&mut self, sequence: &Sequence, outcome: CommandOutcome) -> io::Result<()> {
        let slot = self
            .slots
            .get_mut(&sequence.key)
            .ok_or_else(|| bridge::invalid("episode missing"))?;
        let uncertain =
            sequence.group_submission && matches!(outcome, CommandOutcome::PossiblySent { .. });
        let result = if sequence.group_submission {
            slot.episode.record_group(sequence.plan.id, outcome)
        } else {
            slot.episode
                .record_command(sequence.plan.id, sequence.next, outcome)
        };
        result.map_err(|e| bridge::invalid(format!("command evidence: {e:?}")))?;
        if uncertain {
            slot.episode
                .lose("uncertain native group transmission; authority revoked without replay");
        }
        Ok(())
    }

    pub fn accept_group(
        &mut self,
        sequence: &Sequence,
        accepted: ConditionalAllocation,
    ) -> io::Result<()> {
        let slot = self
            .slots
            .get_mut(&sequence.key)
            .ok_or_else(|| bridge::invalid("episode missing"))?;
        let native = slot
            .native
            .as_mut()
            .ok_or_else(|| bridge::invalid("native group episode missing"))?;
        if accepted.context != native.baseline.context
            || accepted
                .members
                .iter()
                .map(|m| m.target_content)
                .collect::<Vec<_>>()
                != sequence
                    .plan
                    .targets
                    .iter()
                    .map(|t| t.extent)
                    .collect::<Vec<_>>()
        {
            return Err(bridge::invalid("native receipt context/targets mismatch"));
        }
        native.previous_basis = Some(native.accepted.basis.clone());
        native.accepted = accepted;
        native.awaiting_basis = true;
        self.record(sequence, CommandOutcome::Handled)
    }

    pub fn finish(&mut self, sequence: &Sequence) -> io::Result<()> {
        self.slots
            .get_mut(&sequence.key)
            .ok_or_else(|| bridge::invalid("episode missing"))?
            .episode
            .finish_execution(sequence.plan.id)
            .map_err(|e| bridge::invalid(format!("finish execution: {e:?}")))
    }

    pub fn next_deadline(&self) -> Option<u64> {
        self.slots
            .values()
            .filter_map(|slot| slot.episode.next_reconsideration())
            .min()
    }
}
