// pattern: Functional Core

use crate::export::filter::ExportFilter;
use crate::export::record::{ExportOutputMode, ExportRecord};
use std::collections::HashMap;

pub fn collect_snapshot_records(
    windows: Vec<niri_ipc::Window>,
    workspaces: Vec<niri_ipc::Workspace>,
    outputs: HashMap<String, niri_ipc::Output>,
    casts: Vec<niri_ipc::Cast>,
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
    records.extend(casts.into_iter().map(|cast| ExportRecord::Cast { data: cast }));

    records
        .into_iter()
        .filter(|record| record.matches_filters(filters))
        .collect()
}
