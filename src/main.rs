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
        eprintln!("Starting polling for window focus changes...");
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
    let mut last_focused_window: Option<(u64, String, Option<i32>)> = None;
    
    if args.verbose {
        eprintln!("Tracking app_id: {}", args.app_id);
    }
    
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        
        let mut socket = Socket::connect()?;
        let reply = socket.send(Request::Windows)?;
        
        let focused_window = match reply {
            Ok(Response::Windows(windows)) => {
                windows.iter().find(|w| w.is_focused).map(|w| {
                    (w.id, w.app_id.clone().unwrap_or_default(), w.pid, w.is_focused)
                })
            }
            _ => None,
        };
        
        if let Some((window_id, app_id, pid, _)) = focused_window {
            let window_info = (window_id, app_id.clone(), pid);
            
            let focus_changed = match &last_focused_window {
                Some((id, _, _)) => id != &window_id,
                None => true,
            };
            
            if focus_changed {
                if args.verbose {
                    eprintln!(
                        "Focus changed to window {} (app_id: {}, pid: {:?})",
                        window_id, app_id, pid
                    );
                }
                
                if is_kitty_window(&app_id, &args.app_id) {
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
                    
                    focus_tracker.on_focus_gained(pid.unwrap_or(0));
                    
                    let zooming_result = if let Some(p) = pid {
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
                        window_id,
                        app_id: app_id.clone(),
                        zooming: zooming_result,
                    };
                    println!("{}", serde_json::to_string(&event).unwrap());
                } else if last_focused_window.is_some() {
                    if let Some(prev_pid) = focus_tracker.on_focus_lost() {
                        if args.verbose {
                            eprintln!("Focus moved away from kitty (PID {})", prev_pid);
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
                }
                
                last_focused_window = Some(window_info);
            }
        }
    }
}
