use super::bridge::{self, Context};
use crate::config::LongcatConfig;
use serde_json::{Value, json};
use std::{collections::BTreeMap, io};
use zk_desktop::{DesktopCapture, GroupKey, WindowKey};
use zk_longcat::{CommandOutcome, Decision, Episode, Phase, Plan, Policy};

pub struct Slot {
    pub episode: Episode,
    context: Context,
}

#[derive(Clone)]
pub struct Sequence {
    pub key: GroupKey,
    pub plan: Plan,
    pub next: usize,
}

pub struct Control {
    pub slots: BTreeMap<GroupKey, Slot>,
    pub dry_run: bool,
    pub stopping: bool,
    minimum: u32,
    policy: Policy,
}

impl Control {
    pub fn new(config: &LongcatConfig, dry_run: bool, timeout: u64) -> Self {
        Self {
            slots: BTreeMap::new(),
            dry_run,
            stopping: false,
            minimum: config.minimum_height as u32,
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
                                        && slot.episode.effects().iter().any(|effect| {
                                            effect
                                                .commands
                                                .iter()
                                                .any(CommandOutcome::may_have_effect)
                                        })
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
                                    match Episode::new(observation, self.policy) {
                                        Ok(episode) => {
                                            self.slots
                                                .insert(key.clone(), Slot { episode, context });
                                        }
                                        Err(error) => unsupported = Some(error.to_string()),
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
            } else if capture.capabilities.reported_groups == false {
                unsupported =
                    Some("native grouping/secondary axis unavailable; no active window".into());
            }
        }
        let mut rows = Vec::new();
        let mut selected = None;
        for (key, slot) in &mut self.slots {
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
                });
            }
            rows.push(json!({ "observed": observed, "baseline": slot.episode.baseline(),
                "desired": decision, "preview": preview, "phase": slot.episode.phase(),
                "requested_effect": slot.episode.requested(), "retained_effect_count": slot.episode.effects().len() }));
        }
        if !self.dry_run && !capture.capabilities.height_by_id && selected.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "backend has no by-ID height capability",
            ));
        }
        let record = json!({ "type": "longcat", "reason": reason, "version": capture.version.0,
            "source": format!("{:?}:{}", capture.scope.source.backend, capture.scope.source.name), "epoch": capture.scope.epoch.0,
            "ready": capture.is_ready(), "connected": capture.connected, "active_window": capture.active_window.as_ref().map(|w| bridge::native_id(&w.native)),
            "window_count": capture.windows.len(), "workspace_count": capture.workspaces.len(), "dry_run": self.dry_run,
            "unsupported": unsupported, "groups": rows });
        Ok((record, selected))
    }

    /// Fresh capture for each command. Geometry is checked once, identity/context
    /// and focus between commands; self-induced intermediate extents are allowed.
    pub fn request(
        &self,
        capture: &DesktopCapture,
        sequence: &Sequence,
    ) -> Result<zk_niri::HeightRequest, String> {
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
        Ok(zk_niri::HeightRequest {
            target: key,
            basis: capture.version,
            height,
        })
    }

    pub fn record(&mut self, sequence: &Sequence, outcome: CommandOutcome) -> io::Result<()> {
        self.slots
            .get_mut(&sequence.key)
            .ok_or_else(|| bridge::invalid("episode missing"))?
            .episode
            .record_command(sequence.plan.id, sequence.next, outcome)
            .map_err(|e| bridge::invalid(format!("command evidence: {e:?}")))
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
