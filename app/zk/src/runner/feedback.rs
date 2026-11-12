//! Stock semantic bridge and geometry/policy composition. No sockets or tasks.
use super::{bridge, native};
use crate::config::LongcatConfig;
use std::{collections::BTreeMap, io};
use zk_desktop as d;
use zk_longcat::{self as p, feedback::*};
use zk_umbriel::geometry::{FrozenGeometry, MotionId, PositionResponse, TimerAction};

fn source(scope: &d::Scope) -> p::Source {
    p::Source {
        id: format!("{:?}:{}", scope.source.backend, scope.source.name),
        epoch: scope.epoch.0,
    }
}

fn key(key: &d::GroupKey, axis: d::Axis) -> p::GroupKey {
    p::GroupKey {
        source: source(&key.workspace.scope),
        workspace: bridge::native_id(&key.workspace.native),
        topology: bridge::native_id(&key.position),
        axis: match axis {
            d::Axis::Vertical => p::Axis::Vertical,
            d::Axis::Horizontal => p::Axis::Horizontal,
        },
    }
}

fn focus(capture: &d::DesktopCapture) -> p::Focus {
    match &capture.active_window {
        None => p::Focus::None,
        Some(id) => match capture.windows.get(id).and_then(|w| w.workspace.as_ref()) {
            Some(ws) => p::Focus::Window(p::WindowKey {
                source: source(&id.scope),
                workspace: bridge::native_id(&ws.native),
                id: bridge::native_id(&id.native),
            }),
            None => p::Focus::Unknown,
        },
    }
}

fn member_context(capture: &d::DesktopCapture, w: &d::WindowView) -> String {
    // No title, activation, row/column ordinal or secondary geometry here.
    format!(
        "{:?}|{:?}|{:?}",
        w.placement,
        w.presentation,
        w.output.as_ref().and_then(|o| capture.outputs.get(o))
    )
}

fn raw_group(
    capture: &d::DesktopCapture,
    group: &d::GroupView,
    axis: d::Axis,
    config: &LongcatConfig,
) -> Option<FeedbackGroup> {
    let minimum = match axis {
        d::Axis::Vertical => config.minimum_height,
        d::Axis::Horizontal => config.minimum_width,
    } as u32;
    let mut members = Vec::new();
    let mut context = format!("{axis:?}");
    for id in &group.members {
        let w = capture.windows.get(id)?;
        let start = w.geometry.target_position?;
        let size = w.geometry.content_size?;
        let (start, extent, primary_start, primary_extent) = match axis {
            d::Axis::Vertical => (start.y, size.height, start.x, size.width),
            d::Axis::Horizontal => (start.x, size.width, start.y, size.height),
        };
        if ![start, extent, primary_start, primary_extent]
            .into_iter()
            .all(f64::is_finite)
            || extent <= 0.0
            || primary_extent <= 0.0
        {
            return None;
        }
        context.push_str(&format!(
            "|{}|{primary_start}|{primary_extent}",
            member_context(capture, w)
        ));
        members.push(FeedbackMember {
            id: bridge::native_id(&id.native),
            start,
            extent,
            minimum_extent: minimum,
            maximum_extent: None,
        });
    }
    Some(FeedbackGroup {
        key: key(&group.key, axis),
        context,
        unavailable: None,
        content_budget: members.iter().map(|m| m.extent).sum(),
        members,
        target_extents: None,
        coherent: false,
        motion: Motion::Unclassified,
    })
}

#[derive(Debug)]
struct Hold {
    focus: p::Focus,
    /// Only target movement releases a coupled-response hold. Late client sizes
    /// for that response are execution evidence, not independent guidance.
    starts: Vec<f64>,
}

#[derive(Debug)]
struct Slot {
    axis: d::Axis,
    members: Vec<d::WindowKey>,
    observed: FeedbackGroup,
    model: Option<FrozenGeometry>,
    last_motion_ms: u64,
    acquire_failed: bool,
    owned: Option<CommandKey>,
    hold: Option<Hold>,
}

pub(super) struct Control {
    pub feedback: Feedback,
    slots: BTreeMap<d::GroupKey, Slot>,
    config: LongcatConfig,
    lost_scope: Option<d::Scope>,
    pub stopping: bool,
    pub assessment: Vec<String>,
    assessment_dropped: u64,
}

fn command_key(id: MotionId) -> CommandKey {
    CommandKey {
        attempt: id.attempt,
        sequence: id.sequence,
    }
}

impl Control {
    pub fn new(config: &LongcatConfig, timeout_ms: u64) -> io::Result<Self> {
        Ok(Self {
            feedback: Feedback::new(FeedbackPolicy {
                active_share: config.active_height_proportion
                    / (config.active_height_proportion + config.other_height_proportion),
                resize_debounce_ms: config.resize_debounce_ms,
                convergence_timeout_ms: timeout_ms,
                ..FeedbackPolicy::default()
            })
            .map_err(io::Error::other)?,
            slots: BTreeMap::new(),
            config: config.clone(),
            lost_scope: None,
            stopping: false,
            assessment: Vec::new(),
            assessment_dropped: 0,
        })
    }

    pub fn lose(&mut self, scope: &d::Scope, reason: &str, now: u64) -> io::Result<()> {
        if self.lost_scope.as_ref() != Some(scope) {
            self.feedback.lose(reason, now).map_err(io::Error::other)?;
            self.lost_scope = Some(scope.clone());
            self.slots.clear();
            self.note(format!(
                "source authority lost: {reason}; fresh epoch required"
            ));
        }
        Ok(())
    }

    pub fn usable(&self, c: &d::DesktopCapture) -> bool {
        c.is_ready() && self.lost_scope.as_ref() != Some(&c.scope)
    }

    pub fn update(&mut self, update: &d::DesktopUpdate, now: u64) -> io::Result<()> {
        let initial = update.before.scope.epoch.0 == 0 && self.feedback.input().is_none();
        // Installing each previously stale family in the fresh epoch may itself
        // carry EpochChanged. Those markers are acquisition, not loss of the new
        // authority before it has even become ready.
        let acquiring_fresh = matches!(
            update.continuity,
            d::Continuity::Discontinuous(d::LossReason::EpochChanged)
        ) && self
            .feedback
            .input()
            .is_none_or(|i| i.source != source(&update.after.scope));
        if matches!(update.continuity, d::Continuity::Discontinuous(_))
            && !initial
            && (!acquiring_fresh
                || self
                    .feedback
                    .input()
                    .is_some_and(|i| i.source == source(&update.before.scope)))
        {
            // Reconnection's Epoch marker revokes the OLD epoch. Its new families
            // must still become current before fresh acquisition/probing.
            let scope = if update.before.scope != update.after.scope {
                &update.before.scope
            } else {
                &update.after.scope
            };
            self.lose(scope, &format!("{:?}", update.continuity), now)?;
        }
        self.install(&update.after, now)
    }

    pub fn install(&mut self, c: &d::DesktopCapture, now: u64) -> io::Result<()> {
        if !c.is_ready() {
            if self
                .feedback
                .input()
                .is_some_and(|i| i.source == source(&c.scope))
                && self.lost_scope.as_ref() != Some(&c.scope)
            {
                self.lose(&c.scope, "required source family unavailable", now)?;
            }
            return Ok(());
        }
        if !self.usable(c) {
            return Ok(());
        }
        if self.lost_scope.is_some() {
            self.slots.clear();
            self.lost_scope = None;
        }
        self.slots.retain(|k, _| c.groups.contains_key(k));
        let current_focus = focus(c);
        let mut groups = Vec::new();
        let mut responses = Vec::new();
        for (native_key, group) in &c.groups {
            if !(2..=128).contains(&group.members.len()) {
                self.slots.remove(native_key);
                continue;
            }
            let axis = group
                .secondary_axis
                .or_else(|| self.slots.get(native_key).map(|s| s.axis));
            let Some(axis) = axis else {
                self.slots.remove(native_key);
                continue;
            };
            let Some(mut raw) = raw_group(c, group, axis, &self.config) else {
                self.slots.remove(native_key);
                continue;
            };
            if !matches!(
                group.edge_eligibility,
                d::EdgeEligibility::Eligible | d::EdgeEligibility::AmbiguousAxis
            ) {
                self.slots.remove(native_key);
                continue;
            }
            let reset = self.slots.get(native_key).is_none_or(|s| {
                s.axis != axis || s.members != group.members || s.observed.context != raw.context
            });
            if reset {
                self.slots.insert(
                    native_key.clone(),
                    Slot {
                        axis,
                        members: group.members.clone(),
                        observed: raw.clone(),
                        model: None,
                        last_motion_ms: now,
                        acquire_failed: false,
                        owned: None,
                        hold: None,
                    },
                );
            }
            let slot = self.slots.get_mut(native_key).expect("slot installed");
            let geometry_changed = slot
                .observed
                .members
                .iter()
                .zip(&raw.members)
                .any(|(a, b)| a.start != b.start || a.extent != b.extent);
            if geometry_changed {
                slot.last_motion_ms = now;
                slot.acquire_failed = false;
            }
            if slot.hold.as_ref().is_some_and(|h| {
                h.focus != current_focus
                    || h.starts
                        .iter()
                        .zip(&raw.members)
                        .any(|(a, b)| (a - b.start).abs() > 2.0)
            }) {
                slot.hold = None;
                slot.model = None;
                slot.acquire_failed = false;
                // A changed focus starts a fresh execution generation, not a
                // manual resize of the failed native result.
                if !geometry_changed {
                    raw.motion = slot.owned.map_or(Motion::Unclassified, Motion::Owned);
                }
            }
            if slot.hold.is_some() {
                raw.content_budget = slot.observed.content_budget;
                raw.target_extents.clone_from(&slot.observed.target_extents);
                raw.motion = slot.owned.map_or(Motion::Unclassified, Motion::Owned);
            } else if let Some(model) = &mut slot.model {
                match model.observe(c, now) {
                    Ok(assessment) => {
                        raw.content_budget = model.content_budget();
                        raw.target_extents = Some(assessment.frame.target_extents);
                        raw.coherent = assessment.frame.coherent;
                        slot.owned = assessment.owned.map(command_key);
                        raw.motion = slot.owned.map_or(Motion::Unclassified, Motion::Owned);
                        if let Some(response) = assessment.response {
                            responses.push(response);
                        }
                    }
                    Err(reason) => {
                        // Compatible movement of the requested boundary together
                        // with other boundaries is a coupled native response, not
                        // a new preferred layout. Reversal/unrelated movement is
                        // independent/ambiguous and instead yields to guidance.
                        let edge = self
                            .feedback
                            .current()
                            .filter(|a| a.goal.inputs.group == raw.key)
                            .and_then(|a| a.commands.last());
                        let coupled = edge.is_some_and(|edge| {
                            let i = edge.intent.boundary;
                            let before = &model.frame().starts;
                            edge.response.is_none()
                                && (raw.members[i + 1].start - before[i + 1]) * edge.intent.travel
                                    > 0.0
                                && raw
                                    .members
                                    .iter()
                                    .zip(before)
                                    .enumerate()
                                    .any(|(j, (m, b))| j != i + 1 && (m.start - b).abs() > 2.0)
                        });
                        let commit_only_execution = slot.owned.is_some()
                            && (edge.is_some() || !slot.observed.coherent)
                            && raw
                                .members
                                .iter()
                                .zip(&model.frame().starts)
                                .all(|(m, b)| (m.start - b).abs() <= 0.01);
                        if coupled || commit_only_execution {
                            if let Some(attempt) = self
                                .feedback
                                .current()
                                .filter(|a| a.goal.inputs.group == raw.key)
                            {
                                let id = attempt.goal.id;
                                self.feedback
                                    .limit_current(
                                        id,
                                        LimitReason::Executor {
                                            reason: format!("coupled native geometry: {reason}"),
                                        },
                                    )
                                    .map_err(io::Error::other)?;
                            }
                            raw.content_budget = slot.observed.content_budget;
                            raw.target_extents.clone_from(&slot.observed.target_extents);
                            raw.motion = slot.owned.map_or(Motion::Unclassified, Motion::Owned);
                            slot.hold = Some(Hold {
                                focus: current_focus.clone(),
                                starts: raw.members.iter().map(|m| m.start).collect(),
                            });
                        } else if let Some(edge) = edge.filter(|e| {
                            (e.outcome.is_none() || e.response.is_none())
                                && e.outcome
                                    .as_ref()
                                    .is_none_or(p::CommandOutcome::may_have_effect)
                        }) {
                            raw.motion = Motion::Unattributed(edge.key);
                        }
                        if self.assessment.len() < 32 {
                            self.assessment.push(reason);
                        } else {
                            self.assessment_dropped += 1;
                        }
                        slot.model = None;
                    }
                }
            }
            // Rebuild only on quiet independent geometry and with no unresolved
            // command receipt. Never rebuild a failed execution merely to retry it.
            if slot.model.is_none()
                && slot.hold.is_none()
                && !slot.acquire_failed
                && self
                    .feedback
                    .current()
                    .is_none_or(|a| a.pending().is_none())
                && now
                    >= slot
                        .last_motion_ms
                        .saturating_add(self.config.resize_debounce_ms)
            {
                match FrozenGeometry::acquire(c, native_key, 2) {
                    Ok(model) => {
                        raw.content_budget = model.content_budget();
                        raw.target_extents = Some(model.frame().target_extents.clone());
                        raw.coherent = true;
                        slot.model = Some(model);
                    }
                    Err(reason) => {
                        slot.acquire_failed = true;
                        if self.assessment.len() < 32 {
                            self.assessment
                                .push(format!("quiet geometry acquisition parked: {reason}"));
                        } else {
                            self.assessment_dropped += 1;
                        }
                    }
                }
            }
            slot.observed = raw.clone();
            groups.push(raw);
        }
        let live_members = c
            .windows
            .values()
            .map(|w| LiveMember {
                id: bridge::native_id(&w.key.native),
                workspace: w.workspace.as_ref().map_or_else(
                    || "outside-native-workspace".into(),
                    |ws| bridge::native_id(&ws.native),
                ),
                context: member_context(c, w),
            })
            .collect();
        self.feedback
            .observe(
                FeedbackInput {
                    source: source(&c.scope),
                    version: c.version.0,
                    live_members,
                    groups,
                    focus: current_focus,
                },
                now,
            )
            .map_err(io::Error::other)?;
        // Install the actual current capture BEFORE B records the position response.
        for response in responses {
            let (id, version, kind) = match response {
                PositionResponse::Progress { id, version, .. } => {
                    (id, version, ResponseKind::Progress)
                }
                PositionResponse::Blocked { id, version } => (
                    id,
                    version,
                    ResponseKind::Blocked {
                        reason: "native position did not progress".into(),
                    },
                ),
            };
            self.response(command_key(id), version.0, kind)?;
        }
        Ok(())
    }

    fn response(&mut self, key: CommandKey, version: u64, kind: ResponseKind) -> io::Result<()> {
        if self.feedback.current().is_some_and(|a| {
            a.commands
                .iter()
                .any(|e| e.key == key && e.response.is_none())
        }) {
            self.feedback
                .record_response(
                    key,
                    EdgeResponse {
                        observed_version: version,
                        kind,
                    },
                )
                .map_err(io::Error::other)?;
        }
        Ok(())
    }

    pub fn timer(
        &mut self,
        c: &d::DesktopCapture,
        now: u64,
        refresh_pending: bool,
    ) -> io::Result<Option<MotionId>> {
        if refresh_pending || !self.usable(c) {
            return Ok(None);
        }
        let mut refresh = None;
        let mut blocked = Vec::new();
        for slot in self.slots.values_mut() {
            if let Some(model) = &mut slot.model {
                match model.on_timer(now).map_err(bridge::invalid)? {
                    TimerAction::RefreshWindows(id) => refresh = Some(id),
                    TimerAction::Blocked(id) => blocked.push(id),
                    _ => {}
                }
            }
        }
        for id in blocked {
            self.response(
                command_key(id),
                c.version.0,
                ResponseKind::Blocked {
                    reason: "no position progress after timed windows refresh".into(),
                },
            )?;
        }
        Ok(refresh)
    }

    pub fn next_deadline(&self, refresh_pending: bool) -> Option<u64> {
        let model = self
            .slots
            .values()
            .filter_map(|s| s.model.as_ref())
            .filter_map(|m| m.next_deadline_ms())
            .filter(|_| !refresh_pending)
            .min();
        let acquire = self
            .slots
            .values()
            .filter(|s| s.model.is_none() && s.hold.is_none() && !s.acquire_failed)
            .filter(|_| {
                self.feedback
                    .current()
                    .is_none_or(|a| a.pending().is_none())
            })
            .map(|s| {
                s.last_motion_ms
                    .saturating_add(self.config.resize_debounce_ms)
            })
            .min();
        self.feedback
            .next_reconsideration()
            .into_iter()
            .chain(model)
            .chain(acquire)
            .min()
    }

    pub fn prepare(
        &mut self,
        c: &d::DesktopCapture,
        goal: &FeedbackGoal,
        now: u64,
    ) -> io::Result<Option<native::Request>> {
        if now >= goal.deadline_ms {
            self.feedback.reconsider(now).map_err(io::Error::other)?;
            return Ok(None);
        }
        let slot = self
            .slots
            .values_mut()
            .find(|s| s.observed.key == goal.inputs.group)
            .ok_or_else(|| bridge::invalid("missing stock goal geometry"))?;
        let Some(model) = &mut slot.model else {
            self.feedback
                .limit_current(
                    goal.id,
                    LimitReason::Executor {
                        reason: "frozen execution geometry unavailable".into(),
                    },
                )
                .map_err(io::Error::other)?;
            return Ok(None);
        };
        // A superseded attempt may still await its position/refresh outcome.
        // A focus change cannot bypass that owned observation obligation.
        if model.next_deadline_ms().is_some() {
            return Ok(None);
        }
        match edges::select_edge(goal, &model.frame().target_extents, self.feedback.current()) {
            edges::EdgeSelection::PositionsMatch => Ok(None),
            edges::EdgeSelection::Limited { reason } => {
                self.feedback
                    .limit_current(goal.id, LimitReason::Executor { reason })
                    .map_err(io::Error::other)?;
                Ok(None)
            }
            edges::EdgeSelection::Step(step) => {
                let compiled = match model.compile(c, step.boundary, step.travel) {
                    Ok(edge) => edge,
                    Err(reason) => {
                        self.feedback
                            .limit_current(goal.id, LimitReason::Executor { reason })
                            .map_err(io::Error::other)?;
                        return Ok(None);
                    }
                };
                let key = self
                    .feedback
                    .begin_command(
                        goal.id,
                        EdgeIntent {
                            basis_version: c.version.0,
                            boundary: step.boundary,
                            travel: compiled.travel,
                        },
                        now,
                    )
                    .map_err(io::Error::other)?;
                if let Err(reason) = model.begin(
                    MotionId {
                        attempt: key.attempt,
                        sequence: key.sequence,
                    },
                    &compiled,
                    now,
                ) {
                    self.feedback
                        .record_outcome(key, p::CommandOutcome::Unsent)
                        .map_err(io::Error::other)?;
                    self.assessment.push(reason);
                    return Ok(None);
                }
                slot.owned = Some(key);
                Ok(Some(native::Request::Edge(Box::new(compiled))))
            }
        }
    }

    pub fn receipt(&mut self, key: CommandKey, outcome: p::CommandOutcome) -> io::Result<()> {
        self.feedback
            .record_outcome(key, outcome)
            .map_err(io::Error::other)
    }

    fn note(&mut self, reason: String) {
        if self.assessment.len() < 32 {
            self.assessment.push(reason);
        } else {
            self.assessment_dropped += 1;
        }
    }

    pub fn restore(&mut self, now: u64) -> io::Result<()> {
        self.stopping = true;
        self.feedback
            .request_restore(now)
            .map_err(io::Error::other)?;
        Ok(())
    }

    pub fn diagnostics(
        &mut self,
        c: &d::DesktopCapture,
        decision: &FeedbackDecision,
        preview: bool,
    ) -> serde_json::Value {
        serde_json::json!({"type":"longcat-feedback", "preview":preview,
            "source":format!("{:?}",c.scope), "version":c.version.0, "ready":self.usable(c),
            "health":format!("windows={:?}, workspaces={:?}, outputs={:?}",c.windows_status,c.workspaces_status,c.outputs_status),
            "capability":"stock qualified internal edges; inferred geometry; no atomic/exact restore",
            "decision":decision, "preferences":self.feedback.preferences(),
            "preference_generation":self.feedback.preference_generation(), "current":self.feedback.current(),
            "guidance":self.feedback.guidance().collect::<Vec<_>>(), "counters":self.feedback.counters(),
            "assessment":std::mem::take(&mut self.assessment),
            "assessment_omitted_count":std::mem::take(&mut self.assessment_dropped),
            "groups":self.slots.values().map(|s| serde_json::json!({"observed":s.observed,
                "axis":format!("{:?}",s.axis),"membership":"reported","geometry":"inferred",
                "strips":s.model.as_ref().map(|m|m.strips()), "empirical_scale":s.model.as_ref().and_then(|m|m.empirical_scale()),
                "coupled_response_hold":s.hold.is_some()})).collect::<Vec<_>>(),
            "observed_count":c.windows.len(), "observed_truncated":c.windows.len()>128,
            "observed":c.windows.values().take(128).map(|w|serde_json::json!({"id":bridge::native_id(&w.key.native),
                "workspace":w.workspace.as_ref().map(|w|bridge::native_id(&w.native)),"geometry":format!("{:?}",w.geometry)})).collect::<Vec<_>>()})
    }
}
