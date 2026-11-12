use figment2::{
    Figment,
    providers::{Env, Format, Toml},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[cfg(feature = "kitty")]
use std::time::Duration;
#[cfg(feature = "kitty")]
pub use zk_kitty::RegistryConfig;
#[cfg(feature = "kitty")]
pub use zk_zoomer::ZoomConfig;
#[cfg(all(feature = "kitty", any(test, feature = "niri", feature = "umbriel")))]
pub use zk_zoomer::ZoomType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Verbosity {
    #[default]
    Quiet = 0,
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

impl Verbosity {
    pub fn from_count(count: u8) -> Self {
        match count {
            0 => Self::Quiet,
            1 => Self::Error,
            2 => Self::Warn,
            3 => Self::Info,
            4 => Self::Debug,
            5 => Self::Trace,
            _ => Self::Trace,
        }
    }

    #[cfg(all(any(feature = "niri", feature = "umbriel"), feature = "kitty"))]
    pub fn log_window_events(&self) -> bool {
        *self >= Self::Debug
    }
}

/// CLI arguments subset that can override config
#[derive(Debug, Clone)]
pub struct CliArgs {
    pub app_id: String,
    pub verbosity: Verbosity,
    pub socket_timeout: u64,
    pub max_retries: u32,
    pub max_connections: usize,
    pub idle_timeout: u64,
    pub reap_interval: u64,
}

fn default_app_id() -> String {
    String::from("kitty")
}

fn default_verbose() -> bool {
    false
}

fn default_socket_timeout() -> u64 {
    5
}

fn default_max_retries() -> u32 {
    3
}

fn default_max_connections() -> usize {
    30
}

fn default_idle_timeout() -> u64 {
    1800 // 30 minutes
}

fn default_reap_interval() -> u64 {
    300 // 5 minutes
}

/// Longcat configuration for managing window heights in columns
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LongcatConfig {
    /// Height proportion for active window (default: 0.5)
    #[serde(default = "default_active_height_proportion")]
    pub active_height_proportion: f64,

    /// Height proportion for other windows in the column (default: 0.5)
    #[serde(default = "default_other_height_proportion")]
    pub other_height_proportion: f64,

    /// Minimum height for any window in pixels (default: 60)
    #[serde(default = "default_minimum_height")]
    pub minimum_height: i32,

    /// Minimum width for horizontal secondary sizing (default: 60).
    pub minimum_width: i32,

    /// Debounce time in ms before detecting manual resize (default: 500)
    #[serde(default = "default_resize_debounce_ms")]
    pub resize_debounce_ms: u64,
}

impl Default for LongcatConfig {
    fn default() -> Self {
        Self {
            active_height_proportion: default_active_height_proportion(),
            other_height_proportion: default_other_height_proportion(),
            minimum_height: default_minimum_height(),
            minimum_width: default_minimum_height(),
            resize_debounce_ms: default_resize_debounce_ms(),
        }
    }
}

fn default_active_height_proportion() -> f64 {
    0.7
}

fn default_other_height_proportion() -> f64 {
    0.3
}

fn default_minimum_height() -> i32 {
    60
}

fn default_resize_debounce_ms() -> u64 {
    500
}

/// CLI arguments subset for zoom configuration
#[derive(Debug, Clone, Default)]
#[cfg_attr(not(feature = "kitty"), allow(dead_code))]
pub struct CliZoomArgs {
    pub absolute: Option<f64>,
    pub additive: Option<f64>,
    pub multiplicative: Option<f64>,
    pub step_size: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Application ID to track (e.g., "kitty")
    #[serde(default = "default_app_id")]
    pub app_id: String,

    /// Enable verbose logging
    #[serde(default = "default_verbose")]
    pub verbose: bool,

    /// Verbosity level (0-5, higher = more verbose)
    #[serde(skip)]
    pub verbosity: Verbosity,

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

    /// Zoom configuration
    #[cfg(feature = "kitty")]
    pub zoom: ZoomConfig,

    /// Longcat configuration for window height management
    pub longcat: LongcatConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app_id: default_app_id(),
            verbose: default_verbose(),
            verbosity: Verbosity::Info,
            socket_timeout_secs: default_socket_timeout(),
            max_retries: default_max_retries(),
            max_connections: default_max_connections(),
            idle_timeout_secs: default_idle_timeout(),
            reap_interval_secs: default_reap_interval(),
            #[cfg(feature = "kitty")]
            zoom: ZoomConfig::default(),
            longcat: LongcatConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from multiple sources in order:
    /// 1. Default values
    /// 2. Config file at $XDG_CONFIG_HOME/kitty-focus-tracker/config.toml
    /// 3. Environment variables (ZK_* prefix)
    /// 4. CLI args (if provided)
    pub fn load(
        args: Option<&CliArgs>,
        zoom_args: Option<&CliZoomArgs>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut figment = Figment::new();

        // Add config file if it exists
        if let Some(config_path) = Self::config_path()
            && config_path.exists()
            && let Some(path_str) = config_path.to_str()
        {
            figment = figment.merge(Toml::file(path_str));
        }

        // Add environment variables with ZK_ prefix
        figment = figment.merge(Env::prefixed("ZK_").split("__"));

        // Add CLI args if provided
        if let Some(args) = args {
            if !args.app_id.is_empty() {
                figment = figment.merge(("app_id", &args.app_id));
            }
            if args.verbosity != Verbosity::default() {
                figment = figment.merge(("verbose", true));
            }
            figment = figment.merge(("socket_timeout_secs", args.socket_timeout));
            figment = figment.merge(("max_retries", args.max_retries));
            figment = figment.merge(("max_connections", args.max_connections));
            figment = figment.merge(("idle_timeout_secs", args.idle_timeout));
            figment = figment.merge(("reap_interval_secs", args.reap_interval));
        }

        // Extract base config
        let config: Config = figment.extract()?;
        #[cfg(feature = "kitty")]
        let mut config = config;
        #[cfg(not(feature = "kitty"))]
        let _ = zoom_args;

        // Validate zoom config from file
        #[cfg(feature = "kitty")]
        config
            .zoom
            .validate()
            .map_err(|e| format!("Invalid zoom configuration: {}", e))?;

        // Apply CLI zoom args if provided (overrides config file)
        #[cfg(feature = "kitty")]
        if let Some(zoom) = zoom_args {
            if zoom.absolute.is_some() {
                config.zoom.absolute = zoom.absolute;
                config.zoom.additive = None;
                config.zoom.multiplicative = None;
            }
            if zoom.additive.is_some() {
                config.zoom.additive = zoom.additive;
                config.zoom.absolute = None;
                config.zoom.multiplicative = None;
            }
            if zoom.multiplicative.is_some() {
                config.zoom.multiplicative = zoom.multiplicative;
                config.zoom.absolute = None;
                config.zoom.additive = None;
            }
            if let Some(step_size) = zoom.step_size {
                config.zoom.step_size = step_size;
            }

            // Validate after CLI overrides
            config
                .zoom
                .validate()
                .map_err(|e| format!("Invalid zoom configuration: {}", e))?;
        }

        Ok(config)
    }

    /// Get path to config file
    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join("kitty-focus-tracker").join("config.toml"))
    }

    /// Convert to RegistryConfig for KittyRegistry
    #[cfg(feature = "kitty")]
    pub fn to_registry_config(&self) -> RegistryConfig {
        RegistryConfig {
            socket_timeout: Duration::from_secs(self.socket_timeout_secs),
            max_retries: self.max_retries,
            max_connections: self.max_connections,
            idle_timeout: Duration::from_secs(self.idle_timeout_secs),
            reap_interval: Duration::from_secs(self.reap_interval_secs),
            verbose: self.verbose || self.verbosity >= Verbosity::Debug,
        }
    }
}

#[cfg(all(test, feature = "kitty"))]
mod tests {
    use super::*;

    #[test]
    fn test_zoom_config_validate_single_type() {
        let config = ZoomConfig {
            additive: Some(6.0),
            ..ZoomConfig::default()
        };
        assert!(config.validate().is_ok());
        assert_eq!(config.active_type(), Some(ZoomType::Additive));
        assert_eq!(config.value(), Some(6.0));
    }

    #[test]
    fn test_zoom_config_validate_multiple_types_error() {
        let config = ZoomConfig {
            additive: Some(6.0),
            multiplicative: Some(1.5),
            ..ZoomConfig::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_zoom_config_validate_absolute() {
        let config = ZoomConfig {
            absolute: Some(18.0),
            ..ZoomConfig::default()
        };
        assert!(config.validate().is_ok());
        assert_eq!(config.active_type(), Some(ZoomType::Absolute));
        assert_eq!(config.value(), Some(18.0));
    }

    #[test]
    fn test_zoom_config_validate_multiplicative() {
        let config = ZoomConfig {
            multiplicative: Some(1.5),
            ..ZoomConfig::default()
        };
        assert!(config.validate().is_ok());
        assert_eq!(config.active_type(), Some(ZoomType::Multiplicative));
        assert_eq!(config.value(), Some(1.5));
    }

    #[test]
    fn test_zoom_config_no_type() {
        let config = ZoomConfig::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.active_type(), None);
        assert_eq!(config.value(), None);
    }
}
