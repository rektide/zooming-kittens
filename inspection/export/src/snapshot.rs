// pattern: Functional Core

use crate::filter::ExportFilter;
use crate::record::{ExportOutputMode, ExportRecord};
use std::collections::HashMap;

pub fn collect_snapshot_records(
    version: String,
    windows: Vec<niri_ipc::Window>,
    workspaces: Vec<niri_ipc::Workspace>,
    outputs: HashMap<String, niri_ipc::Output>,
    layers: Vec<niri_ipc::LayerSurface>,
    casts: Vec<niri_ipc::Cast>,
    keyboard_layouts: niri_ipc::KeyboardLayouts,
    overview: niri_ipc::Overview,
    filters: &[ExportFilter],
    output_mode: ExportOutputMode,
) -> Vec<ExportRecord> {
    let mut records = Vec::new();

    records.push(ExportRecord::Version { version });
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
    records.extend(layers.into_iter().map(|layer| ExportRecord::Layer { data: layer }));
    records.extend(casts.into_iter().map(|cast| ExportRecord::Cast { data: cast }));
    records.push(ExportRecord::KeyboardLayouts {
        data: keyboard_layouts,
    });
    records.push(ExportRecord::Overview { data: overview });

    records
        .into_iter()
        .filter(|record| record.matches_filters(filters))
        .collect()
}
