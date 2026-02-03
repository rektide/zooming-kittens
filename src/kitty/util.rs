use std::fs;
use std::path::PathBuf;

pub fn get_kitty_password() -> Result<String, std::io::Error> {
    let password_path = dirs::config_dir()
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Config directory not found")
        })?
        .join("kitty/rc.password");

    if !password_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Password file not found",
        ));
    }

    fs::read_to_string(password_path).map(|s| s.trim().to_string())
}

pub fn get_kitty_socket_path(pid: i32) -> PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());

    PathBuf::from(runtime_dir).join(format!("kitty-{}.sock", pid))
}

pub fn is_process_alive(pid: i32) -> bool {
    std::path::Path::new(&format!("/proc/{}", pid)).exists()
}
