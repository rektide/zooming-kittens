use clap::{Parser, Subcommand};
use commands::fonts::handle_font_command;
use commands::systemd::generate_systemd_service;
use commands::FontCommand;
use config::{Config, CliArgs};
use kitty::KittyRegistry;
use kitty::resizer::KittyResizer;
use niri::registry::NiriRegistry;

mod commands;
mod config;
mod kitty;
mod niri;

#[derive(Subcommand, Debug)]
enum CliSubcommand {
    #[command(name = "generate-systemd")]
    GenerateSystemd {
        #[arg(short, long)]
        output: bool,
    },
    #[command(name = "cleanup")]
    Cleanup,
    #[command(subcommand)]
    Font(FontCommand),
}



#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "")]
    app_id: String,

    #[arg(short, long)]
    verbose: bool,

    #[arg(long, default_value = "2")]
    socket_timeout: u64,

    #[arg(long, default_value = "3")]
    max_retries: u32,

    #[arg(long, default_value = "10")]
    max_connections: usize,

    #[arg(long, default_value = "1800")]
    idle_timeout: u64,

    #[arg(long, default_value = "300")]
    reap_interval: u64,

    #[command(subcommand)]
    command: Option<CliSubcommand>,
}

impl Args {
    /// Convert to CliArgs for config loading
    fn to_cli_args(&self) -> CliArgs {
        // Handle ZOOMING_APPNAME env var for backwards compatibility
        let app_id = if self.app_id.is_empty() {
            match std::env::var("ZOOMING_APPNAME") {
                Ok(val) => val,
                Err(_) => String::from("kitty"),
            }
        } else {
            self.app_id.clone()
        };

        CliArgs {
            app_id,
            verbose: self.verbose,
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

    if let Some(CliSubcommand::Font(font_cmd)) = args.command {
        handle_font_command(font_cmd).await.map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
        })?;
        return Ok(());
    }

    // Load config from file, env, and CLI args
    let config = Config::load(Some(&cli_args)).unwrap_or_else(|e| {
        eprintln!("Config error: {}, using defaults", e);
        Config::default()
    });

    let app_id = config.app_id.clone();

    if config.verbose {
        eprintln!("Starting event stream for window focus changes...");
        eprintln!("Tracking app_id: {}", app_id);
    }

    let kitty_registry = KittyRegistry::new(config.to_registry_config());
    kitty_registry.start_reaper().await;

    let niri_registry = NiriRegistry::new().await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    let mut kitty_resizer = KittyResizer::new(kitty_registry);

    let kitty_events = niri_registry.windows_matching(|window| {
        window.app_id.as_deref() == Some(&app_id)
    });

    let _ = kitty_resizer.process_events(kitty_events).await;

    Ok(())
}
