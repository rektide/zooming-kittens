use std::io;
use zk_desktop as desktop;
use zk_longcat as policy;

pub fn native_id(id: &desktop::NativeId) -> String {
    match id {
        desktop::NativeId::Number(id) => format!("number:{id}"),
        desktop::NativeId::Text(id) => format!("text:{id}"),
    }
}

fn source(scope: &desktop::Scope) -> policy::Source {
    policy::Source {
        id: format!("{:?}:{}", scope.source.backend, scope.source.name),
        epoch: scope.epoch.0,
    }
}

/// Only independent context signals are compared. Inferred bounding extents
/// derived from partially applied tiles are deliberately NOT an input budget.
#[derive(Debug, Clone, PartialEq)]
pub struct Context {
    output: Option<String>,
    reported_span: Option<(f64, f64)>,
    decorations_and_widths: Vec<(Option<f64>, f64)>,
    native: Option<std::sync::Arc<[u8]>>,
}

impl Context {
    pub fn compatible(&self, other: &Self) -> bool {
        self.output == other.output
            && self.native == other.native
            && self.reported_span == other.reported_span
            && self.decorations_and_widths.len() == other.decorations_and_widths.len()
            && self
                .decorations_and_widths
                .iter()
                .zip(&other.decorations_and_widths)
                .all(|(a, b)| {
                    (match (a.0, b.0) {
                        (Some(a), Some(b)) => (a - b).abs() <= 2.0,
                        (None, None) => true,
                        _ => false,
                    }) && (a.1 - b.1).abs() <= 2.0
                })
    }
}

pub fn observation(
    capture: &desktop::DesktopCapture,
    key: &desktop::GroupKey,
    minimum: u32,
    retained_budget: Option<f64>,
) -> Result<(policy::Observation, Context), String> {
    if !capture.is_ready() {
        return Err("source or initial windows/workspaces families are not current".into());
    }
    let group = capture.groups.get(key).ok_or("group disappeared")?;
    if group.membership != desktop::Evidence::Reported {
        return Err("native ordered grouping is not reported unambiguously".into());
    }
    if group.secondary_axis != desktop::Axis::Vertical {
        return Err("height control requires a reported vertical secondary axis".into());
    }
    if group.members.len() < 2 || group.members.len() > 128 {
        return Err("supported group size is 2..=128 members".into());
    }
    let workspace = capture
        .workspaces
        .get(&key.workspace)
        .ok_or("workspace is not known")?;
    let mut members = Vec::new();
    let mut signatures = Vec::new();
    let native = capture.allocations.get(key);
    if native.is_some_and(|a| a.members.len() != group.members.len()) {
        return Err("native member evidence mismatch".into());
    }
    for (index, id) in group.members.iter().enumerate() {
        let window = capture.windows.get(id).ok_or("group member is missing")?;
        if window.placement != desktop::Placement::Tiled
            || window.workspace.as_ref() != Some(&key.workspace)
            || window.group.as_ref() != Some(key)
            || id.scope != capture.scope
        {
            return Err("group membership, placement or source scope is inconsistent".into());
        }
        let content = window
            .geometry
            .content_size
            .ok_or("committed content geometry is unavailable")?;
        if !content.height.is_finite()
            || content.height < 0.0
            || !content.width.is_finite()
            || content.width <= 0.0
        {
            return Err("invalid committed content geometry".into());
        }
        let decoration = window
            .geometry
            .tile_size
            .map(|size| size.height - content.height);
        if decoration.is_some_and(|d| !d.is_finite()) {
            return Err("invalid tile decorations".into());
        }
        if native.is_none() {
            signatures.push((decoration, content.width));
        }
        let constraints = native.map(|allocation| &allocation.members[index]);
        members.push(policy::ObservedMember {
            id: native_id(&id.native),
            extent: content.height,
            minimum: constraints.map_or(minimum, |c| c.minimum.max(minimum)),
            maximum: constraints.and_then(|c| c.maximum),
        });
    }
    let budget = native.map_or_else(
        || retained_budget.unwrap_or_else(|| members.iter().map(|m| m.extent).sum()),
        |a| a.content_budget as f64,
    );
    let focus = match &capture.active_window {
        None => policy::Focus::None,
        Some(id) => match capture.windows.get(id).and_then(|w| w.workspace.as_ref()) {
            Some(workspace) => policy::Focus::Window(policy::WindowKey {
                source: source(&id.scope),
                workspace: native_id(&workspace.native),
                id: native_id(&id.native),
            }),
            None if capture.control_status.is_some() => policy::Focus::Window(policy::WindowKey {
                source: source(&id.scope),
                workspace: "outside-native-layout".into(),
                id: native_id(&id.native),
            }),
            None => policy::Focus::Unknown,
        },
    };
    let reported_span = if group.geometry_evidence == desktop::Evidence::Reported {
        group.bounds.map(|rect| (rect.size.width, rect.size.height))
    } else {
        None
    };
    Ok((
        policy::Observation {
            group: policy::GroupKey {
                source: source(&capture.scope),
                workspace: native_id(&key.workspace.native),
                topology: native_id(&key.position),
                axis: policy::Axis::Vertical,
            },
            version: capture.version.0,
            available_extent: budget,
            gaps: policy::Gaps::AlreadyExcluded,
            members,
            focus,
        },
        Context {
            output: workspace.output.clone(),
            reported_span,
            decorations_and_widths: signatures,
            native: native.map(|a| a.context.clone()),
        },
    ))
}

pub fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}
