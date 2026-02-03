use clap::Subcommand;
use kitty_rc::Kitty;
use kitty_rc::commands::SetFontSizeCommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum FontCommand {
    /// Increase font size
    #[command(name = "inc")]
    Inc {
        /// Kitty PID (optional, will auto-detect if not provided)
        #[arg(short = 'p', long)]
        pid: Option<i32>,

        /// Socket path (optional, auto-generated from PID if not provided)
        #[arg(short = 's', long)]
        socket: Option<String>,

        /// Password for encrypted connection (optional)
        #[arg(short = 'w', long)]
        password: Option<String>,

        /// Number of increments (default: 1)
        #[arg(short, long, default_value = "1")]
        count: u32,
    },

    /// Decrease font size
    #[command(name = "dec")]
    Dec {
        /// Kitty PID (optional, will auto-detect if not provided)
        #[arg(short = 'p', long)]
        pid: Option<i32>,

        /// Socket path (optional, auto-generated from PID if not provided)
        #[arg(short = 's', long)]
        socket: Option<String>,

        /// Password for encrypted connection (optional)
        #[arg(short = 'w', long)]
        password: Option<String>,

        /// Number of decrements (default: 1)
        #[arg(short, long, default_value = "1")]
        count: u32,
    },

    /// Set absolute font size
    #[command(name = "set")]
    Set {
        /// Kitty PID (optional, will auto-detect if not provided)
        #[arg(short = 'p', long)]
        pid: Option<i32>,

        /// Socket path (optional, auto-generated from PID if not provided)
        #[arg(short = 's', long)]
        socket: Option<String>,

        /// Password for encrypted connection (optional)
        #[arg(short = 'w', long)]
        password: Option<String>,

        /// Font size in points
        size: f64,

        /// Apply to all kitty instances
        #[arg(short, long)]
        all: bool,
    },

    /// Show current kitty instances
    #[command(name = "list")]
    List,
}

fn find_kitty_instances() -> Vec<(i32, PathBuf)> {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    let mut instances = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&runtime_dir) {
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                if name.starts_with("kitty-") && name.ends_with(".sock") {
                    let pid_str = name
                        .strip_prefix("kitty-")
                        .and_then(|s| s.strip_suffix(".sock"));
                    if let Some(pid_str) = pid_str {
                        if let Ok(pid) = pid_str.parse::<i32>() {
                            let socket_path = entry.path();
                            if std::path::Path::new(&format!("/proc/{}", pid)).exists() {
                                instances.push((pid, socket_path));
                            }
                        }
                    }
                }
            }
        }
    }

    instances.sort_by_key(|(pid, _)| *pid);
    instances
}

fn get_password() -> Option<String> {
    let config_dir = dirs::config_dir()?.join("kitty");
    let password_path = config_dir.join("rc.password");

    if password_path.exists() {
        std::fs::read_to_string(&password_path)
            .ok()
            .map(|s| s.trim().to_string())
    } else {
        None
    }
}

pub async fn handle_font_command(cmd: FontCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        FontCommand::List => {
            let instances = find_kitty_instances();
            if instances.is_empty() {
                println!("No kitty instances found");
            } else {
                println!("Kitty instances:");
                for (pid, socket) in &instances {
                    println!("  PID {}: {}", pid, socket.display());
                }
            }
            return Ok(());
        }

        FontCommand::Inc {
            pid,
            socket,
            password,
            count,
        } => {
            let pid = pid.unwrap_or_else(|| {
                let instances = find_kitty_instances();
                if instances.len() == 1 {
                    instances[0].0
                } else {
                    eprintln!("Multiple kitty instances found. Please specify --pid");
                    std::process::exit(1);
                }
            });

            let socket = socket.map(PathBuf::from).unwrap_or_else(|| {
                let runtime_dir =
                    std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
                PathBuf::from(runtime_dir).join(format!("kitty-{}.sock", pid))
            });

            let password = password.or_else(get_password);

            let mut kitty = if let Some(pw) = password.as_ref() {
                Kitty::builder()
                    .socket_path(&socket)
                    .password(pw.as_str())
                    .connect()
                    .await?
            } else {
                Kitty::builder().socket_path(&socket).connect().await?
            };

            for _ in 0..count {
                let cmd = SetFontSizeCommand::new(0).increment_op("+").build()?;
                let result = kitty.execute(&cmd).await?;
                if !result.ok {
                    eprintln!("Error: {:?}", result.error);
                    return Err("Failed to increase font size".into());
                }
            }

            println!("Font size increased {} times", count);
        }

        FontCommand::Dec {
            pid,
            socket,
            password,
            count,
        } => {
            let pid = pid.unwrap_or_else(|| {
                let instances = find_kitty_instances();
                if instances.len() == 1 {
                    instances[0].0
                } else {
                    eprintln!("Multiple kitty instances found. Please specify --pid");
                    std::process::exit(1);
                }
            });

            let socket = socket.map(PathBuf::from).unwrap_or_else(|| {
                let runtime_dir =
                    std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
                PathBuf::from(runtime_dir).join(format!("kitty-{}.sock", pid))
            });

            let password = password.or_else(get_password);

            let mut kitty = if let Some(pw) = password.as_ref() {
                Kitty::builder()
                    .socket_path(&socket)
                    .password(pw.as_str())
                    .connect()
                    .await?
            } else {
                Kitty::builder().socket_path(&socket).connect().await?
            };

            for _ in 0..count {
                let cmd = SetFontSizeCommand::new(0).increment_op("-").build()?;
                let result = kitty.execute(&cmd).await?;
                if !result.ok {
                    eprintln!("Error: {:?}", result.error);
                    return Err("Failed to decrease font size".into());
                }
            }

            println!("Font size decreased {} times", count);
        }

        FontCommand::Set {
            pid,
            socket,
            password,
            size,
            all,
        } => {
            if all {
                let instances = find_kitty_instances();
                if instances.is_empty() {
                    println!("No kitty instances found");
                    return Ok(());
                }

                let password = password.or_else(get_password);

                for (pid, socket) in &instances {
                    let mut kitty = if let Some(pw) = password.as_ref() {
                        match Kitty::builder()
                            .socket_path(socket)
                            .password(pw.as_str())
                            .connect()
                            .await
                        {
                            Ok(k) => k,
                            Err(_) => {
                                eprintln!("PID {}: Failed to connect", pid);
                                continue;
                            }
                        }
                    } else {
                        match Kitty::builder().socket_path(socket).connect().await {
                            Ok(k) => k,
                            Err(_) => {
                                eprintln!("PID {}: Failed to connect", pid);
                                continue;
                            }
                        }
                    };

                    let cmd = SetFontSizeCommand::new(size as i32).build()?;
                    let result = kitty.execute(&cmd).await?;
                    if result.ok {
                        println!("PID {}: Font size set to {}", pid, size);
                    } else {
                        eprintln!("PID {}: Error - {:?}", pid, result.error);
                    }
                }
            } else {
                let pid = pid.unwrap_or_else(|| {
                    let instances = find_kitty_instances();
                    if instances.len() == 1 {
                        instances[0].0
                    } else {
                        eprintln!("Multiple kitty instances found. Please specify --pid");
                        std::process::exit(1);
                    }
                });

                let socket = socket.map(PathBuf::from).unwrap_or_else(|| {
                    let runtime_dir =
                        std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
                    PathBuf::from(runtime_dir).join(format!("kitty-{}.sock", pid))
                });

                let password = password.or_else(get_password);

                let mut kitty = if let Some(pw) = password.as_ref() {
                    Kitty::builder()
                        .socket_path(&socket)
                        .password(pw.as_str())
                        .connect()
                        .await?
                } else {
                    Kitty::builder().socket_path(&socket).connect().await?
                };

                let cmd = SetFontSizeCommand::new(size as i32).build()?;
                let result = kitty.execute(&cmd).await?;
                if result.ok {
                    println!("Font size set to {}", size);
                } else {
                    eprintln!("Error: {:?}", result.error);
                    return Err("Failed to set font size".into());
                }
            }
        }
    }

    Ok(())
}
