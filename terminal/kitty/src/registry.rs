use crate::config::RegistryConfig;
use crate::types::{KittyConnectionStatus, ZoomingResult};
use crate::util::{get_kitty_password, get_kitty_socket_path, is_process_alive};
use kitty_rc::Kitty;
use kitty_rc::commands::SetFontSizeCommand;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;

struct ManagedConnection {
    client: Arc<Mutex<Kitty>>,
    last_used: Instant,
}

pub struct KittyRegistry {
    connections: Arc<Mutex<HashMap<i32, ManagedConnection>>>,
    statuses: Arc<Mutex<HashMap<i32, KittyConnectionStatus>>>,
    config: RegistryConfig,
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
                                eprintln!(
                                    "Reaping idle PID {} (unused for >{:?})",
                                    pid, idle_timeout
                                );
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

    pub async fn increase_font_size(
        &self,
        pid: i32,
    ) -> Result<ZoomingResult, Box<dyn std::error::Error>> {
        self.execute_font_command(pid, true, 3).await
    }

    pub async fn decrease_font_size(
        &self,
        pid: i32,
    ) -> Result<ZoomingResult, Box<dyn std::error::Error>> {
        self.execute_font_command(pid, false, 3).await
    }

    pub async fn increase_font_size_by(
        &self,
        pid: i32,
        amount: u32,
    ) -> Result<ZoomingResult, Box<dyn std::error::Error>> {
        self.execute_font_command(pid, true, amount).await
    }

    pub async fn decrease_font_size_by(
        &self,
        pid: i32,
        amount: u32,
    ) -> Result<ZoomingResult, Box<dyn std::error::Error>> {
        self.execute_font_command(pid, false, amount).await
    }

    /// Send one additive adjustment as a single remote command.
    pub async fn add_font_size(
        &self,
        pid: i32,
        amount: f64,
    ) -> Result<ZoomingResult, Box<dyn std::error::Error>> {
        self.execute_font_command_with_op(pid, "+", amount).await
    }

    /// Send one subtractive adjustment as a single remote command.
    pub async fn subtract_font_size(
        &self,
        pid: i32,
        amount: f64,
    ) -> Result<ZoomingResult, Box<dyn std::error::Error>> {
        self.execute_font_command_with_op(pid, "-", amount).await
    }

    pub async fn multiply_font_size_by(
        &self,
        pid: i32,
        factor: f64,
    ) -> Result<ZoomingResult, Box<dyn std::error::Error>> {
        self.execute_font_command_with_op(pid, "*", factor).await
    }

    pub async fn divide_font_size_by(
        &self,
        pid: i32,
        factor: f64,
    ) -> Result<ZoomingResult, Box<dyn std::error::Error>> {
        self.execute_font_command_with_op(pid, "/", factor).await
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

    /// Legacy integer-increment entry points. A whole-number adjustment is one
    /// remote command carrying the exact total; the old one-command-per-point
    /// loop is not needed because kitty applies `old <op> size` atomically.
    async fn execute_font_command(
        &self,
        pid: i32,
        increase: bool,
        amount: u32,
    ) -> Result<ZoomingResult, Box<dyn std::error::Error>> {
        let op = if increase { "+" } else { "-" };
        self.execute_font_command_with_op(pid, op, amount as f64)
            .await
    }

    /// Apply `old <op> amount` to the font size of the kitty instance behind
    /// `pid` as a single `set-font-size` command. Kitty's remote-control
    /// protocol accepts `increment_op` in `+`, `-`, `*`, `/` with a positive
    /// float size, so every adjustment — including fractional and
    /// multiplicative — is one atomic command.
    pub async fn execute_font_command_with_op(
        &self,
        pid: i32,
        op: &str,
        amount: f64,
    ) -> Result<ZoomingResult, Box<dyn std::error::Error>> {
        if !matches!(op, "+" | "-" | "*" | "/") {
            return Err(
                format!("invalid font increment op {op:?}: expected one of +, -, *, /").into(),
            );
        }
        if !amount.is_finite() || amount <= 0.0 {
            return Err(format!(
                "invalid font adjustment amount {amount}: must be a finite positive number"
            )
            .into());
        }

        // Resolve only this process's ancestry, including the supplied PID.
        // Never scan other terminals or trust a stale PID-reuse cache entry.
        let kitty_pid = match crate::process::find_kitty_master_pid(pid) {
            Some(kpid) => kpid,
            None => {
                self.set_status(pid, KittyConnectionStatus::NoSocket).await;
                return Ok(ZoomingResult::NotConfigured);
            }
        };

        let socket_path = get_kitty_socket_path(kitty_pid);

        if !socket_path.exists() {
            self.set_status(pid, KittyConnectionStatus::NoSocket).await;
            return Ok(ZoomingResult::NotConfigured);
        }

        if self.config.verbose {
            eprintln!(
                "Mapped process PID {} to kitty master PID {}",
                pid, kitty_pid
            );
        }

        let password = match get_kitty_password() {
            Ok(pw) => pw,
            Err(_) => {
                self.set_status(kitty_pid, KittyConnectionStatus::NotConfigured)
                    .await;
                return Ok(ZoomingResult::NotConfigured);
            }
        };

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

            let client = match self
                .get_or_create_connection(kitty_pid, &socket_path, &password)
                .await
            {
                Ok(client) => client,
                Err(e) => {
                    last_error = Some(e);
                    continue;
                }
            };

            let cmd = SetFontSizeCommand::builder()
                .size(amount)
                .increment_op(op.to_string())
                .build()
                .to_message()?;

            if self.config.verbose {
                eprintln!(
                    "Sending font adjustment {}{} to PID {} (kitty: {})",
                    op, amount, pid, kitty_pid
                );
            }

            let mut client = client.lock().await;
            let result = client.execute(&cmd).await;

            if self.config.verbose {
                eprintln!(
                    "Font command reply for PID {} (kitty: {}): received={}",
                    pid,
                    kitty_pid,
                    result.is_ok()
                );
            }
            match result {
                Ok(response) => {
                    if response.ok {
                        self.update_last_used(kitty_pid).await;
                        self.set_status(pid, KittyConnectionStatus::Ready).await;

                        let font_adjustment = format!("{}{}", op, amount);
                        return Ok(ZoomingResult::Success {
                            pid,
                            font_adjustment,
                        });
                    }
                    let error_msg = response
                        .error
                        .unwrap_or_else(|| "Unknown error".to_string());
                    eprintln!(
                        "Kitty refused font command for PID {} (kitty: {})",
                        pid, kitty_pid
                    );
                    last_error = Some(error_msg);
                }
                Err(e) => {
                    eprintln!(
                        "Error executing font command for PID {} (kitty: {})",
                        pid, kitty_pid
                    );
                    last_error = Some(e.to_string());
                }
            }
        }

        self.set_status(pid, KittyConnectionStatus::Failed).await;

        if let Some(err) = &last_error
            && (err.contains("auth") || err.contains("password"))
        {
            return Ok(ZoomingResult::AuthFailed);
        }

        Ok(ZoomingResult::ConnectionFailed)
    }

    async fn get_or_create_connection(
        &self,
        pid: i32,
        socket_path: &PathBuf,
        password: &str,
    ) -> Result<Arc<Mutex<Kitty>>, String> {
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
            eprintln!(
                "Connecting to kitty PID {} at socket: {:?}",
                pid, socket_path
            );
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
                eprintln!("Failed to connect to kitty PID {}", pid);
                self.set_status(pid, KittyConnectionStatus::Failed).await;
                return Err(e.to_string());
            }
        };

        let mut connections = self.connections.lock().await;
        let client_arc = Arc::new(Mutex::new(client));
        connections.insert(
            pid,
            ManagedConnection {
                client: Arc::clone(&client_arc),
                last_used: Instant::now(),
            },
        );

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

    pub fn verbose(&self) -> bool {
        self.config.verbose
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
