use std::io::Write;

pub fn generate_systemd_service(output: bool) -> std::io::Result<()> {
    let service_name = std::env::var("ZOOMING_APPNAME")
        .ok()
        .unwrap_or_else(|| "zooming-kittens".to_string());
    let description = format!("{} Focus Tracker", service_name);
    let binary_path = std::env::current_exe()?;
    let binary_path = binary_path
        .to_str()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "kitty-focus-tracker"))?;

    if output {
        std::io::stdout().write_all(b"[Unit]\n")?;
        std::io::stdout().write_all(format!("Description={}\n", description).as_bytes())?;
        std::io::stdout().write_all(b"After=niri.target\n")?;
        std::io::stdout().write_all(b"Wants=niri.target\n")?;
        std::io::stdout().write_all(b"\n")?;
        std::io::stdout().write_all(b"[Service]\n")?;
        std::io::stdout().write_all(b"Type=simple\n")?;
        std::io::stdout().write_all(b"ExecStart=")?;
        std::io::stdout().write_all(binary_path.as_bytes())?;
        std::io::stdout().write_all(b"\n")?;
        std::io::stdout().write_all(b"Environment=RUST_BACKTRACE=full\n")?;
        std::io::stdout().write_all(b"Restart=always\n")?;
        std::io::stdout().write_all(b"\n")?;
        std::io::stdout().write_all(b"[Install]\n")?;
        std::io::stdout().write_all(b"WantedBy=default.target\n")?;
    }
    Ok(())
}
