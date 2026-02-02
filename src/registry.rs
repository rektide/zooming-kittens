use kitty_rc::commands::SetFontSizeCommand;
use kitty_rc::Kitty;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;

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
    Success {
        pid: i32,
        font_adjustment: String,
    },
    NotConfigured,
    ConnectionFailed,
    AuthFailed,
    Failed,
}

struct ManagedConnection {
    client: Arc<Mutex<Kitty>>,
    last_used: Instant,
}

pub struct KittyRegistry {
    connections: Arc<Mutex<HashMap<i32, ManagedConnection>>>,
    statuses: Arc<Mutex<HashMap<i32, KittyConnectionStatus>>>,
    config: RegistryConfig,
}

#[derive(Clone)]
pub struct RegistryConfig {
    pub socket_timeout: Duration,
    pub max_retries: u32,
    pub max_connections: usize,
    pub idle_timeout: Duration,
    pub reap_interval: Duration,
    pub verbose: bool,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            socket_timeout: Duration::from_secs(2),
            max_retries: 3,
            max_connections: 10,
            idle_timeout: Duration::from_secs(1800), // 30 minutes
            reap_interval: Duration::from_secs(300),  // 5 minutes
            verbose: false,
        }
    }
}

pub struct FocusTracker {
    current_focused_kitty: Option<i32>,
}

impl FocusTracker {
    pub fn new() -> Self {
        Self {
            current_focused_kitty: None,
        }
    }

    pub fn on_focus_gained(&mut self, pid: i32) {
        self.current_focused_kitty = Some(pid);
    }

    pub fn on_focus_lost(&mut self) -> Option<i32> {
        self.current_focused_kitty.take()
    }

    pub fn current_focused(&self) -> Option<i32> {
        self.current_focused_kitty
    }
}

impl KittyRegistry {
    pub fn new(config: RegistryConfig) -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            statuses: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(RegistryConfig::default())
    }

    pub async fn start_reaper(&self) {
        let connections = Arc::clone(&self.connections);
        let statuses = Arc::clone(&self.statuses);
        let idle_timeout = self.config.idle_timeout;
        let reap_interval = self.config.reap_interval;

        tokio::spawn(async move {
            loop {
                sleep(reap_interval).await;

                let mut to_remove = Vec::new();

                {
                    let connections = connections.lock().await;
                    let now = Instant::now();

                    for (pid, conn) in connections.iter() {
                        let is_dead = !is_process_alive(*pid);
                        let is_idle = now.duration_since(conn.last_used) > idle_timeout;

                        if is_dead || is_idle {
                            if is_dead {
                                eprintln!("Reaping dead PID {}", pid);
                            } else {
                                eprintln!("Reaping idle PID {} (unused for >{:?})", pid, idle_timeout);
                            }
                            to_remove.push(*pid);
                        }
                    }
                }

                for pid in &to_remove {
                    let mut connections = connections.lock().await;
                    if let Some(conn) = connections.remove(pid) {
                        let mut client = conn.client.lock().await;
                        if let Err(e) = client.close().await {
                            eprintln!("Error closing connection for PID {}: {}", pid, e);
                        }
                    }
                    statuses.lock().await.remove(pid);
                }
            }
        });
    }

    pub async fn increase_font_size(&self, pid: i32) -> Result<ZoomingResult, Box<dyn std::error::Error>> {
        self.execute_font_command(pid, true).await
    }

    pub async fn decrease_font_size(&self, pid: i32) -> Result<ZoomingResult, Box<dyn std::error::Error>> {
        self.execute_font_command(pid, false).await
    }

    pub async fn cleanup_dead_connections(&self) {
        let mut to_remove = Vec::new();

        {
            let connections = self.connections.lock().await;

            for pid in connections.keys() {
                if !is_process_alive(*pid) {
                    to_remove.push(*pid);
                }
            }
        }

        for pid in &to_remove {
            eprintln!("Cleaning up dead PID {}", pid);
            let mut connections = self.connections.lock().await;
            if let Some(conn) = connections.remove(pid) {
                let mut client = conn.client.lock().await;
                if let Err(e) = client.close().await {
                    eprintln!("Error closing connection for PID {}: {}", pid, e);
                }
            }
            self.statuses.lock().await.remove(pid);
        }
    }

    async fn execute_font_command(&self, pid: i32, increase: bool) -> Result<ZoomingResult, Box<dyn std::error::Error>> {
        let password = match get_kitty_password() {
            Ok(pw) => pw,
            Err(_) => {
                self.set_status(pid, KittyConnectionStatus::NotConfigured).await;
                return Ok(ZoomingResult::NotConfigured);
            }
        };

        let socket_path = get_kitty_socket_path(pid);

        if !socket_path.exists() {
            self.set_status(pid, KittyConnectionStatus::NoSocket).await;
            return Ok(ZoomingResult::NotConfigured);
        }

        let increment_op = if increase { "+" } else { "-" };

        let mut last_error = None;

        for attempt in 0..self.config.max_retries {
            if attempt > 0 {
                let delay = match attempt {
                    1 => Duration::ZERO,
                    2 => Duration::from_millis(100),
                    _ => Duration::from_millis(200),
                };
                sleep(delay).await;
            }

            let client = match self.get_or_create_connection(pid, &socket_path, &password).await {
                Ok(client) => client,
                Err(e) => {
                    last_error = Some(e.to_string());
                    continue;
                }
            };

            let mut all_succeeded = true;

            for _ in 0..3 {
                let cmd = SetFontSizeCommand::new(0)
                    .increment_op(increment_op)
                    .build()?;

                if self.config.verbose {
                    eprintln!("Sending command to PID {}: {:?}", pid, cmd);
                }

                let mut client = client.lock().await;
                let result = client.execute(&cmd).await;
                if self.config.verbose {
                    eprintln!("Font command result for PID {}: {:?}", pid, result);
                }
                match result {
                    Ok(response) => {
                        if !response.ok {
                            all_succeeded = false;
                            let error_msg = response.error.unwrap_or_else(|| "Unknown error".to_string());
                            eprintln!("Kitty returned error for PID {}: {}", pid, error_msg);
                            last_error = Some(error_msg);
                            break;
                        }
                    }
                    Err(e) => {
                        all_succeeded = false;
                        last_error = Some(e.to_string());
                        eprintln!("Error executing font command for PID {}: {}", pid, e);
                        break;
                    }
                }
            }

            if all_succeeded {
                self.update_last_used(pid).await;
                self.set_status(pid, KittyConnectionStatus::Ready).await;

                let font_adjustment = format!("{}3", if increase { "+" } else { "-" });
                return Ok(ZoomingResult::Success {
                    pid,
                    font_adjustment,
                });
            }
        }

        self.set_status(pid, KittyConnectionStatus::Failed).await;

        if let Some(err) = last_error {
            if err.contains("auth") || err.contains("password") {
                return Ok(ZoomingResult::AuthFailed);
            }
        }

        Ok(ZoomingResult::ConnectionFailed)
    }

    async fn get_or_create_connection(&self, pid: i32, socket_path: &PathBuf, password: &str) -> Result<Arc<Mutex<Kitty>>, String> {
        {
            let mut connections = self.connections.lock().await;

            if let Some(conn) = connections.get_mut(&pid) {
                conn.last_used = Instant::now();
                return Ok(Arc::clone(&conn.client));
            }

            if connections.len() >= self.config.max_connections {
                let oldest_pid = connections
                    .iter()
                    .min_by_key(|(_, conn)| conn.last_used)
                    .map(|(pid, _)| *pid);

                if let Some(old_pid) = oldest_pid {
                    if let Some(old_conn) = connections.remove(&old_pid) {
                        let mut client = old_conn.client.lock().await;
                        if let Err(e) = client.close().await {
                            eprintln!("Error closing connection for PID {}: {}", old_pid, e);
                        }
                    }
                    self.statuses.lock().await.remove(&old_pid);
                }
            }
        }

        if self.config.verbose {
            eprintln!("Connecting to kitty PID {} at socket: {:?}", pid, socket_path);
        }

        let client = match Kitty::builder()
            .socket_path(socket_path)
            .timeout(self.config.socket_timeout)
            .password(password)
            .connect()
            .await
        {
            Ok(c) => {
                if self.config.verbose {
                    eprintln!("Successfully connected to kitty PID {}", pid);
                }
                c
            }
            Err(e) => {
                eprintln!("Failed to connect to kitty PID {}: {}", pid, e);
                self.set_status(pid, KittyConnectionStatus::Failed).await;
                return Err(e.to_string());
            }
        };

        let mut connections = self.connections.lock().await;
        let client_arc = Arc::new(Mutex::new(client));
        connections.insert(pid, ManagedConnection {
            client: Arc::clone(&client_arc),
            last_used: Instant::now(),
        });

        Ok(client_arc)
    }

    async fn update_last_used(&self, pid: i32) {
        let mut connections = self.connections.lock().await;
        if let Some(conn) = connections.get_mut(&pid) {
            conn.last_used = Instant::now();
        }
    }

    async fn set_status(&self, pid: i32, status: KittyConnectionStatus) {
        self.statuses.lock().await.insert(pid, status);
    }

    pub async fn get_status(&self, pid: i32) -> Option<KittyConnectionStatus> {
        self.statuses.lock().await.get(&pid).cloned()
    }

    pub async fn shutdown(&self) {
        let mut connections = self.connections.lock().await;

        for (pid, conn) in connections.drain() {
            let mut client = conn.client.lock().await;
            if let Err(e) = client.close().await {
                eprintln!("Error closing connection for PID {}: {}", pid, e);
            }
        }

        self.statuses.lock().await.clear();
    }
}

fn get_kitty_password() -> Result<String, std::io::Error> {
    let password_path = dirs::config_dir()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Config directory not found"))?
        .join("kitty/rc.password");

    if !password_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Password file not found",
        ));
    }

    fs::read_to_string(password_path)
        .map(|s| s.trim().to_string())
}

fn get_kitty_socket_path(pid: i32) -> PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| "/tmp".to_string());

    PathBuf::from(runtime_dir)
        .join(format!("kitty-{}.sock", pid))
}

fn is_process_alive(pid: i32) -> bool {
    std::path::Path::new(&format!("/proc/{}", pid)).exists()
}
