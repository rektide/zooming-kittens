use clap::{Parser, Subcommand};
use commands::FontCommand;
use commands::fonts::handle_font_command;
use commands::systemd::generate_systemd_service;
use commands::zoomer::run_zoomer;
use config::{CliArgs, Config, Verbosity};
use kitty::KittyRegistry;
use kitty::resizer::KittyResizer;
use niri::registry::NiriRegistry;

mod commands;
mod config;
mod kitty;
mod niri;

#[derive(Subcommand, Debug)]
enum CliSubcommand {
    #[command(
        name = "generate-systemd",
        about = "Generate a systemd service file for auto-startup"
    )]
    GenerateSystemd {
        #[arg(short, long, help = "Print the systemd service file to stdout")]
        output: bool,
    },
    #[command(
        name = "cleanup",
        about = "Clean up dead connections in the connection pool"
    )]
    Cleanup,
    #[command(
        name = "zoomer",
        about = "Run focus tracking for a specific app with +6/-6 font adjustments"
    )]
    Zoomer {
        #[arg(
            short,
            long,
            default_value = "kitty",
            help = "Application ID to track (e.g., 'kitty')"
        )]
        app_id: String,

        #[arg(short, long = "verbose", action = clap::ArgAction::Count, help = "Increase verbosity level (use -v, -vv, -vvv, -vvvv)")]
        verbose_count: u8,
    },
    #[command(subcommand)]
    #[command(about = "Manually control kitty font sizes")]
    Font(FontCommand),
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Track niri window focus and adjust kitty terminal font sizes", long_about = None)]
struct Args {
    #[arg(
        short,
        long,
        default_value = "kitty",
        long,
        help = "Application ID to track when running in default mode"
    )]
    app_id: String,

    #[arg(short, long = "verbose", action = clap::ArgAction::Count, help = "Increase verbosity level (use -v, -vv, -vvv, -vvvv)")]
    verbose_count: u8,

    #[arg(long, default_value = "5", long, help = "Socket timeout in seconds")]
    socket_timeout: u64,

    #[arg(
        long,
        default_value = "3",
        long,
        help = "Maximum connection retry attempts"
    )]
    max_retries: u32,

    #[arg(
        long,
        default_value = "30",
        long,
        help = "Maximum number of concurrent connections"
    )]
    max_connections: usize,

    #[arg(
        long,
        default_value = "1800",
        long,
        help = "Idle connection timeout in seconds"
    )]
    idle_timeout: u64,

    #[arg(
        long,
        default_value = "300",
        long,
        help = "Connection pool reaping interval in seconds"
    )]
    reap_interval: u64,

    #[command(subcommand)]
    command: Option<CliSubcommand>,
}

 impl Args {
    /// Convert to CliArgs for config loading
    fn to_cli_args(&self) -> CliArgs {
        CliArgs {
            app_id: self.app_id.clone(),
            verbosity: Verbosity::from_count(self.verbose_count),
            socket_timeout: self.socket_timeout,
            max_retries: self.max_retries,
            max_connections: self.max_connections,
            idle_timeout: self.idle_timeout,
            reap_interval: self.reap_interval,
        }
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let cli_args = args.to_cli_args();

    // Handle subcommands
    if let Some(CliSubcommand::GenerateSystemd { output }) = args.command {
        generate_systemd_service(output)?;
        return Ok(());
    }

    if let Some(CliSubcommand::Cleanup) = args.command {
        let config = Config::load(None).unwrap_or_default();
        let registry = KittyRegistry::new(config.to_registry_config());
        registry.cleanup_dead_connections().await;
        eprintln!("Cleanup complete");
        return Ok(());
    }

    if let Some(CliSubcommand::Zoomer {
        app_id: zoomer_app_id,
        verbose_count,
    }) = args.command
    {
        let config = Config::load(Some(&cli_args)).unwrap_or_default();
        let verbosity = Verbosity::from_count(args.verbose_count + verbose_count);
        run_zoomer(zoomer_app_id, verbosity, config.to_registry_config())
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        return Ok(());
    }

    if let Some(CliSubcommand::Font(font_cmd)) = args.command {
        handle_font_command(font_cmd)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        return Ok(());
    }

    // Load config from file, env, and CLI args
    let config = Config::load(Some(&cli_args)).unwrap_or_else(|e| {
        eprintln!("Config error: {}, using defaults", e);
        Config::default()
    });

    let app_id = config.app_id.clone();
    let verbosity = config.verbosity;

    if verbosity.log_window_events() {
        eprintln!("Starting event stream for window focus changes...");
        eprintln!("Tracking app_id: {}", app_id);
    }

    let kitty_registry = KittyRegistry::new(config.to_registry_config());
    kitty_registry.start_reaper().await;

    let niri_registry = NiriRegistry::new_with_verbosity(verbosity)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    let mut kitty_resizer = KittyResizer::new(kitty_registry);

    let kitty_events =
        niri_registry.windows_matching(|window| window.app_id.as_deref() == Some(&app_id));

    let _ = kitty_resizer.process_events(kitty_events).await;

    Ok(())
}
