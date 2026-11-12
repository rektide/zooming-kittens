//! Frozen stock content geometry. Positions are reported targets; target extents
//! below are estimates derived from measured strips and a quiet content budget.
//! Callers own quiet acquisition, monotonic timers, receipts and goal lifetime.
use zk_desktop::*;

pub const POSITION_WAIT_MS: u64 = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MotionId {
    pub attempt: u64,
    pub sequence: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeometryFrame {
    pub version: ObservationVersion,
    pub starts: Vec<f64>,
    pub committed: Vec<f64>,
    /// Model-derived; never a native reported target-size vector.
    pub target_extents: Vec<f64>,
    pub coherent: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PositionResponse {
    Progress {
        id: MotionId,
        version: ObservationVersion,
        overshoot: bool,
    },
    Blocked {
        id: MotionId,
        version: ObservationVersion,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Assessment {
    pub frame: GeometryFrame,
    /// Cumulative prediction identity, including all preceding unresolved commits.
    pub owned: Option<MotionId>,
    pub response: Option<PositionResponse>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimerAction {
    WaitUntil(u64),
    /// Exactly one fresh query per silent edge, through Source::refresh_windows.
    RefreshWindows(MotionId),
    Blocked(MotionId),
    Idle,
}

#[derive(Clone, Debug)]
struct Context {
    group: GroupKey,
    members: Vec<WindowKey>,
    axis: Axis,
    output: OutputView,
    presentation: Vec<Presentation>,
    primary: Vec<(f64, f64)>,
}

/// An unforgeable local admission token. Only a validated frozen model can
/// create it. It is still a local check, not a remote compare-and-swap.
#[derive(Clone, Debug)]
pub struct FrozenAuthority {
    context: Context,
    version: ObservationVersion,
    geometry: Vec<Geometry>,
}

impl FrozenAuthority {
    pub(crate) fn validates(
        &self,
        capture: &DesktopCapture,
        request: &RelativeEdgeRequest,
    ) -> bool {
        request.group == self.context.group
            && request.axis == self.context.axis
            && request.basis == self.version
            && capture.version == self.version
            && self.context.validate(capture, 0.0).is_ok()
            && self
                .context
                .members
                .iter()
                .zip(&self.geometry)
                .all(|(id, geometry)| {
                    capture
                        .windows
                        .get(id)
                        .is_some_and(|w| w.geometry == *geometry)
                })
    }
}

#[derive(Clone, Debug)]
pub struct CompiledEdge {
    pub request: RelativeEdgeRequest,
    pub authority: FrozenAuthority,
    /// Pixel travel intended in the current empirical scale, not a guarantee.
    pub travel: f64,
    boundary: usize,
    before: GeometryFrame,
    calibrated: bool,
}

#[derive(Clone, Debug)]
struct Pending {
    id: MotionId,
    edge: CompiledEdge,
    response: bool,
    deadline_ms: u64,
    refreshed: bool,
    travel: f64,
}

/// One group/context's finite cumulative prediction. Do not reconstruct this
/// model on partial commits, focus switches or native handling replies.
#[derive(Clone, Debug)]
pub struct FrozenGeometry {
    context: Context,
    budget: f64,
    strips: Vec<f64>,
    start: f64,
    end: f64,
    tolerance: f64,
    current: GeometryFrame,
    /// Bounded O(n) late-commit corridors; collapse individually on catch-up.
    commit_ranges: Vec<(f64, f64)>,
    owned: Option<MotionId>,
    pending: Option<Pending>,
    empirical_scale: Option<f64>,
    step_cap: f64,
    last_now_ms: u64,
    valid: bool,
}

impl FrozenGeometry {
    /// The caller must establish relevant geometry quietness and drain previous
    /// unknown effects first. Stock cannot prove that a quiet client has committed.
    pub fn acquire(
        capture: &DesktopCapture,
        key: &GroupKey,
        tolerance: u32,
    ) -> Result<Self, String> {
        let group = capture.groups.get(key).ok_or("missing group")?;
        if group.edge_eligibility != EdgeEligibility::Eligible {
            return Err("quiet acquisition requires an eligible candidate axis".into());
        }
        let axis = group.secondary_axis.ok_or("unknown axis")?;
        let windows: Vec<_> = group
            .members
            .iter()
            .map(|id| &capture.windows[id])
            .collect();
        let output_key = windows[0].output.as_ref().ok_or("missing output")?;
        let output = capture
            .outputs
            .get(output_key)
            .ok_or("missing output")?
            .clone();
        let context = Context {
            group: key.clone(),
            members: group.members.clone(),
            axis,
            output,
            presentation: windows.iter().map(|w| w.presentation.clone()).collect(),
            primary: windows
                .iter()
                .map(|w| components(w, axis).map(|(_, _, p, e)| (p, e)))
                .collect::<Result<_, _>>()?,
        };
        context.validate(capture, 0.0)?;
        let parts: Vec<_> = windows
            .iter()
            .map(|w| components(w, axis))
            .collect::<Result<_, _>>()?;
        let starts: Vec<_> = parts.iter().map(|p| p.0).collect();
        let committed: Vec<_> = parts.iter().map(|p| p.1).collect();
        let strips: Vec<_> = (0..parts.len() - 1)
            .map(|i| starts[i + 1] - starts[i] - committed[i])
            .collect();
        if strips.iter().any(|v| !v.is_finite() || *v < 0.0) {
            return Err("overlapping content cannot establish measured strips".into());
        }
        let budget = committed.iter().sum();
        let start = starts[0];
        let end = starts[starts.len() - 1] + committed[committed.len() - 1];
        let current = GeometryFrame {
            version: capture.version,
            starts,
            target_extents: committed.clone(),
            committed: committed.clone(),
            coherent: true,
        };
        Ok(Self {
            context,
            budget,
            strips,
            start,
            end,
            tolerance: f64::from(tolerance),
            current,
            commit_ranges: committed.iter().map(|v| (*v, *v)).collect(),
            owned: None,
            pending: None,
            empirical_scale: None,
            step_cap: f64::INFINITY,
            last_now_ms: 0,
            valid: true,
        })
    }

    pub fn axis(&self) -> Axis {
        self.context.axis
    }
    pub fn group(&self) -> &GroupKey {
        &self.context.group
    }
    pub fn members(&self) -> &[WindowKey] {
        &self.context.members
    }
    pub fn content_budget(&self) -> f64 {
        self.budget
    }
    pub fn strips(&self) -> &[f64] {
        &self.strips
    }
    pub fn envelope(&self) -> (f64, f64) {
        (self.start, self.end)
    }
    pub fn frame(&self) -> &GeometryFrame {
        &self.current
    }
    pub fn empirical_scale(&self) -> Option<f64> {
        self.empirical_scale
    }
    pub fn is_valid(&self) -> bool {
        self.valid
    }

    /// Position targets corresponding to Taffy's content vector.
    pub fn desired_starts(&self, desired: &[f64]) -> Result<Vec<f64>, String> {
        if desired.len() != self.context.members.len()
            || desired.iter().any(|v| !v.is_finite() || *v <= 0.0)
            || (desired.iter().sum::<f64>() - self.budget).abs() > 0.01
        {
            return Err("desired vector does not conserve frozen content budget".into());
        }
        let mut result = vec![self.start];
        for i in 0..desired.len() - 1 {
            result.push(result[i] + desired[i] + self.strips[i]);
        }
        Ok(result)
    }

    /// Compile one latest donor-ready pixel step. First move is useful but capped
    /// at 16px using output workarea as an estimate; native tiledArea may differ.
    pub fn compile(
        &self,
        capture: &DesktopCapture,
        boundary: usize,
        travel: f64,
    ) -> Result<CompiledEdge, String> {
        if !self.valid
            || capture.version != self.current.version
            || self.pending.as_ref().is_some_and(|p| !p.response)
            || boundary >= self.context.members.len() - 1
            || !travel.is_finite()
            || travel == 0.0
        {
            return Err("invalid, pending or stale frozen step".into());
        }
        self.context.validate(capture, self.tolerance)?;
        // A caller must assess the latest capture, not stamp old prediction facts
        // with a new capture version or mint mixed-time authority independently.
        if self.read_frame(capture)? != self.current {
            return Err("unassessed capture".into());
        }
        let output = self.context.output.usable_area.expect("validated output");
        let initial_scale = match self.context.axis {
            Axis::Vertical => output.size.height,
            Axis::Horizontal => output.size.width,
        };
        let scale = self.empirical_scale.unwrap_or(initial_scale);
        let cap = if self.empirical_scale.is_none() {
            16.0
        } else {
            f64::INFINITY
        };
        let travel = travel
            .abs()
            .min(cap)
            .min(self.step_cap)
            .min(scale * 0.9)
            .copysign(travel);
        let fraction = travel / scale;
        let request = RelativeEdgeRequest {
            group: self.context.group.clone(),
            target: self.context.members[boundary].clone(),
            following: self.context.members[boundary + 1].clone(),
            basis: capture.version,
            axis: self.context.axis,
            fraction,
        };
        let authority = FrozenAuthority {
            context: self.context.clone(),
            version: capture.version,
            geometry: self
                .context
                .members
                .iter()
                .map(|id| capture.windows[id].geometry.clone())
                .collect(),
        };
        crate::edge::prepare_frozen(capture, &request, &authority)?;
        Ok(CompiledEdge {
            request,
            authority,
            travel,
            boundary,
            before: self.current.clone(),
            calibrated: self.empirical_scale.is_some(),
        })
    }

    /// Install BEFORE starting the native future. B supplies the known command
    /// key; B and the application still enforce receipt/goal/submission bounds.
    pub fn begin(&mut self, id: MotionId, edge: &CompiledEdge, now_ms: u64) -> Result<(), String> {
        self.clock(now_ms)?;
        if !self.valid
            || edge.before != self.current
            || edge.authority.context.group != self.context.group
            || self.pending.as_ref().is_some_and(|p| !p.response)
            || self.owned.is_some_and(|old| {
                id.attempt < old.attempt || id.attempt == old.attempt && id.sequence <= old.sequence
            })
        {
            return Err("stale or outstanding geometry intent".into());
        }
        self.owned = Some(id);
        self.pending = Some(Pending {
            id,
            edge: edge.clone(),
            response: false,
            deadline_ms: now_ms.saturating_add(POSITION_WAIT_MS),
            refreshed: false,
            travel: 0.0,
        });
        Ok(())
    }

    /// Compatible isolated target movement and cumulative late commits are owned.
    /// Any other motion revokes the model; callers yield/reacquire after quietness,
    /// never recalibrate strips from this mixed-time frame.
    pub fn observe(&mut self, capture: &DesktopCapture, now_ms: u64) -> Result<Assessment, String> {
        self.clock(now_ms)?;
        let result = self.assess(capture);
        if result.is_err() {
            self.valid = false;
        }
        result
    }

    fn assess(&mut self, capture: &DesktopCapture) -> Result<Assessment, String> {
        if !self.valid || capture.version < self.current.version {
            return Err("retired/stale geometry model".into());
        }
        self.context.validate(capture, self.tolerance)?;
        let frame = self.read_frame(capture)?;
        let mut response = None;
        let moved = frame
            .starts
            .iter()
            .zip(&self.current.starts)
            .any(|(a, b)| (a - b).abs() > 0.01);
        if moved {
            let pending = self.pending.as_mut().ok_or("unexplained target motion")?;
            let i = pending.edge.boundary;
            let travel = frame.starts[i + 1] - pending.edge.before.starts[i + 1];
            if travel * pending.edge.travel <= 0.0
                || (travel.abs() + 0.01 < pending.travel.abs())
                || frame
                    .starts
                    .iter()
                    .zip(&pending.edge.before.starts)
                    .enumerate()
                    .any(|(j, (a, b))| j != i + 1 && (a - b).abs() > self.tolerance)
            {
                return Err("unrelated, reversing or coupled target motion".into());
            }
            let overshoot = travel.abs() > pending.edge.travel.abs() + self.tolerance;
            for (range, value) in self.commit_ranges.iter_mut().zip(&frame.target_extents) {
                range.0 = range.0.min(*value);
                range.1 = range.1.max(*value);
            }
            if !pending.response {
                response = Some(PositionResponse::Progress {
                    id: pending.id,
                    version: frame.version,
                    overshoot,
                });
                pending.response = true;
                if overshoot && pending.edge.calibrated {
                    self.empirical_scale = None;
                    self.step_cap = (pending.edge.travel.abs() / 2.0).max(1.0);
                } else {
                    let scale = travel / pending.edge.request.fraction;
                    if scale.is_finite()
                        && scale > 0.0
                        && (travel.abs() >= 8.0 || self.empirical_scale.is_none())
                    {
                        self.empirical_scale = Some(scale);
                        self.step_cap = f64::INFINITY;
                    }
                }
            }
            pending.travel = travel;
        }
        for (i, range) in self.commit_ranges.iter_mut().enumerate() {
            let committed = frame.committed[i];
            if committed < range.0 - self.tolerance || committed > range.1 + self.tolerance {
                return Err("committed extent outside cumulative owned prediction".into());
            }
            if (committed - frame.target_extents[i]).abs() <= self.tolerance {
                *range = (frame.target_extents[i], frame.target_extents[i]);
            }
        }
        self.current = frame.clone();
        Ok(Assessment {
            frame,
            owned: self.owned,
            response,
        })
    }

    /// Deadline includes at least two publication pacing intervals. The caller
    /// executes one bounded refresh; arm this again even when no event arrives.
    pub fn on_timer(&mut self, now_ms: u64) -> Result<TimerAction, String> {
        self.clock(now_ms)?;
        let Some(p) = self.pending.as_mut().filter(|p| !p.response) else {
            return Ok(TimerAction::Idle);
        };
        if now_ms < p.deadline_ms {
            return Ok(TimerAction::WaitUntil(p.deadline_ms));
        }
        if !p.refreshed {
            p.refreshed = true;
            p.deadline_ms = now_ms.saturating_add(POSITION_WAIT_MS);
            Ok(TimerAction::RefreshWindows(p.id))
        } else {
            p.response = true;
            Ok(TimerAction::Blocked(p.id))
        }
    }

    pub fn next_deadline_ms(&self) -> Option<u64> {
        self.pending
            .as_ref()
            .filter(|p| !p.response)
            .map(|p| p.deadline_ms)
    }

    fn clock(&mut self, now_ms: u64) -> Result<(), String> {
        if now_ms < self.last_now_ms {
            return Err("geometry clock regressed".into());
        }
        self.last_now_ms = now_ms;
        Ok(())
    }

    fn read_frame(&self, c: &DesktopCapture) -> Result<GeometryFrame, String> {
        let parts: Vec<_> = self
            .context
            .members
            .iter()
            .map(|id| components(&c.windows[id], self.context.axis))
            .collect::<Result<_, _>>()?;
        let starts: Vec<_> = parts.iter().map(|p| p.0).collect();
        let committed: Vec<_> = parts.iter().map(|p| p.1).collect();
        if (starts[0] - self.start).abs() > self.tolerance {
            return Err("outer start changed".into());
        }
        let mut target_extents: Vec<_> = (0..starts.len() - 1)
            .map(|i| starts[i + 1] - starts[i] - self.strips[i])
            .collect();
        target_extents.push(self.budget - target_extents.iter().sum::<f64>());
        if target_extents.iter().any(|v| !v.is_finite() || *v <= 0.0) {
            return Err("invalid model-derived target extent".into());
        }
        let coherent = committed
            .iter()
            .zip(&target_extents)
            .all(|(a, b)| (a - b).abs() <= self.tolerance)
            && (starts[starts.len() - 1] + committed[committed.len() - 1] - self.end).abs()
                <= self.tolerance;
        Ok(GeometryFrame {
            version: c.version,
            starts,
            committed,
            target_extents,
            coherent,
        })
    }
}

impl Context {
    fn validate(&self, c: &DesktopCapture, tolerance: f64) -> Result<(), String> {
        let group = c.groups.get(&self.group).ok_or("group retired")?;
        if !c.is_ready()
            || c.scope.source.backend != Backend::Umbriel
            || self.group.workspace.scope != c.scope
            || c.control_status.is_some()
            || c.outputs_status.is_none()
            || group.membership != Evidence::Reported
            || group.members != self.members
            || !matches!(
                group.edge_eligibility,
                EdgeEligibility::Eligible | EdgeEligibility::AmbiguousAxis
            )
            || group.secondary_axis.is_some_and(|axis| axis != self.axis)
            || c.outputs.get(&self.output.key) != Some(&self.output)
        {
            return Err("frozen topology/output/axis authority retired".into());
        }
        for (i, id) in self.members.iter().enumerate() {
            let w = c.windows.get(id).ok_or("member retired")?;
            let (_, _, start, extent) = components(w, self.axis)?;
            if w.placement != Placement::Tiled
                || !w.presentation.ordinary_untabbed()
                || w.presentation != self.presentation[i]
                || w.output.as_ref() != Some(&self.output.key)
                || w.workspace.as_ref() != Some(&self.group.workspace)
                || w.group.as_ref() != Some(&self.group)
                || (start - self.primary[i].0).abs() > tolerance
                || (extent - self.primary[i].1).abs() > tolerance
            {
                return Err("presentation or primary envelope changed".into());
            }
        }
        Ok(())
    }
}

fn components(w: &WindowView, axis: Axis) -> Result<(f64, f64, f64, f64), String> {
    let p = w
        .geometry
        .target_position
        .ok_or("missing content target position")?;
    let s = w
        .geometry
        .content_size
        .ok_or("missing committed content size")?;
    if ![p.x, p.y, s.width, s.height]
        .into_iter()
        .all(f64::is_finite)
        || s.width <= 0.0
        || s.height <= 0.0
    {
        return Err("invalid content rectangle".into());
    }
    Ok(match axis {
        Axis::Vertical => (p.y, s.height, p.x, s.width),
        Axis::Horizontal => (p.x, s.width, p.y, s.height),
    })
}
