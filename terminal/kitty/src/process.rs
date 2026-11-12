use std::fs;

pub fn find_kitty_master_pid(shell_pid: i32) -> Option<i32> {
    find_master(shell_pid, |pid| {
        fs::read_to_string(format!("/proc/{pid}/stat")).ok()
    })
}

fn find_master(shell_pid: i32, read_stat: impl Fn(i32) -> Option<String>) -> Option<i32> {
    let mut pid = shell_pid;
    let max_depth = 20;

    for _ in 0..max_depth {
        if pid <= 0 {
            break;
        }
        if let Some(stat) = read_stat(pid)
            && let Some((comm, parent)) = parse_stat(&stat)
        {
            if comm == "kitty" {
                return Some(pid);
            }
            if parent == pid {
                break;
            }
            pid = parent;
            continue;
        }
        break;
    }
    None
}

// comm may contain whitespace and ')'; field offsets begin AFTER the last ')'.
pub(crate) fn parse_stat(stat: &str) -> Option<(&str, i32)> {
    let (_, rest) = stat.split_once('(')?;
    let (comm, fields) = rest.rsplit_once(')')?;
    let parent = fields.split_whitespace().nth(1)?.parse().ok()?;
    Some((comm, parent))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stat_comm_is_not_split_on_spaces_or_parentheses() {
        assert_eq!(parse_stat("42 (kitty) S 10 0 0"), Some(("kitty", 10)));
        assert_eq!(parse_stat("42 (a shell) S 10 0 0"), Some(("a shell", 10)));
        assert_eq!(
            parse_stat("42 (a ) shell) S 10 0 0"),
            Some(("a ) shell", 10))
        );
        assert_eq!(parse_stat("42 (not-kitty) S 10"), Some(("not-kitty", 10)));
        assert_eq!(parse_stat("42 (kitty) S nope"), None);
        assert_eq!(parse_stat("not a stat"), None);
    }

    #[test]
    fn invalid_pids_never_discover_an_unrelated_terminal() {
        assert_eq!(find_kitty_master_pid(0), None);
        assert_eq!(find_kitty_master_pid(-1), None);
    }

    #[test]
    fn compositor_master_pid_is_used_directly_and_shell_resolution_stays_in_its_ancestry() {
        let read = |pid| match pid {
            100 => Some("100 (kitty) S 1".into()),
            200 => Some("200 (a shell) S 100".into()),
            300 => Some("300 (other) S 1".into()),
            _ => None,
        };
        assert_eq!(find_master(100, read), Some(100));
        assert_eq!(find_master(200, read), Some(100));
        assert_eq!(find_master(300, read), None);
        assert_eq!(find_master(400, read), None);
        assert_eq!(
            find_master(500, |pid| Some(format!("{pid} (cycle) S {pid}"))),
            None
        );
    }
}
