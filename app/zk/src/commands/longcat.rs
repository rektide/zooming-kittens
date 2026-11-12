use crate::config::{Config, LongcatConfig};
use clap::{Args, ValueEnum};
use std::{io, path::PathBuf};

pub use super::backend::Backend;

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq, Default)]
pub enum UmbrielControl {
    /// Stock snapshots and verified, qualified internal-edge feedback.
    #[default]
    Stock,
    /// Explicit bespoke layout-state/layout-allocate conditional control.
    Conditional,
}

#[derive(Debug, Clone, Args)]
pub struct LongcatCommand {
    #[arg(long, value_enum, default_value = "niri")]
    pub backend: Backend,
    #[arg(long, value_enum, default_value = "stock")]
    pub umbriel_control: UmbrielControl,
    /// Observe and preview decisions without sending native commands.
    #[arg(long)]
    pub dry_run: bool,
    /// Native Unix socket; otherwise use the backend's environment discovery.
    #[arg(long)]
    pub socket: Option<PathBuf>,
    /// Stop after this many ready capture/decision records (startup waits do not count).
    #[arg(long, value_parser = clap::value_parser!(u64).range(1..))]
    pub limit: Option<u64>,
    /// Relative active weight; normalized against the other weight.
    #[arg(long)]
    pub active_height_proportion: Option<f64>,
    #[arg(long)]
    pub other_height_proportion: Option<f64>,
    #[arg(long)]
    pub minimum_height: Option<i32>,
    #[arg(long)]
    pub minimum_width: Option<i32>,
    /// Quiet interval before stock acquisition or resize guidance.
    #[arg(long)]
    pub resize_debounce_ms: Option<u64>,
    /// Deadline for execution and observed convergence, independent of resize debounce.
    #[arg(long, default_value = "1500", value_parser = clap::value_parser!(u64).range(1..=60000))]
    pub convergence_timeout_ms: u64,
    /// Conditional restoration and command-evidence drain budget at shutdown.
    #[arg(long, default_value = "2500", value_parser = clap::value_parser!(u64).range(1..=60000))]
    pub shutdown_timeout_ms: u64,
    /// Suppress diagnostic output (errors are still returned).
    #[arg(short, long)]
    pub quiet: bool,
    #[arg(short, long = "verbose", action = clap::ArgAction::Count)]
    pub verbose_count: u8,
}

impl LongcatCommand {
    pub fn compose(&self, mut config: LongcatConfig) -> io::Result<LongcatConfig> {
        if let Some(value) = self.active_height_proportion {
            config.active_height_proportion = value;
        }
        if let Some(value) = self.other_height_proportion {
            config.other_height_proportion = value;
        }
        if let Some(value) = self.minimum_height {
            config.minimum_height = value;
        }
        if let Some(value) = self.minimum_width {
            config.minimum_width = value;
        }
        if let Some(value) = self.resize_debounce_ms {
            config.resize_debounce_ms = value;
        }
        let a = config.active_height_proportion;
        let b = config.other_height_proportion;
        if !a.is_finite()
            || !b.is_finite()
            || a <= 0.0
            || b <= 0.0
            || !(a + b).is_finite()
            || config.minimum_height <= 0
            || config.minimum_width <= 0
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Longcat weights must be positive finite values with a finite sum; minimum-height must be positive; minimum-width must be positive",
            ));
        }
        let share = a / (a + b);
        if share <= 0.0 || share >= 1.0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "normalized active share is not representable strictly between zero and one",
            ));
        }
        Ok(config)
    }
}

pub async fn run_longcat(command: LongcatCommand) -> io::Result<()> {
    command.backend.require()?;
    // No global CLI defaults are merged: Longcat overrides only explicitly supplied values.
    let loaded = Config::load(None, None)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
    let config = command.compose(loaded.longcat)?;
    #[cfg(any(feature = "niri", feature = "umbriel"))]
    {
        crate::runner::run(command, config).await
    }
    #[cfg(not(any(feature = "niri", feature = "umbriel")))]
    {
        let _ = config;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Longcat requires feature `niri` or `umbriel`",
        ))
    }
}
