//! Linux, target-bound kitty authentication discovery. No helper executable,
//! caller environment, PID-only TSV, process-wide scan or remote command.
//!
//! Native kitty `Child.get_final_env()` sets KITTY_PID and KITTY_PUBLIC_KEY
//! together. A validated direct child is therefore the local association; a
//! kitty master's own environment can instead describe its parent terminal.
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::{self, BufRead, BufReader, Read},
    os::{fd::AsRawFd, unix::fs::MetadataExt},
    path::{Path, PathBuf},
};

const MAX_THREADS: usize = 64;
const MAX_CHILDREN: usize = 128;
const MAX_STAT_BYTES: usize = 8192;
const MAX_CHILDREN_BYTES: usize = 16 * 1024;
const MAX_ENV_BYTES: usize = 256 * 1024;
const MAX_TOTAL_ENV_BYTES: usize = 1024 * 1024;

/// Safe diagnostics: no environment values, public keys, passwords or payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryError {
    InvalidPid,
    NotKitty,
    ProcessChanged,
    Unavailable {
        operation: &'static str,
        kind: io::ErrorKind,
    },
    LimitExceeded(&'static str),
    InvalidProcessData,
    MalformedEnvironment,
    MissingKey,
    MalformedKey,
    AmbiguousKeys,
}

impl std::fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPid => f.write_str("invalid target PID"),
            Self::NotKitty => f.write_str("target process is not kitty"),
            Self::ProcessChanged => {
                f.write_str("target or direct child exited, changed identity or was reparented")
            }
            Self::Unavailable { operation, kind } => {
                write!(f, "{operation} unavailable ({kind:?})")
            }
            Self::LimitExceeded(resource) => {
                write!(f, "bounded public-key discovery exceeded {resource}")
            }
            Self::InvalidProcessData => f.write_str("invalid target process metadata"),
            Self::MalformedEnvironment => {
                f.write_str("malformed or duplicate kitty authentication environment fields")
            }
            Self::MissingKey => {
                f.write_str("no target-bound KITTY_PUBLIC_KEY on verified direct children")
            }
            Self::MalformedKey => {
                f.write_str("target-bound public key is not a version-1 32-byte Base85 key")
            }
            Self::AmbiguousKeys => {
                f.write_str("verified direct children disagree on the target public key")
            }
        }
    }
}
impl std::error::Error for DiscoveryError {}

fn unavailable(operation: &'static str, error: io::Error) -> DiscoveryError {
    DiscoveryError::Unavailable {
        operation,
        kind: error.kind(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Stamp {
    start_time: u64,
    uid: u32,
    device: u64,
    inode: u64,
}

struct Process {
    root: PathBuf,
    pid: i32,
    directory: File,
    stamp: Stamp,
    parent: i32,
    kitty: bool,
}

impl Process {
    fn open(root: &Path, pid: i32) -> Result<Self, DiscoveryError> {
        if pid <= 0 {
            return Err(DiscoveryError::InvalidPid);
        }
        let directory = File::open(root.join(pid.to_string()))
            .map_err(|e| unavailable("process directory", e))?;
        let meta = directory
            .metadata()
            .map_err(|e| unavailable("process identity", e))?;
        let mut process = Self {
            root: root.into(),
            pid,
            directory,
            stamp: Stamp {
                start_time: 0,
                uid: meta.uid(),
                device: meta.dev(),
                inode: meta.ino(),
            },
            parent: 0,
            kitty: false,
        };
        let (parent, start_time, kitty) = process.stat()?;
        process.stamp.start_time = start_time;
        process.parent = parent;
        process.kitty = kitty;
        process.verify(None)?;
        Ok(process)
    }

    // An open proc directory pins this process instance, not a reusable PID
    // pathname. Check both the pinned directory and the current PID entry.
    fn path(&self) -> PathBuf {
        PathBuf::from(format!("/proc/self/fd/{}", self.directory.as_raw_fd()))
    }

    fn stat(&self) -> Result<(i32, u64, bool), DiscoveryError> {
        let data = read_bounded(self.path().join("stat"), MAX_STAT_BYTES, "process stat")?;
        let stat = std::str::from_utf8(&data).map_err(|_| DiscoveryError::InvalidProcessData)?;
        let (comm, parent) =
            crate::process::parse_stat(stat).ok_or(DiscoveryError::InvalidProcessData)?;
        let (pid, _) = stat
            .split_once('(')
            .ok_or(DiscoveryError::InvalidProcessData)?;
        if pid.trim().parse::<i32>().ok() != Some(self.pid) {
            return Err(DiscoveryError::ProcessChanged);
        }
        let (_, fields) = stat
            .rsplit_once(')')
            .ok_or(DiscoveryError::InvalidProcessData)?;
        // Linux /proc/PID/stat field 22; fields after comm begin with field 3.
        let start_time = fields
            .split_whitespace()
            .nth(19)
            .and_then(|v| v.parse().ok())
            .ok_or(DiscoveryError::InvalidProcessData)?;
        Ok((parent, start_time, comm == "kitty"))
    }

    fn verify(&self, parent: Option<i32>) -> Result<(), DiscoveryError> {
        let meta = fs::metadata(self.root.join(self.pid.to_string()))
            .map_err(|_| DiscoveryError::ProcessChanged)?;
        let (current_parent, start_time, kitty) =
            self.stat().map_err(|_| DiscoveryError::ProcessChanged)?;
        if self.stamp
            != (Stamp {
                start_time,
                uid: meta.uid(),
                device: meta.dev(),
                inode: meta.ino(),
            })
            || (self.kitty && !kitty)
            || parent.is_some_and(|p| p != current_parent)
        {
            return Err(DiscoveryError::ProcessChanged);
        }
        Ok(())
    }

    fn children(&self) -> Result<BTreeSet<i32>, DiscoveryError> {
        let tasks = fs::read_dir(self.path().join("task"))
            .map_err(|e| unavailable("target thread census", e))?;
        let mut children = BTreeSet::new();
        for (index, entry) in tasks.enumerate() {
            if index == MAX_THREADS {
                return Err(DiscoveryError::LimitExceeded("64 target threads"));
            }
            let entry = entry.map_err(|e| unavailable("target thread census", e))?;
            let data = read_bounded(
                entry.path().join("children"),
                MAX_CHILDREN_BYTES,
                "direct child census",
            )?;
            let data =
                std::str::from_utf8(&data).map_err(|_| DiscoveryError::InvalidProcessData)?;
            for pid in data.split_whitespace() {
                let pid = pid
                    .parse::<i32>()
                    .ok()
                    .filter(|p| *p > 0)
                    .ok_or(DiscoveryError::InvalidProcessData)?;
                children.insert(pid);
                if children.len() > MAX_CHILDREN {
                    return Err(DiscoveryError::LimitExceeded("128 direct children"));
                }
            }
        }
        Ok(children)
    }
}

fn read_bounded(
    path: PathBuf,
    limit: usize,
    operation: &'static str,
) -> Result<Vec<u8>, DiscoveryError> {
    let file = File::open(path).map_err(|e| unavailable(operation, e))?;
    let mut data = Vec::new();
    file.take((limit + 1) as u64)
        .read_to_end(&mut data)
        .map_err(|e| unavailable(operation, e))?;
    if data.len() > limit {
        return Err(DiscoveryError::LimitExceeded(operation));
    }
    Ok(data)
}

/// A public key bound to one still-live kitty instance. Debug deliberately
/// redacts the key. Keep this owner with cached connections and verify it before
/// every send; do not replace it silently when its PID is reused.
pub struct TargetPublicKey {
    process: Process,
    key: String,
}

impl std::fmt::Debug for TargetPublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TargetPublicKey")
            .field("pid", &self.process.pid)
            .finish_non_exhaustive()
    }
}

impl TargetPublicKey {
    pub fn discover(pid: i32) -> Result<Self, DiscoveryError> {
        Self::discover_with(Path::new("/proc"), pid, |_| {})
    }

    fn discover_with(
        root: &Path,
        pid: i32,
        mut after_environment: impl FnMut(i32),
    ) -> Result<Self, DiscoveryError> {
        let process = Process::open(root, pid)?;
        if !process.kitty {
            return Err(DiscoveryError::NotKitty);
        }
        let mut selected: Option<String> = None;
        let mut remaining = MAX_TOTAL_ENV_BYTES;
        for child_pid in process.children()? {
            let child = Process::open(root, child_pid)?;
            if child.parent != pid
                || child.stamp.uid != process.stamp.uid
                || child.stamp.start_time < process.stamp.start_time
            {
                continue;
            }
            let env = File::open(child.path().join("environ"))
                .map_err(|e| unavailable("direct child authentication fields", e))?;
            let (candidate, bytes) = read_key(env, pid, remaining.min(MAX_ENV_BYTES))?;
            remaining = remaining
                .checked_sub(bytes)
                .ok_or(DiscoveryError::LimitExceeded(
                    "1 MiB total environment bytes",
                ))?;
            after_environment(child_pid);
            child.verify(Some(pid))?;
            if let Some(key) = candidate {
                if selected.as_ref().is_some_and(|old| old != &key) {
                    return Err(DiscoveryError::AmbiguousKeys);
                }
                selected = Some(key);
            }
        }
        process.verify(None)?;
        Ok(Self {
            process,
            key: selected.ok_or(DiscoveryError::MissingKey)?,
        })
    }

    pub fn verify(&self) -> Result<(), DiscoveryError> {
        self.process.verify(None)
    }
    pub fn pid(&self) -> i32 {
        self.process.pid
    }
    pub fn public_key(&self) -> &str {
        &self.key
    }
}

// Stream only the two named authentication fields into retained values. Other
// environment entries (including passwords) are discarded, never diagnosed.
fn read_key(
    reader: impl Read,
    target: i32,
    limit: usize,
) -> Result<(Option<String>, usize), DiscoveryError> {
    let mut reader = BufReader::new(reader.take((limit + 1) as u64));
    let mut entry = Vec::new();
    let mut bytes = 0;
    let mut pid = None;
    let mut key = None;
    loop {
        entry.clear();
        let count = reader
            .read_until(0, &mut entry)
            .map_err(|e| unavailable("direct child authentication fields", e))?;
        if count == 0 {
            break;
        }
        bytes += count;
        if bytes > limit {
            return Err(DiscoveryError::LimitExceeded(
                "authentication environment byte budget",
            ));
        }
        if entry.last() != Some(&0) {
            return Err(DiscoveryError::MalformedEnvironment);
        }
        entry.pop();
        if let Some(value) = entry.strip_prefix(b"KITTY_PID=") {
            if pid.is_some() || value.len() > 10 {
                return Err(DiscoveryError::MalformedEnvironment);
            }
            pid = Some(
                std::str::from_utf8(value)
                    .ok()
                    .and_then(|v| v.parse::<i32>().ok())
                    .filter(|p| *p > 0)
                    .ok_or(DiscoveryError::MalformedEnvironment)?,
            );
        } else if let Some(value) = entry.strip_prefix(b"KITTY_PUBLIC_KEY=") {
            if key.is_some() || value.len() > 64 {
                return Err(DiscoveryError::MalformedEnvironment);
            }
            key = Some(
                std::str::from_utf8(value)
                    .map_err(|_| DiscoveryError::MalformedKey)?
                    .to_owned(),
            );
        }
    }
    if pid != Some(target) {
        return Ok((None, bytes));
    }
    let Some(key) = key else {
        return Ok((None, bytes));
    };
    if !valid_key(&key) {
        return Err(DiscoveryError::MalformedKey);
    }
    Ok((Some(key), bytes))
}

fn valid_key(key: &str) -> bool {
    // Native kitty uses Python/RFC-1924 Base85: 32 bytes are exactly eight
    // five-character words. Check each word fits u32 BEFORE kitty-rc's decoder,
    // whose arithmetic otherwise panics (debug) or wraps (release) on overflow.
    const ALPHABET: &[u8] =
        b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz!#$%&()*+-;<=>?@^_`{|}~";
    if key.len() != 42 || !key.starts_with("1:") {
        return false;
    }
    for word in key.as_bytes()[2..].as_chunks::<5>().0 {
        let mut decoded = 0u32;
        for byte in word {
            let Some(value) = ALPHABET.iter().position(|candidate| candidate == byte) else {
                return false;
            };
            let Some(next) = decoded
                .checked_mul(85)
                .and_then(|n| n.checked_add(value as u32))
            else {
                return false;
            };
            decoded = next;
        }
    }
    true
}

#[cfg(test)]
pub(crate) mod tests;
