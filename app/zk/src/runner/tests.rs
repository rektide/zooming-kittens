use super::*;
use crate::commands::longcat::{Backend, LongcatCommand};
use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[command(flatten)]
    command: LongcatCommand,
}

fn command(args: &[&str]) -> LongcatCommand {
    Cli::try_parse_from(std::iter::once("longcat").chain(args.iter().copied()))
        .unwrap()
        .command
}

#[test]
fn cli_defaults_are_optional_overrides_and_weights_normalize() {
    let command = command(&[]);
    assert_eq!(command.backend, Backend::Niri);
    assert!(!command.dry_run);
    assert_eq!(command.minimum_height, None);
    let configured = LongcatConfig {
        active_height_proportion: 3.0,
        other_height_proportion: 2.0,
        minimum_height: 91,
        minimum_width: 81,
        resize_debounce_ms: 777,
    };
    let config = command.compose(configured).unwrap();
    assert_eq!(config.minimum_height, 91);
    assert_eq!(config.resize_debounce_ms, 777);
    assert_eq!(
        config.active_height_proportion
            / (config.active_height_proportion + config.other_height_proportion),
        0.6
    );
}

#[test]
fn cli_rejects_bad_limits_timeouts_and_policy() {
    for args in [
        ["--limit", "0"],
        ["--convergence-timeout-ms", "0"],
        ["--shutdown-timeout-ms", "60001"],
    ] {
        assert!(Cli::try_parse_from(["longcat", args[0], args[1]]).is_err());
    }
    for args in [
        ["--minimum-height", "0"],
        ["--active-height-proportion", "NaN"],
        ["--other-height-proportion", "0"],
    ] {
        assert!(command(&args).compose(LongcatConfig::default()).is_err());
    }
}

use niri_ipc::{Event, Window, WindowLayout, Workspace};
use zk_desktop::{Epoch, NativeId, Publisher, SourceId};
use zk_longcat::Phase;

fn window(id: u64, workspace: u64, row: usize, height: i32, focused: bool) -> Window {
    Window {
        id,
        title: None,
        app_id: Some("test".into()),
        pid: None,
        workspace_id: Some(workspace),
        is_focused: focused,
        is_floating: false,
        is_urgent: false,
        focus_timestamp: None,
        layout: WindowLayout {
            pos_in_scrolling_layout: Some((1, row + 1)),
            tile_size: (800.0, f64::from(height) + 2.0),
            window_size: (798, height),
            tile_pos_in_workspace_view: Some((0.0, row as f64 * 402.0)),
            window_offset_in_tile: (1.0, 1.0),
        },
    }
}

fn workspace(id: u64) -> Workspace {
    Workspace {
        id,
        idx: id as u8,
        name: None,
        output: Some("test".into()),
        is_active: true,
        is_focused: id == 1,
        is_urgent: false,
        active_window_id: None,
    }
}

fn ready() -> zk_niri::Reducer {
    let mut reducer = zk_niri::Reducer::new(
        SourceId::new(zk_desktop::Backend::Niri, "replay"),
        Epoch(1),
        zk_niri::Limits::default(),
    );
    reducer
        .apply(Event::WindowsChanged {
            windows: vec![
                window(1, 1, 0, 400, true),
                window(2, 1, 1, 300, false),
                window(3, 1, 2, 200, false),
            ],
        })
        .unwrap();
    reducer
        .apply(Event::WorkspacesChanged {
            workspaces: vec![workspace(1), workspace(2)],
        })
        .unwrap();
    reducer
}

// Execute this helper in a subprocess: its stdout pipe is intentionally never
// drained, so a Tokio blocking-pool regression cannot hang the test runner.
#[test]
fn stdout_pipe_child() {
    let Ok(mode) = std::env::var("ZK_TEST_STDOUT_PIPE") else {
        return;
    };
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let (_publisher, observer) = Publisher::new(ready().capture(), 16);
        let mut rows = Vec::new();
        drive(
            observer,
            None,
            &command(&["--dry-run", "--limit", "1"]),
            &LongcatConfig::default(),
            &mut |mut record: Value| {
                // One synthetic diagnostic is larger than a Linux pipe. No
                // compositor connection or native mutation occurs in this test.
                record["synthetic_padding"] = Value::String("x".repeat(1024 * 1024));
                let mut row = serde_json::to_vec(&record)?;
                row.push(b'\n');
                rows.push(row);
                Ok(())
            },
            std::future::pending().boxed(),
        )
        .await
        .unwrap();
        if mode == "tokio" {
            use tokio::io::AsyncWriteExt;
            let mut stdout = tokio::io::stdout();
            assert!(
                tokio::time::timeout(Duration::from_millis(100), async {
                    for row in &rows {
                        stdout.write_all(row).await?;
                    }
                    stdout.flush().await
                })
                .await
                .is_err()
            );
            eprintln!("tokio-write-timed-out");
            // Dropping the runtime below still waits for the blocked write.
        } else {
            let output = output::Output::stdout().unwrap();
            for row in rows {
                output.rows.try_send(row).unwrap();
            }
            assert_eq!(
                output.finish().await.unwrap_err().kind(),
                io::ErrorKind::TimedOut
            );
            eprintln!("detached-writer-drain-timed-out");
        }
    });
    drop(runtime);
    // Avoid the test harness writing its own result line to the full pipe.
    // The runtime has already shut down; process exit performs std cleanup.
    std::process::exit(0);
}

#[test]
fn unread_stdout_pipe_reproduces_tokio_hang_but_detached_writer_exits() {
    use std::process::{Command, Stdio};
    for (mode, bound, should_exit, marker) in [
        (
            "tokio",
            Duration::from_secs(1),
            false,
            "tokio-write-timed-out",
        ),
        (
            "detached",
            Duration::from_secs(5),
            true,
            "detached-writer-drain-timed-out",
        ),
    ] {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "runner::tests::stdout_pipe_child", "--nocapture"])
            .env("ZK_TEST_STDOUT_PIPE", mode)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + bound;
        let exited = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break Some(status);
            }
            if std::time::Instant::now() >= deadline {
                break None;
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        if exited.is_none() {
            child.kill().unwrap();
        }
        let result = child.wait_with_output().unwrap();
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(marker),
            "{mode}: stderr={}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(exited.is_some(), should_exit, "{mode}");
        if should_exit {
            assert!(result.status.success(), "{}", result.status);
        }
    }
}

#[test]
fn bridge_keeps_baseline_budget_during_partial_native_application() {
    let mut reducer = ready();
    let mut control = Control::new(&LongcatConfig::default(), false, 100);
    let initial = reducer.capture();
    let key = initial.groups.keys().next().unwrap().clone();
    let (_, selected) = control.evaluate(&initial, 0, None, "initial").unwrap();
    let sequence = selected.unwrap();
    assert_eq!(
        sequence
            .plan
            .targets
            .iter()
            .map(|t| t.extent)
            .collect::<Vec<_>>(),
        vec![630, 162, 108]
    );
    control.record(&sequence, CommandOutcome::Handled).unwrap();
    reducer
        .apply(Event::WindowOpenedOrChanged {
            window: window(1, 1, 0, 630, true),
        })
        .unwrap();
    let partial = reducer.capture();
    let (observation, _) = bridge::observation(&partial, &key, 60, Some(900.0)).unwrap();
    assert_eq!(observation.available_extent, 900.0);
    assert_eq!(
        observation.members.iter().map(|m| m.extent).sum::<f64>(),
        1130.0
    );
    let mut next = sequence.clone();
    next.next = 1;
    let request = control.request(&partial, &next).unwrap();
    let native::Request::Height(request) = request else {
        panic!("expected sequential height request")
    };
    assert_eq!(request.basis, partial.version);
    assert_eq!(request.target.native, NativeId::Number(2));
    assert_eq!(request.height, 162);
    control
        .evaluate(&partial, 1, Some(&next), "partial")
        .unwrap();
    assert_eq!(control.slots[&key].episode.phase(), Phase::Applying);
    assert_eq!(
        control.slots[&key]
            .episode
            .baseline()
            .observation
            .available_extent,
        900.0
    );
}

#[test]
fn fresh_focus_and_geometry_validation_stop_stale_plan_before_send() {
    let mut reducer = ready();
    let mut control = Control::new(&LongcatConfig::default(), false, 100);
    let (_, selected) = control
        .evaluate(&reducer.capture(), 0, None, "initial")
        .unwrap();
    let sequence = selected.unwrap();
    reducer
        .apply(Event::WindowFocusChanged { id: None })
        .unwrap();
    assert!(
        control
            .request(&reducer.capture(), &sequence)
            .unwrap_err()
            .contains("FocusChanged")
    );
    control.finish(&sequence).unwrap();
    let (record, selected) = control
        .evaluate(&reducer.capture(), 1, None, "none")
        .unwrap();
    assert!(selected.is_none());
    assert_eq!(record["groups"][0]["observed"]["focus"], "None");
    assert!(control.slots.values().all(|slot| {
        slot.episode
            .effects()
            .iter()
            .all(|effect| effect.commands.iter().all(|c| *c == CommandOutcome::Unsent))
    }));
}

#[test]
fn move_invalidates_original_group_and_same_ordinal_is_not_same_workspace() {
    let mut reducer = ready();
    let mut control = Control::new(&LongcatConfig::default(), false, 100);
    let (_, selected) = control
        .evaluate(&reducer.capture(), 0, None, "initial")
        .unwrap();
    let sequence = selected.unwrap();
    control.record(&sequence, CommandOutcome::Handled).unwrap();
    reducer
        .apply(Event::WindowsChanged {
            windows: vec![
                window(1, 2, 0, 630, true),
                window(2, 1, 1, 300, false),
                window(3, 1, 2, 200, false),
                window(4, 2, 1, 400, false),
            ],
        })
        .unwrap();
    assert!(control.request(&reducer.capture(), &sequence).is_err());
    control.finish(&sequence).unwrap();
    let (record, selected) = control
        .evaluate(&reducer.capture(), 1, None, "move")
        .unwrap();
    assert!(selected.is_none());
    assert!(record["unsupported"].as_str().unwrap().contains("overlap"));
    assert_eq!(control.slots.len(), 1);
    assert_eq!(control.slots[&sequence.key].episode.phase(), Phase::Lost);
    assert_eq!(sequence.key.workspace.native, NativeId::Number(1));
}

#[test]
fn independent_context_change_revokes_authority_without_recomputing_budget() {
    let reducer = ready();
    let mut control = Control::new(&LongcatConfig::default(), false, 100);
    let (_, selected) = control
        .evaluate(&reducer.capture(), 0, None, "initial")
        .unwrap();
    let sequence = selected.unwrap();
    let mut changed = (*reducer.capture()).clone();
    for window in changed.windows.values_mut() {
        window.geometry.content_size.as_mut().unwrap().width += 100.0;
    }
    assert!(control.request(&changed, &sequence).is_err());
    control
        .evaluate(&changed, 1, Some(&sequence), "width-change")
        .unwrap();
    assert_eq!(control.slots[&sequence.key].episode.phase(), Phase::Lost);
}

fn stop_on_conflict(sender: &mut Option<tokio::sync::oneshot::Sender<()>>, value: &Value) {
    if value["groups"]
        .as_array()
        .is_some_and(|groups| groups.iter().any(|group| group["phase"] == "Conflicted"))
        && let Some(sender) = sender.take()
    {
        let _ = sender.send(());
    }
}

#[tokio::test(start_paused = true)]
async fn runner_timer_fires_after_last_event_without_polling_or_retries() {
    let reducer = ready();
    let (_publisher, observer) = Publisher::new(reducer.capture(), 16);
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counted = calls.clone();
    let executor: Executor = Arc::new(move |_| {
        counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        async { Transmission::Handled }.boxed()
    });
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let mut sender = Some(sender);
    let mut records = Vec::new();
    let mut sink = |value: Value| {
        stop_on_conflict(&mut sender, &value);
        records.push(value);
        Ok(())
    };
    let mut command = command(&[]);
    command.convergence_timeout_ms = 50;
    drive(
        observer,
        Some(executor),
        &command,
        &LongcatConfig::default(),
        &mut sink,
        async move { receiver.await.map_err(io::Error::other) }.boxed(),
    )
    .await
    .unwrap();
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 3);
    assert!(
        records
            .iter()
            .any(|record| record["reason"] == "convergence-timer"
                && record["groups"][0]["phase"] == "Conflicted")
    );
    assert!(
        records
            .iter()
            .any(|record| record["groups"][0]["desired"]["Suspend"] == "ConvergenceTimedOut")
    );
}

#[tokio::test(start_paused = true)]
async fn observations_continue_during_command_and_focus_change_stops_unsent_tail() {
    let mut reducer = ready();
    let initial = reducer.capture();
    let (publisher, observer) = Publisher::new(initial.clone(), 16);
    let publisher = Arc::new(publisher);
    let update = reducer
        .apply(Event::WindowFocusChanged { id: None })
        .unwrap();
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counted = calls.clone();
    let executor: Executor = Arc::new(move |_| {
        let publisher = publisher.clone();
        let update = update.clone();
        counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        async move {
            publisher.publish(update);
            tokio::time::sleep(Duration::from_millis(10)).await;
            Transmission::Handled
        }
        .boxed()
    });
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let mut sender = Some(sender);
    let mut records = Vec::new();
    let mut sink = |value: Value| {
        stop_on_conflict(&mut sender, &value);
        records.push(value);
        Ok(())
    };
    let mut command = command(&[]);
    command.convergence_timeout_ms = 50;
    drive(
        observer,
        Some(executor),
        &command,
        &LongcatConfig::default(),
        &mut sink,
        async move { receiver.await.map_err(io::Error::other) }.boxed(),
    )
    .await
    .unwrap();
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(records.iter().any(|r| r["reason"] == "capture"
        && r["groups"][0]["observed"]["focus"] == "None"
        && r["groups"][0]["requested_effect"]["execution_finished"] == false));
    let final_record = records.last().unwrap();
    assert_eq!(
        final_record["episodes"][0]["requested_effect"]["commands"],
        serde_json::json!(["Handled", "Unsent", "Unsent"])
    );
}

#[tokio::test]
async fn dry_run_never_submits_or_records_handled_effects() {
    let reducer = ready();
    let (_publisher, observer) = Publisher::new(reducer.capture(), 16);
    let executor: Executor = Arc::new(|_| panic!("dry-run must not call executor"));
    let mut records = Vec::new();
    let mut sink = |value| {
        records.push(value);
        Ok(())
    };
    drive(
        observer,
        Some(executor),
        &command(&["--dry-run", "--limit", "1"]),
        &LongcatConfig::default(),
        &mut sink,
        std::future::pending().boxed(),
    )
    .await
    .unwrap();
    assert!(records[0]["groups"][0]["desired"]["Plan"].is_object());
    assert!(records[0]["groups"][0]["requested_effect"].is_null());
    assert_eq!(records[0]["groups"][0]["retained_effect_count"], 0);
    assert_eq!(
        records.last().unwrap()["episodes"][0]["retained_effect_count"],
        0
    );
}

#[tokio::test(start_paused = true)]
async fn source_loss_during_pending_command_retains_uncertainty_and_unsent_tail() {
    let mut reducer = ready();
    let initial = reducer.capture();
    let (publisher, observer) = Publisher::new(initial, 16);
    let publisher = Arc::new(publisher);
    let update = reducer.loss(zk_desktop::LossReason::Disconnected);
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counted = calls.clone();
    let executor: Executor = Arc::new(move |_| {
        let publisher = publisher.clone();
        let update = update.clone();
        counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        async move {
            publisher.publish(update);
            std::future::pending().await
        }
        .boxed()
    });
    let mut records = Vec::new();
    let mut sink = |value| {
        records.push(value);
        Ok(())
    };
    let mut command = command(&[]);
    command.shutdown_timeout_ms = 20;
    let result = drive(
        observer,
        Some(executor),
        &command,
        &LongcatConfig::default(),
        &mut sink,
        std::future::pending().boxed(),
    )
    .await;
    assert!(result.is_err());
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    let final_record = records.last().unwrap();
    assert_eq!(final_record["type"], "longcat-stopped");
    assert_eq!(final_record["episodes"][0]["phase"], "Lost");
    assert!(
        final_record["episodes"][0]["requested_effect"]["commands"][0]["PossiblySent"].is_object()
    );
    assert_eq!(
        final_record["episodes"][0]["requested_effect"]["commands"][1],
        "Unsent"
    );
    assert_eq!(
        final_record["episodes"][0]["requested_effect"]["execution_finished"],
        true
    );
}

#[test]
fn repeated_dry_run_focus_previews_retain_one_immutable_baseline_and_no_requests() {
    let mut reducer = ready();
    let mut control = Control::new(&LongcatConfig::default(), true, 100);
    let (first, _) = control
        .evaluate(&reducer.capture(), 0, None, "first")
        .unwrap();
    reducer
        .apply(Event::WindowFocusChanged { id: Some(2) })
        .unwrap();
    let (second, selected) = control
        .evaluate(&reducer.capture(), 1, None, "second")
        .unwrap();
    assert!(selected.is_none());
    assert_eq!(
        first["groups"][0]["baseline"],
        second["groups"][0]["baseline"]
    );
    assert!(second["groups"][0]["desired"]["Plan"].is_object());
    assert_eq!(second["groups"][0]["retained_effect_count"], 0);
    assert_ne!(
        first["groups"][0]["desired"]["Plan"]["targets"],
        second["groups"][0]["desired"]["Plan"]["targets"]
    );
}

#[tokio::test]
async fn queued_discontinuity_before_healthy_latest_blocks_first_send_and_revokes_preview_baseline()
{
    for dry_run in [false, true] {
        let reducer = ready();
        let initial = reducer.capture();
        let (publisher, observer) = Publisher::new(initial.clone(), 64);
        let mut injected = false;
        let mut records = Vec::new();
        let mut sink = |value: Value| {
            if value["type"] == "longcat" && value["ready"] == true && !injected {
                injected = true;
                let mut before = initial.clone();
                // Exceed one eight-item drain budget. The same source/epoch then
                // loses a family and becomes healthy again before admission.
                for index in 1..=12 {
                    let mut after = (*initial).clone();
                    after.version = zk_desktop::ObservationVersion(initial.version.0 + index);
                    let continuity = if index == 11 {
                        after
                            .windows_status
                            .stale(zk_desktop::LossReason::InvalidSnapshot);
                        Continuity::Discontinuous(zk_desktop::LossReason::InvalidSnapshot)
                    } else {
                        Continuity::Continuous
                    };
                    let after = Arc::new(after);
                    publisher.publish(zk_desktop::DesktopUpdate::new(
                        before,
                        after.clone(),
                        continuity,
                    ));
                    before = after;
                }
                assert!(before.is_ready());
            }
            records.push(value);
            Ok(())
        };
        let executor: Executor =
            Arc::new(|_| panic!("unprocessed continuity must block all native mutation"));
        let mut command = command(&[]);
        command.dry_run = dry_run;
        command.shutdown_timeout_ms = 20;
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            drive(
                observer,
                Some(executor),
                &command,
                &LongcatConfig::default(),
                &mut sink,
                std::future::pending().boxed(),
            ),
        )
        .await
        .unwrap();
        assert!(result.is_err());
        assert_eq!(records.last().unwrap()["episodes"][0]["phase"], "Lost");
        assert_eq!(
            records.last().unwrap()["episodes"][0]["baseline"],
            records[0]["groups"][0]["baseline"]
        );
        if !dry_run {
            assert_eq!(
                records.last().unwrap()["episodes"][0]["requested_effect"]["commands"],
                serde_json::json!(["Unsent", "Unsent", "Unsent"])
            );
        }
    }
}

#[tokio::test]
async fn enabled_backends_acquire_read_only_through_real_native_subscriptions() {
    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::UnixListener,
    };
    for backend in ["niri", "umbriel"] {
        if backend == "umbriel" && !cfg!(feature = "umbriel") {
            continue;
        }
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("script.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let (release, released) = tokio::sync::oneshot::channel();
        let peer = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut line = String::new();
            BufReader::new(&mut socket)
                .read_line(&mut line)
                .await
                .unwrap();
            if backend == "niri" {
                assert_eq!(line.trim(), "\"EventStream\"");
                socket.write_all(b"{\"Ok\":\"Handled\"}\n").await.unwrap();
                for event in [
                    Event::WindowsChanged {
                        windows: vec![window(1, 1, 0, 400, true), window(2, 1, 1, 300, false)],
                    },
                    Event::WorkspacesChanged {
                        workspaces: vec![workspace(1)],
                    },
                ] {
                    let mut bytes = serde_json::to_vec(&event).unwrap();
                    bytes.push(b'\n');
                    socket.write_all(&bytes).await.unwrap();
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            } else {
                assert!(line.contains("subscribe"));
                assert!(line.contains("outputs"));
                socket
                    .write_all(b"{\"event\":\"windows\",\"data\":[]}\n")
                    .await
                    .unwrap();
                tokio::time::sleep(Duration::from_millis(5)).await;
                socket
                    .write_all(b"{\"event\":\"workspaces\",\"data\":[]}\n")
                    .await
                    .unwrap();
                socket
                    .write_all(b"{\"event\":\"outputs\",\"data\":[]}\n")
                    .await
                    .unwrap();
            }
            let _ = released.await;
            assert!(
                tokio::time::timeout(Duration::from_millis(20), listener.accept())
                    .await
                    .is_err(),
                "read-only acquisition must not open command connections"
            );
        });
        let cmd = command(&[
            "--backend",
            backend,
            "--dry-run",
            "--socket",
            path.to_str().unwrap(),
        ]);
        let (owner, observer, _) = native::subscribe(&cmd).await.unwrap();
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let mut sender = Some(sender);
        let mut records = Vec::new();
        let mut sink = |value: Value| {
            if value["ready"] == true
                && let Some(sender) = sender.take()
            {
                let _ = sender.send(());
            }
            records.push(value);
            Ok(())
        };
        let config=LongcatConfig::default();
        let stop=async move { receiver.await.map_err(io::Error::other) }.boxed();
        tokio::time::timeout(Duration::from_secs(2),async {
            #[cfg(feature="umbriel")]
            if let native::Owner::Umbriel(source)=&owner {
                return stock::drive(observer,stock::Services::native(source),&cmd,&config,&mut sink,stop).await;
            }
            drive(
                observer,
                None,
                &cmd,
                &config,
                &mut sink,
                stop,
            ).await
        })
        .await
        .unwrap()
        .unwrap();
        assert!(records.iter().any(|r| r["ready"] == true));
        if backend == "umbriel" {
            assert!(records.iter().any(|r|r["type"]=="longcat-feedback"));
        }
        owner.shutdown().await.unwrap();
        release.send(()).unwrap();
        peer.await.unwrap();
    }
}

#[tokio::test]
async fn scripted_niri_runner_executes_by_id_then_conditionally_restores_on_shutdown() {
    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::UnixListener,
    };
    async fn send(socket: &mut tokio::net::UnixStream, event: Event) {
        let mut bytes = serde_json::to_vec(&event).unwrap();
        bytes.push(b'\n');
        socket.write_all(&bytes).await.unwrap();
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("native.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let (release, released) = tokio::sync::oneshot::channel();
    let peer = tokio::spawn(async move {
        let (mut events, _) = listener.accept().await.unwrap();
        let mut line = String::new();
        BufReader::new(&mut events)
            .read_line(&mut line)
            .await
            .unwrap();
        assert_eq!(line.trim(), "\"EventStream\"");
        events.write_all(b"{\"Ok\":\"Handled\"}\n").await.unwrap();
        send(
            &mut events,
            Event::WorkspacesChanged {
                workspaces: vec![workspace(1)],
            },
        )
        .await;
        send(
            &mut events,
            Event::WindowsChanged {
                windows: vec![
                    window(1, 1, 0, 400, true),
                    window(2, 1, 1, 300, false),
                    window(3, 1, 2, 200, false),
                ],
            },
        )
        .await;
        for heights in [[630, 162, 108], [400, 300, 200]] {
            for (index, height) in heights.iter().enumerate() {
                let (mut command, _) = listener.accept().await.unwrap();
                let mut line = String::new();
                BufReader::new(&mut command)
                    .read_line(&mut line)
                    .await
                    .unwrap();
                let request: Value = serde_json::from_str(&line).unwrap();
                assert_eq!(request["Action"]["SetWindowHeight"]["id"], index + 1);
                assert_eq!(
                    request["Action"]["SetWindowHeight"]["change"]["SetFixed"],
                    *height
                );
                command.write_all(b"{\"Ok\":\"Handled\"}\n").await.unwrap();
            }
            send(
                &mut events,
                Event::WindowsChanged {
                    windows: heights
                        .into_iter()
                        .enumerate()
                        .map(|(index, height)| {
                            window(index as u64 + 1, 1, index, height, index == 0)
                        })
                        .collect(),
                },
            )
            .await;
        }
        let _ = released.await;
    });
    let command = command(&["--socket", path.to_str().unwrap()]);
    let (owner, observer, executor) = native::subscribe(&command).await.unwrap();
    let executor = executor.unwrap();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let mut sender = Some(sender);
    let mut records = Vec::new();
    let mut sink = |value: Value| {
        if value["groups"][0]["phase"] == "Settled"
            && let Some(sender) = sender.take()
        {
            let _ = sender.send(());
        }
        records.push(value);
        Ok(())
    };
    tokio::time::timeout(
        Duration::from_secs(3),
        drive(
            observer,
            Some(executor),
            &command,
            &LongcatConfig::default(),
            &mut sink,
            async move { receiver.await.map_err(io::Error::other) }.boxed(),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        records
            .iter()
            .filter(|record| record["type"] == "longcat-command")
            .count(),
        6
    );
    assert!(
        records
            .iter()
            .any(|record| record["groups"][0]["phase"] == "Restored")
    );
    owner.shutdown().await.unwrap();
    release.send(()).unwrap();
    peer.await.unwrap();
}
