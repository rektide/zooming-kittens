use clap::{Parser, Subcommand};
use niri_ipc::socket::Socket;
use niri_ipc::{Request, Response};
use registry::{FocusTracker, KittyRegistry, RegistryConfig};
use serde::Serialize;
use std::io::Write;

mod registry;

#[derive(Subcommand, Debug)]
enum CliSubcommand {
    #[command(name = "generate-systemd")]
    GenerateSystemd {
        #[arg(short, long)]
        output: bool,
    },
    #[command(name = "cleanup")]
    Cleanup,
}

#[derive(Serialize)]
#[serde(tag = "event")]
enum FocusEvent {
    #[serde(rename = "focus_gained")]
    FocusGained {
        window_id: u64,
        app_id: String,
        zooming: registry::ZoomingResult,
    },
    #[serde(rename = "focus_lost")]
    FocusLost {
        zooming: Option<registry::ZoomingResult>,
    },
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

struct KittyWindow {
    app_id: String,
    pid: Option<i32>,
}

fn is_kitty_window(app_id: &str, target_app_id: &str) -> bool {
    app_id == target_app_id
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
    
    let registry = KittyRegistry::new(config);
    registry.start_reaper().await;
    
    let mut focus_tracker = FocusTracker::new();
    
    // Debounce focus changes to avoid rapid font adjustments
    const FOCUS_DEBOUNCE_MS: u64 = 100;
    let mut last_focus_time: Option<std::time::Instant> = None;
    
    fn should_handle_focus_change(last_focus_time: &Option<std::time::Instant>) -> bool {
        match last_focus_time {
            Some(last) => last.elapsed().as_millis() as u64 > FOCUS_DEBOUNCE_MS,
            None => true,
        }
    }
    
    if args.verbose {
        eprintln!("Tracking app_id: {}", app_id);
    }
    
    let mut socket = Socket::connect()?;
    
    if args.verbose {
        eprintln!("Requesting event stream...");
    }
    
    let reply = socket.send(Request::EventStream)?;
    
    if !matches!(reply, Ok(Response::Handled)) {
        eprintln!("Failed to get event stream: {:?}", reply);
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to get event stream",
        ));
    }
    
    if args.verbose {
        eprintln!("Listening for events...");
    }
    
    let mut read_event = socket.read_events();
    
    loop {
        match read_event() {
            Ok(event) => match event {
                niri_ipc::Event::WindowFocusTimestampChanged { id, focus_timestamp: _timestamp } => {
                    let should_handle = should_handle_focus_change(&last_focus_time);
                    if !should_handle {
                        if args.verbose {
                            eprintln!("Debouncing focus change for window {}", id);
                        }
                        continue;
                    }

                    
                    let mut socket_query = Socket::connect()?;
                    let reply = socket_query.send(Request::Windows)?;
                    
                    let window = match reply {
                        Ok(Response::Windows(windows)) => {
                            windows.iter().find(|w| w.id == id).cloned()
                        }
                        _ => None,
                    };
                    
                    if let Some(w) = window {
                        if let Some(ref app_id) = w.app_id {
                            if is_kitty_window(app_id, &app_id) {
                                if args.verbose {
                                    eprintln!(
                                        "Window {} gained focus (app_id: {}, pid: {:?})",
                                        id, app_id, w.pid
                                    );
                                }
                                
                                if let Some(prev_pid) = focus_tracker.on_focus_lost() {
                                    if args.verbose {
                                        eprintln!("Decreasing font size for previously focused kitty PID {}", prev_pid);
                                    }
                                    
                                    match registry.decrease_font_size(prev_pid).await {
                                        Ok(result) => {
                                            let event = FocusEvent::FocusLost { zooming: Some(result) };
                                            println!("{}", serde_json::to_string(&event).unwrap());
                                        }
                                        Err(e) => {
                                            eprintln!("Error adjusting font size: {}", e);
                                            let event = FocusEvent::FocusLost { zooming: Some(registry::ZoomingResult::Failed) };
                                            println!("{}", serde_json::to_string(&event).unwrap());
                                        }
                                    }
                                }
                                
                                focus_tracker.on_focus_gained(w.pid.unwrap_or(0));
                                last_focus_time = Some(std::time::Instant::now());
                                
                                let zooming_result = if let Some(p) = w.pid {
                                    if args.verbose {
                                        eprintln!("Increasing font size for kitty PID {}", p);
                                    }
                                    
                                    match registry.increase_font_size(p).await {
                                        Ok(result) => result,
                                        Err(e) => {
                                            eprintln!("Error adjusting font size: {}", e);
                                            registry::ZoomingResult::Failed
                                        }
                                    }
                                } else {
                                    registry::ZoomingResult::NotConfigured
                                };
                                
                                let event = FocusEvent::FocusGained {
                                    window_id: id,
                                    app_id: app_id.clone(),
                                    zooming: zooming_result,
                                };
                                println!("{}", serde_json::to_string(&event).unwrap());
                            }
                        }
                    }
                }
                _ => {}
            },
            Err(e) => {
                eprintln!("Error reading event: {:?}", e);
                registry.shutdown().await;
                return Err(e);
            }
        }
    }
}
