use crate::config::{Config, LongcatConfig};
use clap::{Args, ValueEnum};
use std::{io, path::PathBuf};

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum Backend {
    Niri,
    Umbriel,
}

#[derive(Debug, Clone, Args)]
pub struct LongcatCommand {
    #[arg(long, value_enum, default_value = "niri")]
    pub backend: Backend,
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
    /// Retained config value; manual resize learning is not enabled.
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
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Longcat weights must be positive finite values with a finite sum, and minimum-height must be positive",
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
    // No global CLI defaults are merged: Longcat overrides only explicitly supplied values.
    let loaded = Config::load(None, None)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
    let config = command.compose(loaded.longcat)?;
    if command.backend == Backend::Umbriel && !command.dry_run {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Umbriel mutation unsupported: native extent actions target the focused window, not a chosen ID; exact groups/axis and reversible layout state are unavailable. Use --dry-run; no focus workaround is performed.",
        ));
    }
    crate::runner::run(command, config).await
}
