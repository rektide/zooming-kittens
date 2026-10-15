// pattern: Functional Core

use crate::filter::ExportFilter;
use crate::record::ExportRecord;

pub fn event_to_record(event: niri_ipc::Event, stream_event_id: u64) -> ExportRecord {
    match event {
        niri_ipc::Event::WorkspacesChanged { workspaces } => ExportRecord::WorkspacesChanged {
            stream_event_id,
            workspaces,
        },
        niri_ipc::Event::WorkspaceUrgencyChanged { id, urgent } => {
            ExportRecord::WorkspaceUrgencyChanged {
                stream_event_id,
                id,
                urgent,
            }
        }
        niri_ipc::Event::WorkspaceActivated { id, focused } => ExportRecord::WorkspaceActivated {
            stream_event_id,
            id,
            focused,
        },
        niri_ipc::Event::WorkspaceActiveWindowChanged {
            workspace_id,
            active_window_id,
        } => ExportRecord::WorkspaceActiveWindowChanged {
            stream_event_id,
            workspace_id,
            active_window_id,
        },
        niri_ipc::Event::WindowsChanged { windows } => ExportRecord::WindowsChanged {
            stream_event_id,
            windows,
        },
        niri_ipc::Event::WindowOpenedOrChanged { window } => ExportRecord::WindowOpenedOrChanged {
            stream_event_id,
            window,
        },
        niri_ipc::Event::WindowClosed { id } => ExportRecord::WindowClosed {
            stream_event_id,
            id,
        },
        niri_ipc::Event::WindowFocusChanged { id } => ExportRecord::WindowFocusChanged {
            stream_event_id,
            id,
        },
        niri_ipc::Event::WindowFocusTimestampChanged {
            id,
            focus_timestamp,
        } => ExportRecord::WindowFocusTimestampChanged {
            stream_event_id,
            id,
            focus_timestamp,
        },
        niri_ipc::Event::WindowUrgencyChanged { id, urgent } => {
            ExportRecord::WindowUrgencyChanged {
                stream_event_id,
                id,
                urgent,
            }
        }
        niri_ipc::Event::WindowLayoutsChanged { changes } => ExportRecord::WindowLayoutsChanged {
            stream_event_id,
            changes,
        },
        niri_ipc::Event::KeyboardLayoutsChanged { keyboard_layouts } => {
            ExportRecord::KeyboardLayoutsChanged {
                stream_event_id,
                keyboard_layouts,
            }
        }
        niri_ipc::Event::KeyboardLayoutSwitched { idx } => ExportRecord::KeyboardLayoutSwitched {
            stream_event_id,
            idx,
        },
        niri_ipc::Event::OverviewOpenedOrClosed { is_open } => {
            ExportRecord::OverviewOpenedOrClosed {
                stream_event_id,
                is_open,
            }
        }
        niri_ipc::Event::ConfigLoaded { failed } => ExportRecord::ConfigLoaded {
            stream_event_id,
            failed,
        },
        niri_ipc::Event::ScreenshotCaptured { path } => ExportRecord::ScreenshotCaptured {
            stream_event_id,
            path,
        },
        niri_ipc::Event::CastsChanged { casts } => ExportRecord::CastsChanged {
            stream_event_id,
            casts,
        },
        niri_ipc::Event::CastStartedOrChanged { cast } => ExportRecord::CastStartedOrChanged {
            stream_event_id,
            cast,
        },
        niri_ipc::Event::CastStopped { stream_id } => ExportRecord::CastStopped {
            stream_event_id,
            stream_id,
        },
    }
}

pub fn filter_stream_record(
    record: ExportRecord,
    filters: &[ExportFilter],
) -> Option<ExportRecord> {
    record.matches_filters(filters).then_some(record)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::parse_filter_expression;

    #[test]
    fn window_closed_event_matches_window_id_filter() {
        let filters = parse_filter_expression("window=4").expect("filters parse");
        let record = event_to_record(niri_ipc::Event::WindowClosed { id: 4 }, 7);

        assert!(filter_stream_record(record, &filters).is_some());
    }

    #[test]
    fn window_layouts_changed_matches_any_changed_window() {
        let filters = parse_filter_expression("window=8").expect("filters parse");
        let layout = niri_ipc::WindowLayout {
            pos_in_scrolling_layout: None,
            tile_size: (0.0, 0.0),
            window_size: (0, 0),
            tile_pos_in_workspace_view: None,
            window_offset_in_tile: (0.0, 0.0),
        };
        let record = event_to_record(
            niri_ipc::Event::WindowLayoutsChanged {
                changes: vec![(4, layout.clone()), (8, layout)],
            },
            9,
        );

        assert!(filter_stream_record(record, &filters).is_some());
    }

    #[test]
    fn type_filter_matches_stream_event_type() {
        let filters = parse_filter_expression("type=window_closed").expect("filters parse");
        let record = event_to_record(niri_ipc::Event::WindowClosed { id: 4 }, 1);

        assert!(filter_stream_record(record, &filters).is_some());
    }
}
