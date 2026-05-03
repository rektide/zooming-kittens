// pattern: Functional Core

use crate::export::filter::{ExportFilter, ExportRecordId, ExportRecordMatch, matches_record};
use serde::Serialize;

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
    WorkspacesChanged {
        stream_event_id: u64,
        workspaces: Vec<niri_ipc::Workspace>,
    },
    WorkspaceUrgencyChanged {
        stream_event_id: u64,
        id: u64,
        urgent: bool,
    },
    WorkspaceActivated {
        stream_event_id: u64,
        id: u64,
        focused: bool,
    },
    WorkspaceActiveWindowChanged {
        stream_event_id: u64,
        workspace_id: u64,
        active_window_id: Option<u64>,
    },
    WindowsChanged {
        stream_event_id: u64,
        windows: Vec<niri_ipc::Window>,
    },
    WindowOpenedOrChanged {
        stream_event_id: u64,
        window: niri_ipc::Window,
    },
    WindowClosed {
        stream_event_id: u64,
        id: u64,
    },
    WindowFocusChanged {
        stream_event_id: u64,
        id: Option<u64>,
    },
    WindowFocusTimestampChanged {
        stream_event_id: u64,
        id: u64,
        focus_timestamp: Option<niri_ipc::Timestamp>,
    },
    WindowUrgencyChanged {
        stream_event_id: u64,
        id: u64,
        urgent: bool,
    },
    WindowLayoutsChanged {
        stream_event_id: u64,
        changes: Vec<(u64, niri_ipc::WindowLayout)>,
    },
    KeyboardLayoutsChanged {
        stream_event_id: u64,
        keyboard_layouts: niri_ipc::KeyboardLayouts,
    },
    KeyboardLayoutSwitched {
        stream_event_id: u64,
        idx: u8,
    },
    OverviewOpenedOrClosed {
        stream_event_id: u64,
        is_open: bool,
    },
    ConfigLoaded {
        stream_event_id: u64,
        failed: bool,
    },
    ScreenshotCaptured {
        stream_event_id: u64,
        path: Option<String>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportOutputMode {
    Full,
    CurrentMode,
}

impl ExportRecord {
    pub fn record_type(&self) -> &'static str {
        match self {
            Self::Window { .. } => "window",
            Self::Workspace { .. } => "workspace",
            Self::Output { .. } => "output",
            Self::WorkspacesChanged { .. } => "workspaces_changed",
            Self::WorkspaceUrgencyChanged { .. } => "workspace_urgency_changed",
            Self::WorkspaceActivated { .. } => "workspace_activated",
            Self::WorkspaceActiveWindowChanged { .. } => "workspace_active_window_changed",
            Self::WindowsChanged { .. } => "windows_changed",
            Self::WindowOpenedOrChanged { .. } => "window_opened_or_changed",
            Self::WindowClosed { .. } => "window_closed",
            Self::WindowFocusChanged { .. } => "window_focus_changed",
            Self::WindowFocusTimestampChanged { .. } => "window_focus_timestamp_changed",
            Self::WindowUrgencyChanged { .. } => "window_urgency_changed",
            Self::WindowLayoutsChanged { .. } => "window_layouts_changed",
            Self::KeyboardLayoutsChanged { .. } => "keyboard_layouts_changed",
            Self::KeyboardLayoutSwitched { .. } => "keyboard_layout_switched",
            Self::OverviewOpenedOrClosed { .. } => "overview_opened_or_closed",
            Self::ConfigLoaded { .. } => "config_loaded",
            Self::ScreenshotCaptured { .. } => "screenshot_captured",
        }
    }

    pub fn record_ids(&self) -> Vec<ExportRecordId> {
        match self {
            Self::Window { data } => vec![ExportRecordId::Window(data.id)],
            Self::Workspace { data } => vec![ExportRecordId::Workspace(data.id)],
            Self::Output { data } => vec![ExportRecordId::Output(data.name().to_owned())],
            Self::WorkspacesChanged { workspaces, .. } => workspaces
                .iter()
                .map(|workspace| ExportRecordId::Workspace(workspace.id))
                .collect(),
            Self::WorkspaceUrgencyChanged { id, .. }
            | Self::WorkspaceActivated { id, .. } => vec![ExportRecordId::Workspace(*id)],
            Self::WorkspaceActiveWindowChanged {
                workspace_id,
                active_window_id,
                ..
            } => {
                let mut ids = vec![ExportRecordId::Workspace(*workspace_id)];
                if let Some(active_window_id) = active_window_id {
                    ids.push(ExportRecordId::Window(*active_window_id));
                }
                ids
            }
            Self::WindowsChanged { windows, .. } => windows
                .iter()
                .map(|window| ExportRecordId::Window(window.id))
                .collect(),
            Self::WindowOpenedOrChanged { window, .. } => vec![ExportRecordId::Window(window.id)],
            Self::WindowClosed { id, .. }
            | Self::WindowFocusTimestampChanged { id, .. }
            | Self::WindowUrgencyChanged { id, .. } => vec![ExportRecordId::Window(*id)],
            Self::WindowFocusChanged { id, .. } => id
                .map(|id| vec![ExportRecordId::Window(id)])
                .unwrap_or_default(),
            Self::WindowLayoutsChanged { changes, .. } => changes
                .iter()
                .map(|(id, _)| ExportRecordId::Window(*id))
                .collect(),
            Self::KeyboardLayoutsChanged { .. }
            | Self::KeyboardLayoutSwitched { .. }
            | Self::OverviewOpenedOrClosed { .. }
            | Self::ConfigLoaded { .. }
            | Self::ScreenshotCaptured { .. } => Vec::new(),
        }
    }

    pub fn matches_filters(&self, filters: &[ExportFilter]) -> bool {
        let ids = self.record_ids();
        matches_record(
            ExportRecordMatch {
            record_type: self.record_type(),
                ids: &ids,
            },
            filters,
        )
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

impl ExportOutputMode {
    pub fn project(self, output: niri_ipc::Output) -> ExportOutputData {
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
