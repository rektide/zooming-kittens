use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: u64,
    pub app_id: Option<String>,
    pub pid: Option<i32>,
    pub title: Option<String>,
}

impl WindowInfo {
    pub fn from_niri_window(window: &niri_ipc::Window) -> Self {
        Self {
            id: window.id,
            app_id: window.app_id.clone(),
            pid: window.pid,
            title: window.title.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NiriEvent {
    #[serde(rename = "focus")]
    Focus { window_id: u64, window: WindowInfo },
    #[serde(rename = "blur")]
    Blur { window_id: u64, window: WindowInfo },
    #[serde(rename = "create")]
    Create { window_id: u64, window: WindowInfo },
    #[serde(rename = "destroy")]
    Destroy { window_id: u64 },
}

impl NiriEvent {
    pub fn window(&self) -> Option<&WindowInfo> {
        match self {
            NiriEvent::Focus { window, .. } => Some(window),
            NiriEvent::Blur { window, .. } => Some(window),
            NiriEvent::Create { window, .. } => Some(window),
            NiriEvent::Destroy { .. } => None,
        }
    }

    pub fn window_id(&self) -> Option<u64> {
        match self {
            NiriEvent::Focus { window_id, .. } => Some(*window_id),
            NiriEvent::Blur { window_id, .. } => Some(*window_id),
            NiriEvent::Create { window_id, .. } => Some(*window_id),
            NiriEvent::Destroy { window_id, .. } => Some(*window_id),
        }
    }
}
