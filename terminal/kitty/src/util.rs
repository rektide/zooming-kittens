use std::fs;
use std::os::unix::fs::MetadataExt;
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
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            fs::metadata("/proc/self")
                .ok()
                .map(|meta| PathBuf::from(format!("/run/user/{}", meta.uid())))
        });
    socket_path(pid, runtime_dir)
}

fn socket_path(pid: i32, runtime_dir: Option<PathBuf>) -> PathBuf {
    let paths: Vec<PathBuf> = match runtime_dir {
        Some(dir) => vec![dir.join(format!("kitty-{}.sock", pid))],
        None => {
            vec![PathBuf::from(format!("/tmp/kitty-{}.sock", pid))]
        }
    };

    for path in &paths {
        if path.exists() {
            return path.clone();
        }
    }

    paths[0].clone()
}

pub fn is_process_alive(pid: i32) -> bool {
    std::path::Path::new(&format!("/proc/{}", pid)).exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_discovery_never_substitutes_another_kitty_pid() {
        let runtime = tempfile::tempdir().unwrap();
        fs::write(runtime.path().join("kitty-222.sock"), []).unwrap();
        assert_eq!(
            socket_path(111, Some(runtime.path().into())),
            runtime.path().join("kitty-111.sock")
        );
        fs::write(runtime.path().join("kitty-111.sock"), []).unwrap();
        assert_eq!(
            socket_path(111, Some(runtime.path().into())),
            runtime.path().join("kitty-111.sock")
        );
    }
}
