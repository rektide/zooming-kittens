use std::process::{Command, Output};

fn invoke(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_zk"))
        .env_remove("ZOOMING_APPNAME")
        .args(args)
        .output()
        .unwrap()
}

fn text(output: Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn bare_generator_preserves_font_service_and_output_opt_in() {
    assert!(text(invoke(&["generate-systemd"])).is_empty());
    let unit = text(invoke(&["generate-systemd", "--output"]));
    assert!(unit.contains("After=niri.target\nWants=niri.target"));
    assert!(unit.contains("Restart=always\n"));
    assert!(unit.contains("WantedBy=default.target\n"));
    assert!(!unit.contains("\"longcat\""));
}

#[test]
fn longcat_services_select_native_session_and_preserve_sizing_arguments() {
    for (backend, enabled, session) in [
        ("niri", cfg!(feature = "niri"), "graphical-session.target"),
        (
            "umbriel",
            cfg!(feature = "umbriel"),
            "umbriel-session.target",
        ),
    ] {
        let output = invoke(&[
            "generate-systemd",
            "--output",
            "longcat",
            "--backend",
            backend,
            "--socket",
            "/tmp/socket with % and $HOME \"quote\" \\slash",
            "--active-height-proportion",
            "3",
            "--other-height-proportion",
            "2",
            "--minimum-height",
            "80",
            "--minimum-width",
            "82",
            "--resize-debounce-ms",
            "400",
            "--convergence-timeout-ms",
            "2000",
            "--shutdown-timeout-ms",
            "60000",
            "--dry-run",
            "--limit",
            "1",
            "--quiet",
            "-vv",
        ]);
        if !enabled {
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8(output.stderr)
                    .unwrap()
                    .contains(&format!("feature `{backend}` is disabled"))
            );
            continue;
        }
        let unit = text(output);
        assert!(unit.contains(&format!(
            "Requisite={backend}.service\nAfter={backend}.service\n"
        )));
        assert!(unit.contains(&format!("PartOf={backend}.service {session}\n")));
        assert!(unit.contains(&format!("WantedBy={session}\n")));
        assert!(unit.contains(&format!("\"longcat\" \"--backend\" \"{backend}\"")));
        assert!(
            unit.contains("\"--socket\" \"/tmp/socket with %% and $HOME \\\"quote\\\" \\\\slash\"")
        );
        for (flag, value) in [
            ("active-height-proportion", "3"),
            ("other-height-proportion", "2"),
            ("minimum-height", "80"),
            ("minimum-width", "82"),
            ("resize-debounce-ms", "400"),
            ("convergence-timeout-ms", "2000"),
            ("shutdown-timeout-ms", "60000"),
            ("limit", "1"),
        ] {
            assert!(
                unit.contains(&format!("\"--{flag}\" \"{value}\"")),
                "{unit}"
            );
        }
        assert!(unit.contains("\"--dry-run\" \"--quiet\" \"-vv\""));
        assert!(unit.contains("ExecStart=:\""));
        assert!(unit.contains("KillSignal=SIGINT\nTimeoutStopSec=70000ms\n"));
        assert!(unit.contains("Restart=on-failure\n"));
        assert!(!unit.contains("niri.target"));
        assert!(!unit.contains("zoomer"));
    }
}

#[test]
#[cfg(feature = "umbriel")]
fn explicit_conditional_route_survives_service_generation() {
    let unit = text(invoke(&[
        "generate-systemd",
        "--output",
        "longcat",
        "--backend",
        "umbriel",
        "--umbriel-control",
        "conditional",
    ]));
    assert!(unit.contains("\"--umbriel-control\" \"conditional\""));
}

#[test]
fn enabled_generator_rejects_invalid_sizing_without_reading_socket() {
    let backend = if cfg!(feature = "niri") {
        "niri"
    } else if cfg!(feature = "umbriel") {
        "umbriel"
    } else {
        return;
    };
    let output = invoke(&[
        "generate-systemd",
        "longcat",
        "--output",
        "--backend",
        backend,
        "--minimum-height",
        "0",
    ]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("minimum-height must be positive")
    );
}

#[test]
fn executable_paths_are_quoted_and_specifiers_are_literal() {
    let directory = tempfile::tempdir().unwrap();
    let executable = directory.path().join("zk space%$VAR");
    std::fs::copy(env!("CARGO_BIN_EXE_zk"), &executable).unwrap();
    let unit = text(
        Command::new(&executable)
            .env("ZOOMING_APPNAME", "test%name")
            .args(["generate-systemd", "--output"])
            .output()
            .unwrap(),
    );
    assert!(unit.contains("ExecStart=:\""));
    assert!(unit.contains("/zk space%%$VAR\"\n"), "{unit}");
    assert!(unit.contains("Description=test%%name Focus Tracker\n"));
}
