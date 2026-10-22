//! Native layout authority and allocation compilation. Opaque semantic-model
//! payloads are produced and consumed here; no other adapter interprets them.
use std::{collections::BTreeSet, sync::Arc};
use umbriel_protocol::{codec, model::*};
use zk_desktop::*;

pub(crate) struct Reducer {
    tracker: umbriel_tracking::Tracker,
    current: Arc<DesktopCapture>,
}
impl Reducer {
    pub fn new(source: SourceId) -> Self {
        let mut capture = DesktopCapture::empty(
            Scope {
                source,
                epoch: Epoch(0),
            },
            Capabilities::READ_ONLY,
        );
        capture.control_status = Some(FamilyStatus::default());
        Self {
            tracker: umbriel_tracking::Tracker::new(
                &[umbriel_protocol::envelope::Family::LayoutState],
                false,
            ),
            current: Arc::new(capture),
        }
    }
    pub fn capture(&self) -> Arc<DesktopCapture> {
        self.current.clone()
    }
    pub fn loss(&mut self, reason: LossReason) -> DesktopUpdate {
        self.tracker.lost();
        let mut after = (*self.current).clone();
        after.connected = false;
        after
            .control_status
            .as_mut()
            .expect("control family")
            .stale(reason.clone());
        after.version.0 += 1;
        let after = Arc::new(after);
        let update = DesktopUpdate::new(
            self.current.clone(),
            after.clone(),
            Continuity::Discontinuous(reason),
        );
        self.current = after;
        update
    }
    pub fn apply(
        &mut self,
        now: std::time::Duration,
        observation: umbriel_client::Observation,
    ) -> Result<Option<DesktopUpdate>, crate::ReduceError> {
        use umbriel_tracking::Update;
        let continuity = match self.tracker.observe(now, observation) {
            Update::Ignored => return Ok(None),
            Update::Epoch { .. } => Continuity::Discontinuous(LossReason::EpochChanged),
            Update::Gap { .. } => return Ok(Some(self.loss(LossReason::Disconnected))),
            Update::Rejected { .. } => return Ok(Some(self.loss(LossReason::InvalidSnapshot))),
            Update::Installed { before, .. } => {
                if before.stamp.is_none() {
                    Continuity::Initial
                } else {
                    Continuity::Continuous
                }
            }
        };
        let snapshot = self.tracker.inventory().snapshot();
        let status = snapshot.layout.status;
        let mut after = DesktopCapture::empty(
            Scope {
                source: self.current.scope.source.clone(),
                epoch: Epoch(snapshot.epoch),
            },
            Capabilities {
                reported_groups: true,
                height_by_id: false,
                atomic_multiwindow: true,
                native_revision_precondition: false,
                exact_restore: true,
            },
        );
        after.version.0 = self.current.version.0 + 1;
        after.connected = snapshot.connected;
        let health = if status.is_ready() {
            Health::Current
        } else if status.stamp.is_some() {
            Health::Stale(LossReason::EpochChanged)
        } else {
            Health::Unseen
        };
        after.control_status = Some(FamilyStatus {
            stamp: status.stamp.map(|s| FamilyStamp {
                epoch: Epoch(s.epoch),
                position: s.position,
            }),
            health,
        });
        if status.is_ready()
            && let Some(state) = snapshot.layout.entries().get(&())
        {
            project(&mut after, state).map_err(crate::ReduceError)?;
        }
        let after = Arc::new(after);
        let update = DesktopUpdate::new(self.current.clone(), after.clone(), continuity);
        self.current = after;
        Ok(Some(update))
    }
}

fn encode(value: &impl serde::Serialize) -> Result<Arc<[u8]>, String> {
    let mut bytes = Vec::new();
    codec::encode(value, &mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes.into())
}

pub fn allocation(column: &LayoutColumn) -> Result<ConditionalAllocation, String> {
    let basis = &column.basis;
    if !basis.supported
        || basis.tabbed
        || basis.members.is_empty()
        || basis.members.len() > 128
        || column.geometry.len() != basis.members.len()
        || basis.weights.rows.len() != basis.members.len()
        || basis.instance.is_empty()
        || basis.workspace.is_empty()
        || basis.output.is_empty()
        || basis.column < 0
        || column.top_gap < 0
        || column.bottom_gap < 0
        || column.row_budget <= 0
        || i64::from(column.cross_span)
            != i64::from(column.row_budget)
                + i64::from(column.top_gap)
                + i64::from(column.bottom_gap)
        || basis
            .weights
            .rows
            .iter()
            .any(|v| !v.is_finite() || *v <= 0.0)
        || !basis.weights.top.is_finite()
        || basis.weights.top < 0.0
        || !basis.weights.bottom.is_finite()
        || basis.weights.bottom < 0.0
    {
        return Err("invalid supported native column".into());
    }
    let vertical = basis.axis == SecondaryAxis::Vertical;
    let mut members = Vec::new();
    let mut ids = BTreeSet::new();
    let mut decorations = 0u32;
    for (member, geometry) in basis.members.iter().zip(&column.geometry) {
        if member.id.is_empty()
            || !ids.insert(&member.id)
            || geometry.id != member.id
            || member.fullscreen
            || member.maximized
            || member.maximized_to_edges
            || member.floating
            || member.title_bar_height < 0
            || geometry.committed_content.width < 0
            || geometry.committed_content.height < 0
        {
            return Err("invalid native member".into());
        }
        let decoration = if vertical {
            member.title_bar_height as u32
        } else {
            0
        };
        let c = &member.constraints;
        let (minimum, maximum, target) = if vertical {
            (c.min_height, c.max_height, geometry.target_content.height)
        } else {
            (c.min_width, c.max_width, geometry.target_content.width)
        };
        if minimum < 0 || maximum < 0 || target <= 0 {
            return Err("invalid native bounds/target".into());
        }
        let minimum = (minimum as u32).saturating_sub(decoration).max(1);
        let maximum = if maximum == 0 {
            None
        } else {
            Some(
                (maximum as u32)
                    .checked_sub(decoration)
                    .ok_or("decoration exceeds maximum")?,
            )
        };
        if maximum.is_some_and(|max| max < minimum) {
            return Err("inverted native bounds".into());
        }
        decorations = decorations
            .checked_add(decoration)
            .ok_or("decoration overflow")?;
        members.push(AllocationMember {
            target_content: target as u32,
            decoration,
            minimum,
            maximum,
        });
    }
    let content_budget = (column.row_budget as u32)
        .checked_sub(decorations)
        .ok_or("decorations exceed row budget")?;
    let mut context = basis.clone();
    context.weights = LayoutWeights {
        rows: Vec::new(),
        top: 0.0,
        bottom: 0.0,
    };
    Ok(ConditionalAllocation {
        basis: encode(basis)?,
        context: encode(&context)?,
        restore_payload: encode(&basis.weights)?,
        content_budget,
        members,
        edge_gaps: (column.top_gap as u32, column.bottom_gap as u32),
    })
}

pub(crate) fn project(capture: &mut DesktopCapture, state: &LayoutState) -> Result<(), String> {
    if state.version != 1
        || state.instance.is_empty()
        || state.active.as_ref().is_some_and(|id| id.is_empty())
        || state.columns.len() > 512
    {
        return Err("unsupported/invalid layout-state version or size".into());
    }
    let scope = capture.scope.clone();
    capture.active_window = state.active.as_ref().map(|id| WindowKey {
        scope: scope.clone(),
        native: NativeId::Text(id.clone()),
    });
    let mut all_ids = BTreeSet::new();
    for column in &state.columns {
        let basis = &column.basis;
        if basis.instance != state.instance {
            return Err("mixed native instances".into());
        }
        // Unsupported groups remain outside the controller; their active ID is
        // still a coherent outside-group focus observation.
        if !basis.supported {
            continue;
        }
        let evidence = allocation(column)?;
        let workspace = WorkspaceKey {
            scope: scope.clone(),
            native: NativeId::Text(basis.workspace.clone()),
        };
        let key = GroupKey {
            workspace: workspace.clone(),
            position: NativeId::Number(basis.column as u64),
        };
        let mut members = Vec::new();
        for (member, geometry) in basis.members.iter().zip(&column.geometry) {
            if !all_ids.insert(member.id.clone()) {
                return Err("member appears in multiple groups".into());
            }
            let id = WindowKey {
                scope: scope.clone(),
                native: NativeId::Text(member.id.clone()),
            };
            members.push(id.clone());
            capture.windows.insert(
                id.clone(),
                WindowView {
                    key: id.clone(),
                    workspace: Some(workspace.clone()),
                    group: Some(key.clone()),
                    placement: Placement::Tiled,
                    title: None,
                    app_id: None,
                    pid: None,
                    active: capture.active_window.as_ref() == Some(&id),
                    geometry: Geometry {
                        content_size: Some(Size {
                            width: geometry.committed_content.width as f64,
                            height: geometry.committed_content.height as f64,
                        }),
                        tile_size: Some(Size {
                            width: geometry.target_tile.width as f64,
                            height: geometry.target_tile.height as f64,
                        }),
                        target_position: Some(Point {
                            x: geometry.target_content.x as f64,
                            y: geometry.target_content.y as f64,
                        }),
                        ..Geometry::default()
                    },
                },
            );
        }
        capture.workspaces.insert(
            workspace.clone(),
            WorkspaceView {
                key: workspace,
                name: None,
                output: Some(basis.output.clone()),
                active: false,
                focused: false,
                layout: Some("scrolling".into()),
                geometry: None,
                grouping: Evidence::Reported,
            },
        );
        if capture
            .groups
            .insert(
                key.clone(),
                GroupView {
                    key: key.clone(),
                    members,
                    secondary_axis: if basis.axis == SecondaryAxis::Vertical {
                        Axis::Vertical
                    } else {
                        Axis::Horizontal
                    },
                    membership: Evidence::Reported,
                    bounds: None,
                    geometry_evidence: Evidence::Reported,
                },
            )
            .is_some()
        {
            return Err("duplicate native column".into());
        }
        capture.allocations.insert(key, evidence);
    }
    Ok(())
}

pub fn compile(request: &GroupAllocationRequest) -> Result<LayoutAllocation, String> {
    let basis: LayoutBasis = codec::decode(&request.expected).map_err(|e| e.to_string())?;
    if request.group.workspace.scope.source.backend != Backend::Umbriel
        || request.group.workspace.native != NativeId::Text(basis.workspace.clone())
        || request.group.position != NativeId::Number(basis.column as u64)
        || basis.axis != SecondaryAxis::Vertical
        || !basis.supported
        || request.targets.len() != basis.members.len()
    {
        return Err("native allocation target/context mismatch".into());
    }
    let expected_active = request
        .expected_active
        .as_ref()
        .map(|id| match &id.native {
            NativeId::Text(value)
                if id.scope == request.group.workspace.scope
                    && basis.members.iter().any(|m| &m.id == value) =>
            {
                Ok(value.clone())
            }
            _ => Err("active target is not a native group member".to_string()),
        })
        .transpose()?;
    let target = expected_active
        .clone()
        .unwrap_or_else(|| basis.members[0].id.clone());
    let weights = if let Some(payload) = &request.restore_payload {
        codec::decode(payload).map_err(|e| e.to_string())?
    } else {
        LayoutWeights {
            rows: request
                .targets
                .iter()
                .zip(&basis.members)
                .map(|(extent, member)| f64::from(*extent) + f64::from(member.title_bar_height))
                .collect(),
            top: request.edge_gaps.0 as f64,
            bottom: request.edge_gaps.1 as f64,
        }
    };
    let target_content_extents = Some(
        request
            .targets
            .iter()
            .map(|v| i32::try_from(*v).map_err(|_| "target overflow".to_string()))
            .collect::<Result<_, _>>()?,
    );
    Ok(LayoutAllocation {
        target,
        expected: basis,
        weights,
        expected_active,
        target_content_extents,
    })
}

#[derive(Debug)]
pub enum Transmission {
    Handled(ConditionalAllocation),
    Unsent(String),
    Rejected(String),
    PossiblySent(String),
}

pub async fn available(endpoint: &umbriel_client::Endpoint) -> Result<bool, umbriel_client::Error> {
    match endpoint
        .query::<LayoutState>(layout_state_request(), &umbriel_client::Options::default())
        .await
    {
        Ok(state) if state.version == 1 => Ok(true),
        Ok(_) => Err(umbriel_client::Error::InvalidOptions(
            "unsupported layout-state schema version",
        )),
        Err(umbriel_client::Error::Remote(message))
            if message == "unknown command: layout-state" =>
        {
            Ok(false)
        }
        Err(error) => Err(error),
    }
}

pub async fn execute(
    endpoint: &umbriel_client::Endpoint,
    request: GroupAllocationRequest,
) -> Transmission {
    let wire = match compile(&request) {
        Ok(wire) => wire,
        Err(e) => return Transmission::Unsent(e),
    };
    let result: Result<LayoutAllocationResult, _> = endpoint
        .query(wire.request(), &umbriel_client::Options::default())
        .await;
    match result {
        Ok(reply) => {
            let checked = (|| {
                if reply.version != 1
                    || reply.previous_weights != wire.expected.weights
                    || reply.accepted.basis.weights != wire.weights
                {
                    return Err("invalid native allocation receipt".into());
                }
                let accepted = allocation(&reply.accepted)?;
                let mut expected_context = wire.expected.clone();
                expected_context.weights = LayoutWeights {
                    rows: Vec::new(),
                    top: 0.0,
                    bottom: 0.0,
                };
                if accepted.context != encode(&expected_context)?
                    || accepted
                        .members
                        .iter()
                        .map(|m| m.target_content)
                        .collect::<Vec<_>>()
                        != request.targets
                {
                    return Err("native receipt changed context or targets".into());
                }
                Ok(accepted)
            })();
            match checked {
                Ok(value) => Transmission::Handled(value),
                Err(e) => Transmission::PossiblySent(e),
            }
        }
        Err(umbriel_client::Error::Remote(message)) => Transmission::Rejected(message),
        Err(
            error @ (umbriel_client::Error::Connect(_)
            | umbriel_client::Error::ConnectTimeout
            | umbriel_client::Error::Admission(_)
            | umbriel_client::Error::TimedOut {
                possibly_sent: false,
                ..
            }),
        ) => Transmission::Unsent(error.to_string()),
        Err(error) => Transmission::PossiblySent(error.to_string()),
    }
}
