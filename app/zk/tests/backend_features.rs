use std::process::{Command, Output};

fn invoke(args: &[&str]) -> Output {
    let directory = tempfile::tempdir().unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_zk"));
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("ZK_") {
            command.env_remove(key);
        }
    }
    command
        .env("XDG_CONFIG_HOME", directory.path())
        .env_remove("NIRI_SOCKET")
        .args(args)
        .output()
        .unwrap()
}

fn disabled(args: &[&str], feature: &str) {
    let output = invoke(args);
    assert!(!output.status.success(), "{args:?} unexpectedly succeeded");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains(&format!("feature `{feature}` is disabled")),
        "{stderr}"
    );
    assert!(
        stderr.contains(&format!("--features {feature}")),
        "{stderr}"
    );
    assert!(!stderr.contains("No such file"), "{stderr}");
}

#[test]
fn disabled_commands_explain_the_build_requirement_before_io() {
    if !cfg!(feature = "kitty") {
        for args in [
            vec!["conf-size", "--config", "/does/not/exist"],
            vec!["cleanup"],
            vec!["font", "set", "18"],
        ] {
            disabled(&args, "kitty");
        }
    }
    if !cfg!(feature = "niri") {
        disabled(&["export", "--stream"], "niri");
    }
    if !cfg!(all(feature = "niri", feature = "kitty")) {
        let feature = if cfg!(feature = "niri") {
            "kitty"
        } else {
            "niri"
        };
        disabled(&[], feature);
        disabled(&["zoomer"], feature);
    }
}

#[test]
fn disabled_backends_are_rejected_before_socket_discovery_or_mutation_checks() {
    for (backend, enabled) in [
        ("niri", cfg!(feature = "niri")),
        ("umbriel", cfg!(feature = "umbriel")),
    ] {
        if !enabled {
            disabled(
                &[
                    "longcat",
                    "--backend",
                    backend,
                    "--socket",
                    "/does/not/exist",
                ],
                backend,
            );
            disabled(&["longcat", "--backend", backend, "--dry-run"], backend);
        }
    }
}

#[test]
fn basic_cli_is_available_in_every_build() {
    for args in [
        vec!["--help"],
        vec!["--version"],
        vec!["longcat", "--help"],
        vec!["generate-systemd", "--output"],
    ] {
        let output = invoke(&args);
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[cfg(feature = "niri")]
#[test]
fn niri_export_is_available_without_requiring_kitty() {
    let output = invoke(&["export", "--help"]);
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("--stream")
    );
}

#[cfg(feature = "kitty")]
#[test]
fn kitty_commands_are_available_without_requiring_niri() {
    for args in [
        vec!["font", "--help"],
        vec!["conf-size", "--help"],
        vec!["cleanup"],
    ] {
        let output = invoke(&args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
