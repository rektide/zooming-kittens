use super::*;

pub(crate) const MASTER: i32 = 100;
pub(crate) const CHILD: i32 = 200;

pub(crate) fn key_a() -> String {
    format!("1:{}", "1".repeat(40))
}
fn key_b() -> String {
    format!("1:{}", "2".repeat(40))
}

pub(crate) struct Fixture {
    pub(crate) root: tempfile::TempDir,
}

impl Fixture {
    pub(crate) fn new() -> Self {
        let fixture = Self {
            root: tempfile::tempdir().unwrap(),
        };
        fixture.process(MASTER, "kitty", 1, 1000);
        fixture.process(CHILD, "a ) shell", MASTER, 1100);
        fixture.children(&[CHILD]);
        fixture.environment(
            CHILD,
            &format!(
                "UNRELATED_PASSWORD=never-display-this\0KITTY_PID={MASTER}\0KITTY_PUBLIC_KEY={}\0",
                key_a()
            ),
        );
        // A master's inherited key is not its own native child-launch key.
        fixture.environment(
            MASTER,
            &format!("KITTY_PID=999\0KITTY_PUBLIC_KEY={}\0", key_b()),
        );
        fixture
    }

    pub(crate) fn process(&self, pid: i32, comm: &str, parent: i32, start_time: u64) {
        let directory = self.root.path().join(pid.to_string());
        fs::create_dir_all(directory.join(format!("task/{pid}"))).unwrap();
        self.stat(pid, comm, parent, start_time);
        fs::write(directory.join(format!("task/{pid}/children")), "").unwrap();
        fs::write(directory.join("environ"), "").unwrap();
    }

    pub(crate) fn stat(&self, pid: i32, comm: &str, parent: i32, start_time: u64) {
        fs::write(
            self.root.path().join(format!("{pid}/stat")),
            format!(
                "{pid} ({comm}) S {parent} {}{start_time} 0 0",
                "0 ".repeat(17)
            ),
        )
        .unwrap();
    }

    fn children(&self, children: &[i32]) {
        let value = children
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ");
        fs::write(
            self.root
                .path()
                .join(format!("{MASTER}/task/{MASTER}/children")),
            value,
        )
        .unwrap();
    }

    fn environment(&self, pid: i32, value: &str) {
        fs::write(self.root.path().join(format!("{pid}/environ")), value).unwrap();
    }

    pub(crate) fn discover(&self) -> Result<TargetPublicKey, DiscoveryError> {
        TargetPublicKey::discover_with(self.root.path(), MASTER, |_| {})
    }
}

#[test]
fn uses_verified_direct_child_key_not_masters_inherited_key() {
    let fixture = Fixture::new();
    let target = fixture.discover().unwrap();
    assert_eq!(target.pid(), MASTER);
    assert!(target.public_key() == key_a());
    target.verify().unwrap();
    let diagnostic = format!("{target:?}");
    assert!(!diagnostic.contains(&key_a()));
    assert!(!diagnostic.contains("never-display-this"));
}

#[test]
fn missing_mismatched_and_malformed_key_fields_fail_closed() {
    let fixture = Fixture::new();
    for value in [
        String::new(),
        format!("KITTY_PID={MASTER}\0"),
        format!("KITTY_PID=999\0KITTY_PUBLIC_KEY={}\0", key_a()),
        format!("KITTY_PUBLIC_KEY={}\0", key_a()),
    ] {
        fixture.environment(CHILD, &value);
        assert_eq!(fixture.discover().unwrap_err(), DiscoveryError::MissingKey);
    }
    for key in [
        "".into(),
        "no-prefix".into(),
        format!("2:{}", "1".repeat(40)),
        format!("1:{}", "1".repeat(39)),
        format!("1:{}", "1".repeat(41)),
        format!("1:{}", "\"".repeat(40)),
        format!("1:{}", "~".repeat(40)),
        format!("1:{}", "é".repeat(20)),
    ] {
        fixture.environment(
            CHILD,
            &format!("KITTY_PID={MASTER}\0KITTY_PUBLIC_KEY={key}\0"),
        );
        let error = fixture.discover().unwrap_err();
        assert_eq!(error, DiscoveryError::MalformedKey);
        assert!(!error.to_string().contains("never-display-this"));
    }
    fixture.environment(
        CHILD,
        &format!(
            "KITTY_PID={MASTER}\0KITTY_PID={MASTER}\0KITTY_PUBLIC_KEY={}\0",
            key_a()
        ),
    );
    assert_eq!(
        fixture.discover().unwrap_err(),
        DiscoveryError::MalformedEnvironment
    );
    fixture.environment(
        CHILD,
        &format!(
            "KITTY_PID={MASTER}\0KITTY_PUBLIC_KEY={}\0KITTY_PUBLIC_KEY={}\0",
            key_a(),
            key_a()
        ),
    );
    assert_eq!(
        fixture.discover().unwrap_err(),
        DiscoveryError::MalformedEnvironment
    );
}

#[test]
fn direct_children_must_agree_and_descendants_are_not_scanned() {
    let fixture = Fixture::new();
    fixture.process(201, "another shell", MASTER, 1101);
    fixture.children(&[CHILD, 201]);
    fixture.environment(
        201,
        &format!("KITTY_PID={MASTER}\0KITTY_PUBLIC_KEY={}\0", key_b()),
    );
    assert_eq!(
        fixture.discover().unwrap_err(),
        DiscoveryError::AmbiguousKeys
    );
    fixture.environment(
        201,
        &format!("KITTY_PID={MASTER}\0KITTY_PUBLIC_KEY={}\0", key_a()),
    );
    fixture.discover().unwrap();
    fixture.environment(CHILD, "KITTY_PID=999\0");
    fixture.stat(201, "grandchild", CHILD, 1101);
    fs::remove_file(fixture.root.path().join("201/environ")).unwrap();
    assert_eq!(fixture.discover().unwrap_err(), DiscoveryError::MissingKey);
}

#[test]
fn direct_children_from_any_bounded_target_thread_are_considered() {
    let fixture = Fixture::new();
    fixture.children(&[]);
    let thread = fixture.root.path().join(format!("{MASTER}/task/101"));
    fs::create_dir(&thread).unwrap();
    fs::write(thread.join("children"), CHILD.to_string()).unwrap();
    fixture.discover().unwrap();
}

#[test]
fn child_exit_reparent_and_pid_reuse_during_discovery_are_not_credentials() {
    for race in 0..3 {
        let fixture = Fixture::new();
        let result =
            TargetPublicKey::discover_with(fixture.root.path(), MASTER, |pid| match race {
                0 => {
                    fs::remove_file(fixture.root.path().join(format!("{pid}/stat"))).unwrap();
                }
                1 => fixture.stat(pid, "shell", 999, 1100),
                _ => {
                    fs::rename(
                        fixture.root.path().join(pid.to_string()),
                        fixture.root.path().join("retired-child"),
                    )
                    .unwrap();
                    fixture.process(pid, "new shell", MASTER, 1200);
                }
            });
        assert_eq!(result.unwrap_err(), DiscoveryError::ProcessChanged);
    }
}

#[test]
fn master_reuse_during_or_after_discovery_revokes_the_binding() {
    let fixture = Fixture::new();
    let result = TargetPublicKey::discover_with(fixture.root.path(), MASTER, |_| {
        fixture.stat(MASTER, "kitty", 1, 2000)
    });
    assert_eq!(result.unwrap_err(), DiscoveryError::ProcessChanged);
    fixture.stat(MASTER, "kitty", 1, 1000);
    let target = fixture.discover().unwrap();
    fs::rename(
        fixture.root.path().join(MASTER.to_string()),
        fixture.root.path().join("retired-master"),
    )
    .unwrap();
    fixture.process(MASTER, "kitty", 1, 2000);
    assert_eq!(target.verify().unwrap_err(), DiscoveryError::ProcessChanged);
}

#[test]
fn discovery_is_bounded_and_does_not_accept_unreadable_or_non_kitty_targets() {
    let fixture = Fixture::new();
    fixture.stat(MASTER, "not kitty", 1, 1000);
    assert_eq!(fixture.discover().unwrap_err(), DiscoveryError::NotKitty);
    fixture.stat(MASTER, "kitty", 1, 1000);
    fixture.children(&(1..=129).collect::<Vec<_>>());
    assert!(matches!(
        fixture.discover(),
        Err(DiscoveryError::LimitExceeded(_))
    ));
    fixture.children(&[CHILD]);
    let data = format!(
        "UNRELATED={}\0KITTY_PID={MASTER}\0KITTY_PUBLIC_KEY={}\0",
        "x".repeat(MAX_ENV_BYTES),
        key_a()
    );
    fixture.environment(CHILD, &data);
    assert!(matches!(
        fixture.discover(),
        Err(DiscoveryError::LimitExceeded(_))
    ));
    fs::remove_file(fixture.root.path().join(format!("{CHILD}/environ"))).unwrap();
    assert!(matches!(
        fixture.discover(),
        Err(DiscoveryError::Unavailable {
            kind: io::ErrorKind::NotFound,
            ..
        })
    ));
    assert_eq!(
        TargetPublicKey::discover(0).unwrap_err(),
        DiscoveryError::InvalidPid
    );
}

#[test]
fn selected_environment_values_are_strict_but_unrelated_binary_values_are_ignored() {
    let mut environment = b"IRRELEVANT=\xff\0".to_vec();
    environment.extend(format!("KITTY_PID={MASTER}\0KITTY_PUBLIC_KEY={}\0", key_a()).as_bytes());
    assert!(
        read_key(&environment[..], MASTER, MAX_ENV_BYTES)
            .unwrap()
            .0
            .is_some()
    );
    assert_eq!(
        read_key(&b"KITTY_PID=not-a-pid\0"[..], MASTER, MAX_ENV_BYTES).unwrap_err(),
        DiscoveryError::MalformedEnvironment
    );
    assert_eq!(
        read_key(
            &b"KITTY_PID=100\0KITTY_PUBLIC_KEY=\xff\0"[..],
            MASTER,
            MAX_ENV_BYTES
        )
        .unwrap_err(),
        DiscoveryError::MalformedKey
    );
    assert_eq!(
        read_key(&b"KITTY_PID=100"[..], MASTER, MAX_ENV_BYTES).unwrap_err(),
        DiscoveryError::MalformedEnvironment
    );
}

#[test]
fn thread_and_total_environment_budgets_are_enforced() {
    let fixture = Fixture::new();
    for thread in 1..=MAX_THREADS {
        let directory = fixture.root.path().join(format!("{MASTER}/task/{thread}"));
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("children"), "").unwrap();
    }
    assert!(matches!(
        fixture.discover(),
        Err(DiscoveryError::LimitExceeded(_))
    ));
    let fixture = Fixture::new();
    for pid in 201..=205 {
        fixture.process(pid, "shell", MASTER, 1100);
        fixture.environment(
            pid,
            &format!(
                "UNRELATED={}\0KITTY_PID={MASTER}\0KITTY_PUBLIC_KEY={}\0",
                "x".repeat(210 * 1024),
                key_a()
            ),
        );
    }
    fixture.children(&(201..=205).collect::<Vec<_>>());
    assert!(matches!(
        fixture.discover(),
        Err(DiscoveryError::LimitExceeded(_))
    ));
}
