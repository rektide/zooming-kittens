use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct RegistryConfig {
    pub socket_timeout: Duration,
    pub max_retries: u32,
    pub max_connections: usize,
    pub idle_timeout: Duration,
    pub reap_interval: Duration,
    pub verbose: bool,
}

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
