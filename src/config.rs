use figment2::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Application ID to track (e.g., "kitty")
    #[serde(default = "default_app_id")]
    pub app_id: String,

    /// Enable verbose logging
    #[serde(default = "default_verbose")]
    pub verbose: bool,

    /// Socket timeout in seconds
    #[serde(default = "default_socket_timeout")]
    pub socket_timeout_secs: u64,

    /// Maximum connection retry attempts
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Maximum number of concurrent connections
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    /// Idle timeout for connections in seconds
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_secs: u64,

    /// Interval between reaping idle connections in seconds
    #[serde(default = "default_reap_interval")]
    pub reap_interval_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app_id: default_app_id(),
            verbose: default_verbose(),
            socket_timeout_secs: default_socket_timeout(),
            max_retries: default_max_retries(),
            max_connections: default_max_connections(),
            idle_timeout_secs: default_idle_timeout(),
            reap_interval_secs: default_reap_interval(),
        }
    }
}

impl Config {
    /// Load configuration from multiple sources in order:
    /// 1. Default values
    /// 2. Config file at $XDG_CONFIG_HOME/kitty-focus-tracker/config.toml
    /// 3. Environment variables (ZK_* prefix)
    /// 4. CLI args (if provided)
    pub fn load(args: Option<&CliArgs>) -> Result<Self, figment2::Error> {
        let mut figment = Figment::new();

        // Add config file if it exists
        if let Some(config_path) = Self::config_path() {
            if config_path.exists() {
                if let Some(path_str) = config_path.to_str() {
                    figment = figment.merge(Toml::file(path_str));
                }
            }
        }

        // Add environment variables with ZK_ prefix
        figment = figment.merge(Env::prefixed("ZK_").split("__"));

        // Add CLI args if provided
        if let Some(args) = args {
            if !args.app_id.is_empty() {
                figment = figment.merge(("app_id", &args.app_id));
            }
            if args.verbose {
                figment = figment.merge(("verbose", true));
            }
            figment = figment.merge(("socket_timeout_secs", args.socket_timeout));
            figment = figment.merge(("max_retries", args.max_retries));
            figment = figment.merge(("max_connections", args.max_connections));
            figment = figment.merge(("idle_timeout_secs", args.idle_timeout));
            figment = figment.merge(("reap_interval_secs", args.reap_interval));
        }

        figment.extract()
    }

    /// Get the path to the config file
    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join("kitty-focus-tracker").join("config.toml"))
    }

    /// Convert to RegistryConfig for KittyRegistry
    pub fn to_registry_config(&self) -> RegistryConfig {
        RegistryConfig {
            socket_timeout: Duration::from_secs(self.socket_timeout_secs),
            max_retries: self.max_retries,
            max_connections: self.max_connections,
            idle_timeout: Duration::from_secs(self.idle_timeout_secs),
            reap_interval: Duration::from_secs(self.reap_interval_secs),
            verbose: self.verbose,
        }
    }
}

/// CLI arguments subset that can override config
#[derive(Debug, Clone)]
pub struct CliArgs {
    pub app_id: String,
    pub verbose: bool,
    pub socket_timeout: u64,
    pub max_retries: u32,
    pub max_connections: usize,
    pub idle_timeout: u64,
    pub reap_interval: u64,
}

// Default value functions
fn default_app_id() -> String {
    String::from("kitty")
}

fn default_verbose() -> bool {
    false
}

fn default_socket_timeout() -> u64 {
    2
}

fn default_max_retries() -> u32 {
    3
}

fn default_max_connections() -> usize {
    10
}

fn default_idle_timeout() -> u64 {
    1800 // 30 minutes
}

fn default_reap_interval() -> u64 {
    300 // 5 minutes
}

// Re-export RegistryConfig for kitty module
#[derive(Debug, Clone)]
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
            socket_timeout: Duration::from_secs(default_socket_timeout()),
            max_retries: default_max_retries(),
            max_connections: default_max_connections(),
            idle_timeout: Duration::from_secs(default_idle_timeout()),
            reap_interval: Duration::from_secs(default_reap_interval()),
            verbose: default_verbose(),
        }
    }
}
