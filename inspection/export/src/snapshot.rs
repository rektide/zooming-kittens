// pattern: Functional Core

use crate::filter::ExportFilter;
use crate::record::{ExportOutputMode, ExportRecord};
use std::collections::HashMap;

pub struct Snapshot {
    pub version: String,
    pub windows: Vec<niri_ipc::Window>,
    pub workspaces: Vec<niri_ipc::Workspace>,
    pub outputs: HashMap<String, niri_ipc::Output>,
    pub layers: Vec<niri_ipc::LayerSurface>,
    pub casts: Vec<niri_ipc::Cast>,
    pub keyboard_layouts: niri_ipc::KeyboardLayouts,
    pub overview: niri_ipc::Overview,
}

pub fn collect_snapshot_records(
    snapshot: Snapshot,
    filters: &[ExportFilter],
    output_mode: ExportOutputMode,
) -> Vec<ExportRecord> {
    let Snapshot {
        version,
        windows,
        workspaces,
        outputs,
        layers,
        casts,
        keyboard_layouts,
        overview,
    } = snapshot;
    let mut records = Vec::new();

    records.push(ExportRecord::Version { version });
    records.extend(
        windows
            .into_iter()
            .map(|window| ExportRecord::Window { data: window }),
    );
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
    records.extend(
        layers
            .into_iter()
            .map(|layer| ExportRecord::Layer { data: layer }),
    );
    records.extend(
        casts
            .into_iter()
            .map(|cast| ExportRecord::Cast { data: cast }),
    );
    records.push(ExportRecord::KeyboardLayouts {
        data: keyboard_layouts,
    });
    records.push(ExportRecord::Overview { data: overview });

    records
        .into_iter()
        .filter(|record| record.matches_filters(filters))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::ExportRecordKind;

    fn snapshot() -> Snapshot {
        Snapshot {
            version: "test-version".into(),
            windows: vec![],
            workspaces: vec![],
            outputs: HashMap::new(),
            layers: vec![],
            casts: vec![],
            keyboard_layouts: niri_ipc::KeyboardLayouts {
                names: vec!["English".into()],
                current_idx: 0,
            },
            overview: niri_ipc::Overview { is_open: true },
        }
    }

    #[test]
    fn named_snapshot_preserves_global_records_and_order() {
        let records = collect_snapshot_records(snapshot(), &[], ExportOutputMode::Full);
        assert!(
            matches!(&records[..], [ExportRecord::Version { version }, ExportRecord::KeyboardLayouts { data }, ExportRecord::Overview { data: overview }]
            if version == "test-version" && data.names == ["English"] && data.current_idx == 0 && overview.is_open)
        );
    }

    #[test]
    fn named_snapshot_applies_filters() {
        let records = collect_snapshot_records(
            snapshot(),
            &[ExportFilter::All(ExportRecordKind::Overview)],
            ExportOutputMode::CurrentMode,
        );
        assert!(matches!(&records[..], [ExportRecord::Overview { data }] if data.is_open));
    }
}
