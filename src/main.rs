use clap::{Parser, Subcommand};
use commands::fonts::handle_font_command;
use commands::FontCommand;
use kitty::{KittyRegistry, RegistryConfig};
use kitty::resizer::KittyResizer;
use niri::registry::NiriRegistry;
use std::io::Write;

mod commands;
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

fn print_systemd_service(output: bool) -> std::io::Result<()> {
    let service_name = std::env::var("ZOOMING_APPNAME").ok().unwrap_or_else(|| "zooming-kittens".to_string());
    let _description = format!("{} Focus Tracker", service_name);
    let binary_path = std::env::current_exe()?;
    let binary_path = binary_path.to_str().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "kitty-focus-tracker"))?;

    if output {
        std::io::stdout().write_all(b"[Unit]\n").unwrap();
        std::io::stdout().write_all(format!("Description={}\n", _description).as_bytes()).unwrap();
        std::io::stdout().write_all(b"After=niri.target\n").unwrap();
        std::io::stdout().write_all(b"Wants=niri.target\n").unwrap();
        std::io::stdout().write_all(b"\n").unwrap();
        std::io::stdout().write_all(b"[Service]\n").unwrap();
        std::io::stdout().write_all(b"Type=simple\n").unwrap();
        std::io::stdout().write_all(b"ExecStart=").unwrap();
        std::io::stdout().write_all(binary_path.as_bytes()).unwrap();
        std::io::stdout().write_all(b"\n").unwrap();
        std::io::stdout().write_all(b"Environment=RUST_BACKTRACE=full\n").unwrap();
        std::io::stdout().write_all(b"Restart=always\n").unwrap();
        std::io::stdout().write_all(b"\n").unwrap();
        std::io::stdout().write_all(b"[Install]\n").unwrap();
        std::io::stdout().write_all(b"WantedBy=default.target\n").unwrap();
    }
    Ok(())
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    
    // Set default app_id from ZOOMING_APPNAME env var if not provided
    let app_id = if args.app_id.is_empty() {
        match std::env::var("ZOOMING_APPNAME") {
            Ok(val) => val,
            Err(_) => String::from("kitty"),
        }
    } else {
        args.app_id.as_str().to_string()
    };
    // Handle subcommands
    if let Some(CliSubcommand::GenerateSystemd { output }) = args.command {
        print_systemd_service(output)?;
        return Ok(());
    }

    if let Some(CliSubcommand::Cleanup) = args.command {
        let config = RegistryConfig::default();
        let registry = KittyRegistry::new(config);
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

    if args.verbose {
        eprintln!("Starting event stream for window focus changes...");
    }
    
    if args.verbose {
        eprintln!("Tracking app_id: {}", app_id);
    }
    
    let config = RegistryConfig {
        socket_timeout: std::time::Duration::from_secs(args.socket_timeout),
        max_retries: args.max_retries,
        max_connections: args.max_connections,
        idle_timeout: std::time::Duration::from_secs(args.idle_timeout),
        reap_interval: std::time::Duration::from_secs(args.reap_interval),
        verbose: args.verbose,
    };

    let kitty_registry = KittyRegistry::new(config);
    kitty_registry.start_reaper().await;

    if args.verbose {
        eprintln!("Tracking app_id: {}", app_id);
    }

    let niri_registry = NiriRegistry::new().await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    let mut kitty_resizer = KittyResizer::new(kitty_registry);

    let kitty_events = niri_registry.windows_matching(|window| {
        window.app_id.as_deref() == Some(&app_id)
    });

    let _ = kitty_resizer.process_events(kitty_events).await;

    Ok(())
}
