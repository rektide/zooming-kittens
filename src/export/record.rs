// pattern: Functional Core

use crate::export::filter::{ExportFilter, ExportRecordId, matches_filters};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExportRecord {
    Window { id: u64, data: niri_ipc::Window },
    Workspace { id: u64, data: niri_ipc::Workspace },
    Output { id: String, data: niri_ipc::Output },
}

impl ExportRecord {
    pub fn record_id(&self) -> ExportRecordId {
        match self {
            Self::Window { id, .. } => ExportRecordId::Window(*id),
            Self::Workspace { id, .. } => ExportRecordId::Workspace(*id),
            Self::Output { id, .. } => ExportRecordId::Output(id.clone()),
        }
    }
}

pub fn collect_export_records(
    windows: Vec<niri_ipc::Window>,
    workspaces: Vec<niri_ipc::Workspace>,
    outputs: HashMap<String, niri_ipc::Output>,
    filters: &[ExportFilter],
) -> Vec<ExportRecord> {
    let mut records = Vec::new();

    records.extend(windows.into_iter().map(|window| ExportRecord::Window {
        id: window.id,
        data: window,
    }));
    records.extend(workspaces.into_iter().map(|workspace| ExportRecord::Workspace {
        id: workspace.id,
        data: workspace,
    }));

    let mut outputs = outputs.into_values().collect::<Vec<_>>();
    outputs.sort_by(|left, right| left.name.cmp(&right.name));
    records.extend(outputs.into_iter().map(|output| ExportRecord::Output {
        id: output.name.clone(),
        data: output,
    }));

    records
        .into_iter()
        .filter(|record| matches_filters(&record.record_id(), filters))
        .collect()
}
