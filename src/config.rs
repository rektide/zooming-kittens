use figment2::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

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

    pub fn log_all_events(&self) -> bool {
        *self >= Self::Trace
    }

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

fn default_step_size() -> u32 {
    1
}

/// Zoom type: absolute, additive, or multiplicative
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZoomType {
    /// Set to an absolute font size
    Absolute,
    /// Add to current font size (+N)
    Additive,
    /// Multiply current font size (*N)
    Multiplicative,
}

/// Zoom configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ZoomConfig {
    /// Absolute font size to set on focus
    pub absolute: Option<f64>,

    /// Additive amount (e.g., 6 means +6 on focus, -6 on blur)
    pub additive: Option<f64>,

    /// Multiplicative factor (e.g., 1.5 means *1.5 on focus, /1.5 on blur)
    pub multiplicative: Option<f64>,

    /// Number of steps to apply at once
    #[serde(default = "default_step_size")]
    pub step_size: u32,
}

impl Default for ZoomConfig {
    fn default() -> Self {
        Self {
            absolute: None,
            additive: None,
            multiplicative: None,
            step_size: default_step_size(),
        }
    }
}

impl ZoomConfig {
    /// Validate that only one zoom type is configured
    /// Returns an error if multiple zoom types are set
    pub fn validate(&self) -> Result<(), String> {
        let set_types = [
            self.absolute.is_some(),
            self.additive.is_some(),
            self.multiplicative.is_some(),
        ];

        let count = set_types.iter().filter(|&&x| x).count();

        if count > 1 {
            let mut types = Vec::new();
            if self.absolute.is_some() {
                types.push("absolute");
            }
            if self.additive.is_some() {
                types.push("additive");
            }
            if self.multiplicative.is_some() {
                types.push("multiplicative");
            }
            return Err(format!(
                "Multiple zoom types configured: {}. Only one zoom type may be set.",
                types.join(", ")
            ));
        }

        Ok(())
    }

    /// Get the active zoom type
    pub fn active_type(&self) -> Option<ZoomType> {
        if self.absolute.is_some() {
            Some(ZoomType::Absolute)
        } else if self.additive.is_some() {
            Some(ZoomType::Additive)
        } else if self.multiplicative.is_some() {
            Some(ZoomType::Multiplicative)
        } else {
            None
        }
    }

    /// Get the zoom value for the active type
    pub fn value(&self) -> Option<f64> {
        match self.active_type() {
            Some(ZoomType::Absolute) => self.absolute,
            Some(ZoomType::Additive) => self.additive,
            Some(ZoomType::Multiplicative) => self.multiplicative,
            None => None,
        }
    }

    /// Check if this config has any zoom type set
    pub fn is_configured(&self) -> bool {
        self.active_type().is_some()
    }
}

/// CLI arguments subset for zoom configuration
#[derive(Debug, Clone, Default)]
pub struct CliZoomArgs {
    pub absolute: Option<f64>,
    pub additive: Option<f64>,
    pub multiplicative: Option<f64>,
    pub step_size: Option<u32>,
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
    pub zoom: ZoomConfig,
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
            zoom: ZoomConfig::default(),
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
        let mut config: Config = figment.extract()?;

        // Validate zoom config from file
        config
            .zoom
            .validate()
            .map_err(|e| format!("Invalid zoom configuration: {}", e))?;

        // Apply CLI zoom args if provided (overrides config file)
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
            if zoom.step_size.is_some() {
                config.zoom.step_size = zoom.step_size.unwrap();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zoom_config_validate_single_type() {
        let mut config = ZoomConfig::default();
        config.additive = Some(6.0);
        assert!(config.validate().is_ok());
        assert_eq!(config.active_type(), Some(ZoomType::Additive));
        assert_eq!(config.value(), Some(6.0));
    }

    #[test]
    fn test_zoom_config_validate_multiple_types_error() {
        let mut config = ZoomConfig::default();
        config.additive = Some(6.0);
        config.multiplicative = Some(1.5);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_zoom_config_validate_absolute() {
        let mut config = ZoomConfig::default();
        config.absolute = Some(18.0);
        assert!(config.validate().is_ok());
        assert_eq!(config.active_type(), Some(ZoomType::Absolute));
        assert_eq!(config.value(), Some(18.0));
    }

    #[test]
    fn test_zoom_config_validate_multiplicative() {
        let mut config = ZoomConfig::default();
        config.multiplicative = Some(1.5);
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
