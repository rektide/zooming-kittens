use std::{
    fs,
    process::{Command, Output},
};

fn invoke(config: &str, environment_minimum: Option<&str>, args: &[&str]) -> Output {
    let directory = tempfile::tempdir().unwrap();
    let config_directory = directory.path().join("kitty-focus-tracker");
    fs::create_dir(&config_directory).unwrap();
    fs::write(config_directory.join("config.toml"), config).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_zk"));
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("ZK_") {
            command.env_remove(key);
        }
    }
    command.env("XDG_CONFIG_HOME", directory.path());
    if let Some(value) = environment_minimum {
        command.env("ZK_LONGCAT__MINIMUM_HEIGHT", value);
    }
    command
        .args(["longcat", "--backend", "umbriel"])
        .args(args)
        .output()
        .unwrap()
}

fn error(output: Output) -> String {
    assert!(!output.status.success());
    String::from_utf8(output.stderr).unwrap()
}

#[test]
fn file_environment_and_optional_cli_overrides_have_deliberate_precedence() {
    let config = "[longcat]\nminimum_height = -1\nactive_height_proportion = 3.0\nother_height_proportion = 2.0\n";
    assert!(error(invoke(config, None, &[])).contains("minimum-height must be positive"));
    // A valid environment override reaches the capability refusal, not a socket.
    assert!(error(invoke(config, Some("120"), &[])).contains("Umbriel mutation unsupported"));
    assert!(
        error(invoke("[longcat]\nminimum_height = 90\n", Some("-1"), &[]))
            .contains("minimum-height must be positive")
    );
    assert!(
        error(invoke(config, Some("-1"), &["--minimum-height", "121"]))
            .contains("Umbriel mutation unsupported")
    );
}

#[test]
fn invalid_configuration_is_not_silently_replaced_by_defaults() {
    let message = error(invoke("this is not toml", None, &[]));
    assert!(!message.contains("Umbriel mutation unsupported"));
    assert!(message.contains("InvalidInput"));
}

#[test]
fn unsupported_umbriel_mutation_is_concrete_and_performs_no_focus_workaround() {
    let message = error(invoke("", None, &["--socket", "/does/not/exist"]));
    assert!(message.contains("chosen ID"));
    assert!(message.contains("no focus workaround"));
    assert!(!message.contains("No such file"));
}
