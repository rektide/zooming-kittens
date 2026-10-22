use super::longcat::{Backend, LongcatCommand};
use crate::config::LongcatConfig;
use clap::{Args, Subcommand};
use std::{io, io::Write, path::Path};

#[derive(Debug, Args)]
pub struct GenerateSystemdCommand {
    /// Print the service file to stdout; never install or start it.
    #[arg(short, long, global = true)]
    pub output: bool,
    /// Omit for the existing font tracker service.
    #[command(subcommand)]
    pub mode: Option<ServiceMode>,
}

#[derive(Debug, Subcommand)]
pub enum ServiceMode {
    /// Generate a window-sizing service for a native compositor-managed user session.
    Longcat(LongcatCommand),
}

pub fn generate_systemd_service(command: GenerateSystemdCommand) -> io::Result<()> {
    let service_name =
        std::env::var("ZOOMING_APPNAME").unwrap_or_else(|_| "zooming-kittens".to_string());
    let unit = render_service(&command, &std::env::current_exe()?, &service_name)?;
    if command.output {
        io::stdout().write_all(unit.as_bytes())?;
    }
    Ok(())
}

fn path_text(path: &Path) -> io::Result<&str> {
    path.to_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "systemd service paths must be UTF-8",
        )
    })
}

/// systemd command-line quoting, not shell quoting. The ':' executable prefix
/// below disables environment expansion; '%%' prevents specifier expansion.
fn exec_argument(value: &str) -> String {
    let mut quoted = String::from("\"");
    for character in value.chars() {
        match character {
            '\\' => quoted.push_str("\\\\"),
            '"' => quoted.push_str("\\\""),
            '%' => quoted.push_str("%%"),
            character if character.is_ascii_control() => {
                quoted.push_str(&format!("\\x{:02x}", character as u32));
            }
            character => quoted.push(character),
        }
    }
    quoted.push('"');
    quoted
}

fn render_service(
    command: &GenerateSystemdCommand,
    executable: &Path,
    service_name: &str,
) -> io::Result<String> {
    if service_name.chars().any(char::is_control) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "ZOOMING_APPNAME must not contain control characters",
        ));
    }
    let name = service_name.replace('%', "%%");
    let executable = path_text(executable)?;
    // systemd rejects these in the executable path even when correctly quoted.
    // Arguments (including socket paths) have no corresponding restriction.
    if executable.chars().any(|character| {
        character.is_ascii_control() || matches!(character, '\\' | '"' | '\'' | '*' | '?' | '[')
    }) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "systemd cannot execute paths containing controls, quotes, backslashes or glob characters; move the zk binary to a compatible path",
        ));
    }
    let executable = exec_argument(executable);
    let Some(ServiceMode::Longcat(longcat)) = &command.mode else {
        // Preserve the original font invocation, dependencies and restart policy.
        return Ok(format!(
            "[Unit]\nDescription={name} Focus Tracker\nAfter=niri.target\nWants=niri.target\n\n\
             [Service]\nType=simple\nExecStart=:{executable}\nEnvironment=RUST_BACKTRACE=full\nRestart=always\n\n\
             [Install]\nWantedBy=default.target\n"
        ));
    };
    let (backend, compositor, session) = match longcat.backend {
        Backend::Niri => {
            crate::require_feature(cfg!(feature = "niri"), "niri")?;
            ("niri", "niri.service", "graphical-session.target")
        }
        Backend::Umbriel => {
            crate::require_feature(cfg!(feature = "umbriel"), "umbriel")?;
            ("umbriel", "umbriel.service", "umbriel-session.target")
        }
    };
    longcat.compose(LongcatConfig::default())?;
    let mut args = vec![
        "longcat".to_owned(),
        "--backend".to_owned(),
        backend.to_owned(),
    ];
    if let Some(socket) = &longcat.socket {
        args.extend(["--socket".to_owned(), path_text(socket)?.to_owned()]);
    }
    for (flag, value) in [
        ("--limit", longcat.limit.map(|value| value.to_string())),
        (
            "--active-height-proportion",
            longcat
                .active_height_proportion
                .map(|value| value.to_string()),
        ),
        (
            "--other-height-proportion",
            longcat
                .other_height_proportion
                .map(|value| value.to_string()),
        ),
        (
            "--minimum-height",
            longcat.minimum_height.map(|value| value.to_string()),
        ),
        (
            "--resize-debounce-ms",
            longcat.resize_debounce_ms.map(|value| value.to_string()),
        ),
        (
            "--convergence-timeout-ms",
            Some(longcat.convergence_timeout_ms.to_string()),
        ),
        (
            "--shutdown-timeout-ms",
            Some(longcat.shutdown_timeout_ms.to_string()),
        ),
    ] {
        if let Some(value) = value {
            args.extend([flag.to_owned(), value]);
        }
    }
    if longcat.dry_run {
        args.push("--dry-run".to_owned());
    }
    if longcat.quiet {
        args.push("--quiet".to_owned());
    }
    if longcat.verbose_count > 0 {
        args.push(format!("-{}", "v".repeat(longcat.verbose_count as usize)));
    }
    let args = args
        .iter()
        .map(|arg| exec_argument(arg))
        .collect::<Vec<_>>()
        .join(" ");
    // Runner restoration budget + source shutdown (3s) + output drain (2s),
    // with scheduling margin. SIGTERM would bypass the runner's Ctrl-C path.
    let stop_timeout_ms = longcat.shutdown_timeout_ms + 10_000;
    Ok(format!(
        "[Unit]\nDescription={name} Longcat Window Sizing ({backend})\n\
         Requisite={compositor}\nAfter={compositor}\nPartOf={compositor} {session}\n\n\
         [Service]\nType=simple\nExecStart=:{executable} {args}\n\
         Environment=RUST_BACKTRACE=full\nRestart=on-failure\nRestartSec=1s\n\
         KillSignal=SIGINT\nTimeoutStopSec={stop_timeout_ms}ms\n\n\
         [Install]\nWantedBy={session}\n"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_arguments_escape_unit_syntax_without_shell_quoting() {
        assert_eq!(
            exec_argument("/tmp/a b%\"\\$HOME;\n\t"),
            "\"/tmp/a b%%\\\"\\\\$HOME;\\x0a\\x09\""
        );
    }

    #[test]
    fn default_service_keeps_font_invocation_and_dependencies() {
        let command = GenerateSystemdCommand {
            output: true,
            mode: None,
        };
        let unit = render_service(&command, Path::new("/a path/zk%$x"), "example").unwrap();
        assert!(unit.contains("ExecStart=:\"/a path/zk%%$x\"\n"));
        assert!(unit.contains("After=niri.target\nWants=niri.target"));
        assert!(unit.contains("Restart=always"));
        assert!(unit.contains("WantedBy=default.target"));
        assert!(!unit.contains("longcat"));
        assert!(render_service(&command, Path::new("/zk"), "injected\nExecStart=bad").is_err());
        assert!(render_service(&command, Path::new("/zk\"bad"), "example").is_err());
    }
}
