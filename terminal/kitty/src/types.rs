use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KittyConnectionStatus {
    Ready,
    NoSocket,
    NotConfigured,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ZoomingResult {
    Success { pid: i32, font_adjustment: String },
    NotConfigured,
    ConnectionFailed,
    AuthFailed,
    Failed,
}
