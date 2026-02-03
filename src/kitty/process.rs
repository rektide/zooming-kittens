use std::fs;

pub fn find_kitty_master_pid(shell_pid: i32) -> Option<i32> {
    let mut pid = shell_pid;
    let max_depth = 20;

    for _ in 0..max_depth {
        let proc_path = format!("/proc/{}/stat", pid);
        if let Ok(stat) = fs::read_to_string(&proc_path) {
            let parts: Vec<&str> = stat.split_whitespace().collect();
            if parts.len() > 1 {
                let comm = parts[1];
                let comm_clean = comm.trim_start_matches('(').trim_end_matches(')');
                if comm_clean == "kitty" {
                    return Some(pid);
                }
                if let Some(ppid_str) = parts.get(3) {
                    if let Ok(ppid) = ppid_str.parse::<i32>() {
                        pid = ppid;
                        continue;
                    }
                }
            }
        }
        break;
    }
    None
}
