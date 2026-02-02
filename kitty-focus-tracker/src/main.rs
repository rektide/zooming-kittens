use clap::Parser;
use kitty_rc::commands::SetFontSizeCommand;
use kitty_rc::KittyClient;
use niri_ipc::socket::Socket;
use niri_ipc::{Request, Response, Window};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize)]
#[serde(tag = "event")]
enum FocusEvent {
    #[serde(rename = "focus_gained")]
    FocusGained { window_id: u64, app_id: String },
    #[serde(rename = "focus_lost")]
    FocusLost,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "kitty")]
    app_id: String,

    #[arg(short, long)]
    verbose: bool,
}

struct KittyWindow {
    app_id: String,
    pid: Option<i32>,
}

fn is_kitty_window(app_id: &str, target_app_id: &str) -> bool {
    app_id == target_app_id
}

fn get_kitty_password() -> std::io::Result<String> {
    let password_path = dirs::config_dir()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Config directory not found"))?
        .join("kitty/rc.password");

    fs::read_to_string(password_path)
        .map(|s| s.trim().to_string())
}

fn get_kitty_socket_path(pid: i32) -> PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| "/tmp".to_string());

    PathBuf::from(runtime_dir)
        .join("kitty")
        .join(format!("kitty-{}.sock", pid))
}

async fn adjust_kitty_font_size(pid: i32, increase: bool) -> Result<(), Box<dyn std::error::Error>> {
    let password = get_kitty_password()?;
    let socket_path = get_kitty_socket_path(pid);

    let client = KittyClient::new(&socket_path, Some(&password)).await?;

    let increment_op = if increase { "+" } else { "-" };

    for _ in 0..3 {
        let cmd = SetFontSizeCommand::new(0)
            .increment_op(increment_op)
            .build()?;
        client.execute(&cmd).await?;
    }

    Ok(())
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
                                let event = FocusEvent::FocusGained {
                                    window_id: focused_id,
                                    app_id: window.app_id.clone(),
                                };
                                println!("{}", serde_json::to_string(&event).unwrap());

                                if let Some(pid) = window.pid {
                                    if args.verbose {
                                        eprintln!("Increasing font size for kitty PID {}", pid);
                                    }
                                    if let Err(e) = adjust_kitty_font_size(pid, true).await {
                                        eprintln!("Error adjusting font size: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    None => {
                        let event = FocusEvent::FocusLost;
                        println!("{}", serde_json::to_string(&event).unwrap());

                        if args.verbose {
                            eprintln!("Decreasing font size for all kitty windows");
                        }

                        for window in windows.values() {
                            if is_kitty_window(&window.app_id, &args.app_id) {
                                if let Some(pid) = window.pid {
                                    if args.verbose {
                                        eprintln!("Decreasing font size for kitty PID {}", pid);
                                    }
                                    if let Err(e) = adjust_kitty_font_size(pid, false).await {
                                        eprintln!("Error adjusting font size: {}", e);
                                    }
                                }
                            }
                        }
                    }
                },
                _ => {}
            },
            Err(e) => {
                eprintln!("Error reading event: {:?}", e);
                return Err(e);
            }
        }
    }
}
