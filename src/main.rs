use clap::Parser;
use niri_ipc::socket::Socket;
use niri_ipc::{Request, Response};
use registry::{FocusTracker, KittyRegistry, RegistryConfig};
use serde::Serialize;
use std::collections::HashMap;

mod registry;

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
    #[arg(short, long, default_value = "kitty")]
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
}

struct KittyWindow {
    app_id: String,
    pid: Option<i32>,
}

fn is_kitty_window(app_id: &str, target_app_id: &str) -> bool {
    app_id == target_app_id
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    if args.verbose {
        eprintln!("Connecting to niri IPC socket...");
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

    let config = RegistryConfig {
        socket_timeout: std::time::Duration::from_secs(args.socket_timeout),
        max_retries: args.max_retries,
        max_connections: args.max_connections,
        idle_timeout: std::time::Duration::from_secs(args.idle_timeout),
        reap_interval: std::time::Duration::from_secs(args.reap_interval),
    };

    let registry = KittyRegistry::new(config);
    registry.start_reaper().await;

    let mut focus_tracker = FocusTracker::new();

    if args.verbose {
        eprintln!("Listening for window focus events...");
        eprintln!("Tracking app_id: {}", args.app_id);
    }

    let mut windows: HashMap<u64, KittyWindow> = HashMap::new();
    let mut read_event = socket.read_events();

    loop {
        match read_event() {
            Ok(event) => match event {
                niri_ipc::Event::WindowOpenedOrChanged { window } => {
                    if let Some(ref app_id) = window.app_id {
                        let kitty_window = KittyWindow {
                            app_id: app_id.clone(),
                            pid: window.pid,
                        };
                        windows.insert(window.id, kitty_window);
                        if args.verbose {
                            eprintln!(
                                "Window {} opened with app_id: {}, pid: {:?}",
                                window.id, app_id, window.pid
                            );
                        }
                    }
                }
                niri_ipc::Event::WindowClosed { id } => {
                    windows.remove(&id);
                    if args.verbose {
                        eprintln!("Window {} closed", id);
                    }
                }
                niri_ipc::Event::WindowFocusChanged { id } => match id {
                    Some(focused_id) => {
                        if let Some(window) = windows.get(&focused_id) {
                            if is_kitty_window(&window.app_id, &args.app_id) {
                                let zooming_result = if let Some(pid) = window.pid {
                                    if args.verbose {
                                        eprintln!("Increasing font size for kitty PID {}", pid);
                                    }

                                    match registry.increase_font_size(pid).await {
                                        Ok(result) => result,
                                        Err(e) => {
                                            eprintln!("Error adjusting font size: {}", e);
                                            registry::ZoomingResult::Failed
                                        }
                                    }
                                } else {
                                    registry::ZoomingResult::NotConfigured
                                };

                                focus_tracker.on_focus_gained(window.pid.unwrap_or(0));

                                let event = FocusEvent::FocusGained {
                                    window_id: focused_id,
                                    app_id: window.app_id.clone(),
                                    zooming: zooming_result,
                                };
                                println!("{}", serde_json::to_string(&event).unwrap());
                            }
                        }
                    }
                    None => {
                        let zooming_result = if let Some(pid) = focus_tracker.on_focus_lost() {
                            if args.verbose {
                                eprintln!("Decreasing font size for kitty PID {}", pid);
                            }

                            match registry.decrease_font_size(pid).await {
                                Ok(result) => Some(result),
                                Err(e) => {
                                    eprintln!("Error adjusting font size: {}", e);
                                    Some(registry::ZoomingResult::Failed)
                                }
                            }
                        } else {
                            if args.verbose {
                                eprintln!("No kitty currently focused, skipping font decrease");
                            }
                            None
                        };

                        let event = FocusEvent::FocusLost { zooming: zooming_result };
                        println!("{}", serde_json::to_string(&event).unwrap());
                    }
                },
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
