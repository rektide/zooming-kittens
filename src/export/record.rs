// pattern: Functional Core

use crate::export::filter::{ExportFilter, ExportRecordId, matches_filters};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExportRecord {
    Window {
        #[serde(flatten)]
        data: niri_ipc::Window,
    },
    Workspace {
        #[serde(flatten)]
        data: niri_ipc::Workspace,
    },
    Output {
        #[serde(flatten)]
        data: ExportOutputData,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ExportOutputData {
    Full(niri_ipc::Output),
    CurrentMode(OutputCurrentMode),
}

#[derive(Debug, Clone, Serialize)]
pub struct OutputCurrentMode {
    pub name: String,
    pub make: String,
    pub model: String,
    pub serial: Option<String>,
    pub physical_size: Option<(u32, u32)>,
    pub current_mode: Option<niri_ipc::Mode>,
    pub is_custom_mode: bool,
    pub vrr_supported: bool,
    pub vrr_enabled: bool,
    pub logical: Option<niri_ipc::LogicalOutput>,
}

impl ExportRecord {
    pub fn record_id(&self) -> ExportRecordId {
        match self {
            Self::Window { data } => ExportRecordId::Window(data.id),
            Self::Workspace { data } => ExportRecordId::Workspace(data.id),
            Self::Output { data } => ExportRecordId::Output(data.name().to_owned()),
        }
    }
}

impl ExportOutputData {
    fn name(&self) -> &str {
        match self {
            Self::Full(output) => &output.name,
            Self::CurrentMode(output) => &output.name,
        }
    }
}

pub fn collect_export_records(
    windows: Vec<niri_ipc::Window>,
    workspaces: Vec<niri_ipc::Workspace>,
    outputs: HashMap<String, niri_ipc::Output>,
    filters: &[ExportFilter],
    output_mode: ExportOutputMode,
) -> Vec<ExportRecord> {
    let mut records = Vec::new();

    records.extend(windows.into_iter().map(|window| ExportRecord::Window { data: window }));
    records.extend(
        workspaces
            .into_iter()
            .map(|workspace| ExportRecord::Workspace { data: workspace }),
    );

    let mut outputs = outputs.into_values().collect::<Vec<_>>();
    outputs.sort_by(|left, right| left.name.cmp(&right.name));
    records.extend(outputs.into_iter().map(|output| ExportRecord::Output {
        data: output_mode.project(output),
    }));

    records
        .into_iter()
        .filter(|record| matches_filters(&record.record_id(), filters))
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportOutputMode {
    Full,
    CurrentMode,
}

impl ExportOutputMode {
    fn project(self, output: niri_ipc::Output) -> ExportOutputData {
        match self {
            Self::Full => ExportOutputData::Full(output),
            Self::CurrentMode => ExportOutputData::CurrentMode(OutputCurrentMode {
                name: output.name,
                make: output.make,
                model: output.model,
                serial: output.serial,
                physical_size: output.physical_size,
                current_mode: output
                    .current_mode
                    .and_then(|index| output.modes.get(index).copied()),
                is_custom_mode: output.is_custom_mode,
                vrr_supported: output.vrr_supported,
                vrr_enabled: output.vrr_enabled,
                logical: output.logical,
            }),
        }
    }
}
