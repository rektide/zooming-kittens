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
    {
        if let Some(sender) = sender.take() {
            let _ = sender.send(());
        }
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
        async { zk_niri::Transmission::Handled }.boxed()
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
            zk_niri::Transmission::Handled
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

#[tokio::test]
async fn both_backends_acquire_read_only_through_real_native_subscriptions() {
    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::UnixListener,
    };
    for backend in ["niri", "umbriel"] {
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
                socket
                    .write_all(b"{\"event\":\"windows\",\"data\":[]}\n")
                    .await
                    .unwrap();
                tokio::time::sleep(Duration::from_millis(5)).await;
                socket
                    .write_all(b"{\"event\":\"workspaces\",\"data\":[]}\n")
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
            if value["ready"] == true {
                if let Some(sender) = sender.take() {
                    let _ = sender.send(());
                }
            }
            records.push(value);
            Ok(())
        };
        tokio::time::timeout(
            Duration::from_secs(2),
            drive(
                observer,
                None,
                &cmd,
                &LongcatConfig::default(),
                &mut sink,
                async move { receiver.await.map_err(io::Error::other) }.boxed(),
            ),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(records.iter().any(|r| r["ready"] == true));
        if backend == "umbriel" {
            assert!(records.iter().any(|r| {
                r["unsupported"]
                    .as_str()
                    .is_some_and(|s| s.contains("grouping"))
            }));
        }
        owner.shutdown().await.unwrap();
        release.send(()).unwrap();
        peer.await.unwrap();
    }
}
