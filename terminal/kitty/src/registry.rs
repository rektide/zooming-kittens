use crate::config::RegistryConfig;
use crate::public_key::{DiscoveryError, TargetPublicKey};
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

struct ConnectionLease {
    client: Arc<Mutex<Kitty>>,
    target: Arc<TargetPublicKey>,
}

/// Only classifications escape library errors: arbitrary remote strings and
/// protocol/encryption payloads can contain credentials and are never logged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionFailure {
    Discovery(DiscoveryError),
    Io(std::io::ErrorKind),
    Timeout,
    Closed,
    Encryption,
    ExternalKeyHelper,
    Protocol,
    Command,
    RemoteAuthorization,
    RemoteRefused,
}

impl std::fmt::Display for ConnectionFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Discovery(error) => write!(f, "public-key discovery: {error}"),
            Self::Io(kind) => write!(f, "socket/I/O error ({kind:?})"),
            Self::Timeout => f.write_str("socket command/connection deadline elapsed"),
            Self::Closed => f.write_str("socket peer closed the connection"),
            Self::Encryption => {
                f.write_str("public-key/encryption setup or command encryption failed")
            }
            Self::ExternalKeyHelper => {
                f.write_str("external public-key helper unavailable (explicit target key required)")
            }
            Self::Protocol => f.write_str("invalid remote protocol response"),
            Self::Command => f.write_str("remote command construction failed"),
            Self::RemoteAuthorization => f.write_str("remote authorization refused"),
            Self::RemoteRefused => f.write_str("remote command refused"),
        }
    }
}

impl ConnectionFailure {
    fn from_library(error: &kitty_rc::KittyError) -> Self {
        use kitty_rc::{ConnectionError as C, EncryptionError as E, KittyError as K};
        match error {
            K::Connection(C::ConnectionFailed(_, error)) | K::Io(error) => Self::Io(error.kind()),
            K::Connection(C::TimeoutError(_)) => Self::Timeout,
            K::Connection(C::ConnectionClosed) => Self::Closed,
            K::Connection(C::PermissionDenied(_)) => Self::Io(std::io::ErrorKind::PermissionDenied),
            K::Connection(C::SocketNotFound(_)) => Self::Io(std::io::ErrorKind::NotFound),
            K::Connection(_) => Self::Io(std::io::ErrorKind::Other),
            K::Encryption(E::PublicKeyDatabaseError(_)) => Self::ExternalKeyHelper,
            K::Encryption(_) => Self::Encryption,
            K::Protocol(_) => Self::Protocol,
            K::Command(_) => Self::Command,
        }
    }

    fn is_auth(self) -> bool {
        matches!(
            self,
            Self::Discovery(
                DiscoveryError::MissingKey
                    | DiscoveryError::MalformedKey
                    | DiscoveryError::MalformedEnvironment
                    | DiscoveryError::AmbiguousKeys
            ) | Self::Encryption
                | Self::ExternalKeyHelper
                | Self::RemoteAuthorization
        )
    }
}

async fn connect_target(
    target: &TargetPublicKey,
    socket_path: &PathBuf,
    timeout: Duration,
    password: &str,
) -> Result<Kitty, ConnectionFailure> {
    target.verify().map_err(ConnectionFailure::Discovery)?;
    let client = Kitty::builder()
        .socket_path(socket_path)
        .timeout(timeout)
        .password(password)
        // Never let kitty-rc invoke kitty-pubkey-db or fall back to the
        // caller's unrelated KITTY_PUBLIC_KEY / ~/.config/kitty/key.pub.
        .public_key(target.public_key())
        .connect()
        .await
        .map_err(|e| ConnectionFailure::from_library(&e))?;
    target.verify().map_err(ConnectionFailure::Discovery)?;
    Ok(client)
}

pub struct KittyRegistry {
    connections: Arc<Mutex<HashMap<i32, ManagedConnection>>>,
    statuses: Arc<Mutex<HashMap<i32, KittyConnectionStatus>>>,
    // Keep instance guards even after connection reaping. A reused PID must
    // not inherit an old run's credential or outstanding font-restoration debt.
    targets: Mutex<HashMap<i32, Arc<TargetPublicKey>>>,
    config: RegistryConfig,
}

impl KittyRegistry {
    pub fn new(config: RegistryConfig) -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            statuses: Arc::new(Mutex::new(HashMap::new())),
            targets: Mutex::new(HashMap::new()),
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
            eprintln!("Kitty PID {kitty_pid}: target socket is absent");
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
            Err(error) => {
                eprintln!(
                    "Kitty PID {kitty_pid}: password file unavailable ({:?})",
                    error.kind()
                );
                self.set_status(kitty_pid, KittyConnectionStatus::NotConfigured)
                    .await;
                return Ok(ZoomingResult::NotConfigured);
            }
        };

        let mut last_failure = None;

        for attempt in 0..self.config.max_retries {
            if attempt > 0 {
                let delay = match attempt {
                    1 => Duration::ZERO,
                    2 => Duration::from_millis(100),
                    _ => Duration::from_millis(200),
                };
                sleep(delay).await;
            }

            let lease = match self
                .get_or_create_connection(kitty_pid, &socket_path, &password)
                .await
            {
                Ok(lease) => lease,
                Err(failure) => {
                    eprintln!(
                        "Kitty PID {kitty_pid}: connection/authentication unavailable: {failure}"
                    );
                    last_failure = Some(failure);
                    if matches!(failure, ConnectionFailure::Discovery(_)) {
                        break;
                    }
                    continue;
                }
            };

            let cmd = SetFontSizeCommand::builder()
                .size(amount)
                .increment_op(op.to_string())
                .build()
                .to_message()?;

            let mut client = lease.client.lock().await;
            if let Err(error) = lease.target.verify() {
                eprintln!("Kitty PID {kitty_pid}: refusing stale connection: {error}");
                last_failure = Some(ConnectionFailure::Discovery(error));
                break;
            }
            if self.config.verbose {
                eprintln!(
                    "Attempting font adjustment {}{} to PID {} (kitty: {})",
                    op, amount, pid, kitty_pid
                );
            }
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
                    let message = response.error.unwrap_or_default().to_ascii_lowercase();
                    let failure = if message.contains("auth")
                        || message.contains("password")
                        || message.contains("permission")
                    {
                        ConnectionFailure::RemoteAuthorization
                    } else {
                        ConnectionFailure::RemoteRefused
                    };
                    eprintln!("Kitty PID {kitty_pid}: {failure}");
                    last_failure = Some(failure);
                }
                Err(e) => {
                    let failure = ConnectionFailure::from_library(&e);
                    eprintln!("Kitty PID {kitty_pid}: font command failed: {failure}");
                    last_failure = Some(failure);
                }
            }
        }

        self.set_status(pid, KittyConnectionStatus::Failed).await;

        if last_failure.is_some_and(ConnectionFailure::is_auth) {
            return Ok(ZoomingResult::AuthFailed);
        }

        Ok(ZoomingResult::ConnectionFailed)
    }

    async fn get_or_create_connection(
        &self,
        pid: i32,
        socket_path: &PathBuf,
        password: &str,
    ) -> Result<ConnectionLease, ConnectionFailure> {
        let target = {
            let mut targets = self.targets.lock().await;
            let target = if let Some(target) = targets.get(&pid) {
                Arc::clone(target)
            } else {
                let target =
                    Arc::new(TargetPublicKey::discover(pid).map_err(ConnectionFailure::Discovery)?);
                targets.insert(pid, Arc::clone(&target));
                target
            };
            target.verify().map_err(ConnectionFailure::Discovery)?;
            target
        };
        {
            let mut connections = self.connections.lock().await;

            if let Some(conn) = connections.get_mut(&pid) {
                conn.last_used = Instant::now();
                return Ok(ConnectionLease {
                    client: Arc::clone(&conn.client),
                    target,
                });
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

        let client = match connect_target(
            &target,
            socket_path,
            self.config.socket_timeout,
            password,
        )
        .await
        {
            Ok(c) => {
                if self.config.verbose {
                    eprintln!("Successfully connected to kitty PID {}", pid);
                }
                c
            }
            Err(failure) => {
                self.set_status(pid, KittyConnectionStatus::Failed).await;
                return Err(failure);
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

        Ok(ConnectionLease {
            client: client_arc,
            target,
        })
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
        self.targets.lock().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public_key::tests::{Fixture, MASTER, key_a};

    #[test]
    fn explicit_target_key_connects_without_helper_and_ignores_caller_key() {
        let directory = tempfile::tempdir().unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "registry::tests::helper_free_connection_child",
                "--nocapture",
            ])
            .env("ZK_TEST_HELPER_FREE_CONNECT", "1")
            .env("PATH", directory.path())
            .env("KITTY_PID", "999")
            .env("KITTY_PUBLIC_KEY", "not-the-target-and-not-a-valid-key")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "isolated connection test failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn helper_free_connection_child() {
        if std::env::var_os("ZK_TEST_HELPER_FREE_CONNECT").is_none() {
            return;
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let fixture = Fixture::new();
            let target = fixture.discover().unwrap();
            assert!(target.public_key() == key_a());
            let socket_path = fixture.root.path().join(format!("kitty-{MASTER}.sock"));
            let listener = tokio::net::UnixListener::bind(&socket_path).unwrap();
            let accept = tokio::spawn(async move {
                tokio::time::timeout(Duration::from_secs(1), listener.accept())
                    .await
                    .unwrap()
                    .unwrap();
            });
            // Local fake socket only; no font command, live kitty or helper.
            let client = connect_target(
                &target,
                &socket_path,
                Duration::from_secs(1),
                "synthetic-test-password",
            )
            .await;
            assert!(
                client.is_ok(),
                "explicit target-key setup failed: {:?}",
                client.err()
            );
            accept.await.unwrap();
        });
    }

    #[test]
    fn cached_target_guard_rejects_pid_reuse_before_any_new_connection() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let fixture = Fixture::new();
            let registry = KittyRegistry::with_defaults();
            registry
                .targets
                .lock()
                .await
                .insert(MASTER, Arc::new(fixture.discover().unwrap()));
            fixture.stat(MASTER, "kitty", 1, 2000);
            let result = registry
                .get_or_create_connection(
                    MASTER,
                    &fixture.root.path().join("absent.sock"),
                    "synthetic-test-password",
                )
                .await;
            assert!(matches!(
                result,
                Err(ConnectionFailure::Discovery(DiscoveryError::ProcessChanged))
            ));
            assert!(registry.connections.lock().await.is_empty());
        });
    }

    #[test]
    fn library_failures_keep_actionable_classes_without_untrusted_error_text() {
        let errors = [
            (
                kitty_rc::KittyError::Encryption(
                    kitty_rc::EncryptionError::PublicKeyDatabaseError(
                        "private-payload-marker".into(),
                    ),
                ),
                ConnectionFailure::ExternalKeyHelper,
            ),
            (
                kitty_rc::KittyError::Protocol(kitty_rc::ProtocolError::InvalidMessageFormat(
                    "private-payload-marker".into(),
                )),
                ConnectionFailure::Protocol,
            ),
            (
                kitty_rc::KittyError::Connection(kitty_rc::ConnectionError::TimeoutError(
                    Duration::from_secs(1),
                )),
                ConnectionFailure::Timeout,
            ),
            (
                kitty_rc::KittyError::Io(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "private-payload-marker",
                )),
                ConnectionFailure::Io(std::io::ErrorKind::PermissionDenied),
            ),
        ];
        for (error, expected) in errors {
            let class = ConnectionFailure::from_library(&error);
            assert_eq!(class, expected);
            assert!(!class.to_string().contains("private-payload-marker"));
        }
    }
}
