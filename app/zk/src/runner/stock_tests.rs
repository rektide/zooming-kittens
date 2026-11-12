use super::{
    Executor,
    native::{Request, Transmission},
    stock::{self, Services},
};
use crate::{commands::longcat::LongcatCommand, config::LongcatConfig};
use clap::Parser;
use futures::FutureExt;
use serde_json::Value;
use std::{
    future::Future,
    io,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};
use zk_desktop::*;

#[derive(Parser)]
struct Cli {
    #[command(flatten)]
    command: LongcatCommand,
}
fn command(extra: &[&str]) -> LongcatCommand {
    Cli::parse_from(
        [
            "longcat",
            "--backend",
            "umbriel",
            "--shutdown-timeout-ms",
            "100",
        ]
        .into_iter()
        .chain(extra.iter().copied()),
    )
    .command
}

fn capture(epoch: u64, horizontal: bool, extents: &[f64]) -> DesktopCapture {
    let scope = Scope {
        source: SourceId::new(Backend::Umbriel, "scripted"),
        epoch: Epoch(epoch),
    };
    let ws = WorkspaceKey {
        scope: scope.clone(),
        native: NativeId::Text("ws".into()),
    };
    let group = GroupKey {
        workspace: ws.clone(),
        position: NativeId::Number(0),
    };
    let out = OutputKey {
        scope: scope.clone(),
        name: "OUT".into(),
    };
    let mut c = DesktopCapture::empty(
        scope.clone(),
        Capabilities {
            reported_groups: true,
            relative_edge_by_id: true,
            ..Capabilities::READ_ONLY
        },
    );
    let status = FamilyStatus {
        stamp: Some(FamilyStamp {
            epoch: Epoch(epoch),
            position: 1,
        }),
        health: Health::Current,
    };
    c.windows_status = status.clone();
    c.workspaces_status = status.clone();
    c.outputs_status = Some(status);
    c.version.0 = 1;
    c.outputs.insert(
        out.clone(),
        OutputView {
            key: out.clone(),
            enabled: Some(true),
            powered: Some(true),
            position: Some(Point { x: 0., y: 0. }),
            logical_size: Some(Size {
                width: 1000.,
                height: 1000.,
            }),
            usable_area: Some(Rect {
                origin: Point { x: 0., y: 0. },
                size: Size {
                    width: 1000.,
                    height: 1000.,
                },
            }),
            scale: Some(1.),
            transform: Some("normal".into()),
        },
    );
    c.workspaces.insert(
        ws.clone(),
        WorkspaceView {
            key: ws.clone(),
            name: None,
            output: Some("OUT".into()),
            active: true,
            focused: true,
            layout: Some("scrolling".into()),
            geometry: None,
            grouping: Evidence::Reported,
        },
    );
    let mut start = 50.;
    let mut members = Vec::new();
    for (i, extent) in extents.iter().enumerate() {
        let id = WindowKey {
            scope: scope.clone(),
            native: NativeId::Text(format!("w{i}")),
        };
        c.windows.insert(
            id.clone(),
            WindowView {
                key: id.clone(),
                workspace: Some(ws.clone()),
                group: Some(group.clone()),
                output: Some(out.clone()),
                presentation: Presentation {
                    title_bar: Some(i % 2 == 0),
                    tabbed: Some(false),
                    tab_index: Some(-1),
                    tab_hidden: Some(false),
                    fullscreen: Some(false),
                    maximized: Some(false),
                    maximized_to_edges: Some(false),
                    pinned: Some(false),
                    minimized: Some(false),
                    visible: Some(true),
                    ..Presentation::default()
                },
                placement: Placement::Tiled,
                title: None,
                app_id: None,
                pid: None,
                active: i == 0,
                geometry: Geometry {
                    target_position: Some(if horizontal {
                        Point { x: start, y: 20. }
                    } else {
                        Point { x: 20., y: start }
                    }),
                    content_size: Some(if horizontal {
                        Size {
                            width: *extent,
                            height: 800.,
                        }
                    } else {
                        Size {
                            width: 800.,
                            height: *extent,
                        }
                    }),
                    ..Geometry::default()
                },
            },
        );
        members.push(id);
        start += extent + if i % 2 == 0 { 20. } else { 35. };
    }
    c.active_window = Some(members[0].clone());
    c.groups.insert(
        group.clone(),
        GroupView {
            key: group,
            members,
            secondary_axis: Some(if horizontal {
                Axis::Horizontal
            } else {
                Axis::Vertical
            }),
            membership: Evidence::Reported,
            bounds: None,
            geometry_evidence: Evidence::Inferred,
            edge_eligibility: EdgeEligibility::Eligible,
        },
    );
    c
}

#[derive(Clone)]
struct Peer {
    publisher: Arc<Publisher>,
    current: Arc<Mutex<DesktopCapture>>,
}
impl Peer {
    fn edit(&self, continuity: Continuity, edit: impl FnOnce(&mut DesktopCapture)) {
        let mut c = self.current.lock().unwrap();
        let before = Arc::new(c.clone());
        edit(&mut c);
        c.version.0 += 1;
        self.publisher
            .publish(DesktopUpdate::new(before, Arc::new(c.clone()), continuity));
    }
    fn focus(&self, index: Option<usize>) {
        self.edit(Continuity::Continuous, |c| {
            c.active_window = index.map(|i| c.groups.values().next().unwrap().members[i].clone());
        });
    }
    /// Scripted ideal pair response, not a native convergence oracle.
    fn apply(&self, request: &Request, commit: bool) {
        let Request::Edge(edge) = request else {
            panic!("not stock edge")
        };
        self.edit(Continuity::Continuous, |c| {
            let members = c.groups[&edge.request.group].members.clone();
            let i = members
                .iter()
                .position(|id| id == &edge.request.target)
                .unwrap();
            let delta = edge.request.fraction * 1000.;
            let horizontal = edge.request.axis == Axis::Horizontal;
            let p = c
                .windows
                .get_mut(&members[i + 1])
                .unwrap()
                .geometry
                .target_position
                .as_mut()
                .unwrap();
            if horizontal {
                p.x += delta;
            } else {
                p.y += delta;
            }
            if commit {
                for (index, amount) in [(i, delta), (i + 1, -delta)] {
                    let s = c
                        .windows
                        .get_mut(&members[index])
                        .unwrap()
                        .geometry
                        .content_size
                        .as_mut()
                        .unwrap();
                    if horizontal {
                        s.width += amount;
                    } else {
                        s.height += amount;
                    }
                }
            }
        });
    }
    fn vector(&self, extents: &[f64]) {
        self.edit(Continuity::Continuous, |c| {
            let ids = c.groups.values().next().unwrap().members.clone();
            let mut start = 50.;
            for (i, (id, extent)) in ids.iter().zip(extents).enumerate() {
                let w = c.windows.get_mut(id).unwrap();
                w.geometry.target_position.as_mut().unwrap().y = start;
                w.geometry.content_size.as_mut().unwrap().height = *extent;
                start += extent + if i % 2 == 0 { 20. } else { 35. };
            }
        });
    }
}
struct Call {
    request: Request,
    reply: oneshot::Sender<Transmission>,
}
async fn call(calls: &mut mpsc::UnboundedReceiver<Call>) -> Call {
    tokio::time::timeout(Duration::from_secs(2), calls.recv())
        .await
        .expect("next command deadline")
        .expect("command owner")
}
async fn quiet(calls: &mut mpsc::UnboundedReceiver<Call>, ms: u64) {
    if let Ok(call) = tokio::time::timeout(Duration::from_millis(ms), calls.recv()).await {
        panic!("unexpected native command: {:?}", call.map(|c| c.request));
    }
}
fn handled(peer: &Peer, call: Call, commit: bool) {
    peer.apply(&call.request, commit);
    call.reply.send(Transmission::Handled).unwrap();
}

async fn scripted<F, Fut>(
    initial: DesktopCapture,
    config: LongcatConfig,
    extra: &[&str],
    refresh_delay: u64,
    script: F,
) -> (io::Result<()>, Vec<Value>, usize, usize)
where
    F: FnOnce(Peer, mpsc::UnboundedReceiver<Call>, oneshot::Sender<()>, Arc<AtomicUsize>) -> Fut,
    Fut: Future<Output = ()>,
{
    scripted_output(initial, config, extra, refresh_delay, false, script).await
}

async fn scripted_output<F, Fut>(
    initial: DesktopCapture,
    config: LongcatConfig,
    extra: &[&str],
    refresh_delay: u64,
    fail_pending_output: bool,
    script: F,
) -> (io::Result<()>, Vec<Value>, usize, usize)
where
    F: FnOnce(Peer, mpsc::UnboundedReceiver<Call>, oneshot::Sender<()>, Arc<AtomicUsize>) -> Fut,
    Fut: Future<Output = ()>,
{
    let (publisher, observer) = Publisher::new(Arc::new(initial.clone()), 64);
    let peer = Peer {
        publisher: Arc::new(publisher),
        current: Arc::new(Mutex::new(initial)),
    };
    let (tx, rx) = mpsc::unbounded_channel();
    let executor: Executor = Arc::new(move |request| {
        let tx = tx.clone();
        async move {
            let (reply, result) = oneshot::channel();
            if tx.send(Call { request, reply }).is_err() {
                return Transmission::Unsent("script ended".into());
            }
            result
                .await
                .unwrap_or(Transmission::PossiblySent("script dropped receipt".into()))
        }
        .boxed()
    });
    let probes = Arc::new(AtomicUsize::new(0));
    let probe_count = probes.clone();
    let refreshes = Arc::new(AtomicUsize::new(0));
    let refresh_count = refreshes.clone();
    let services = Services {
        verify: Box::new(move || {
            probe_count.fetch_add(1, Ordering::SeqCst);
            std::future::ready(Ok(executor.clone())).boxed()
        }),
        refresh: Box::new(move |_, _| {
            refresh_count.fetch_add(1, Ordering::SeqCst);
            async move {
                tokio::time::sleep(Duration::from_millis(refresh_delay)).await;
                Ok(())
            }
            .boxed()
        }),
    };
    let (stop, stopped) = oneshot::channel();
    let command = command(extra);
    let mut rows = Vec::new();
    let mut failed = false;
    let mut sink = |row: Value| {
        if fail_pending_output
            && !failed
            && row["type"] == "longcat-feedback"
            && row["current"]["commands"]
                .as_array()
                .is_some_and(|commands| !commands.is_empty())
        {
            failed = true;
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "scripted diagnostic failure",
            ));
        }
        rows.push(row);
        Ok(())
    };
    let _keepalive = peer.clone();
    let drive = stock::drive(
        observer,
        services,
        &command,
        &config,
        &mut sink,
        async move { stopped.await.map_err(io::Error::other) }.boxed(),
    );
    let (result, ()) = tokio::join!(drive, script(peer, rx, stop, probes.clone()));
    (
        result,
        rows,
        probes.load(Ordering::SeqCst),
        refreshes.load(Ordering::SeqCst),
    )
}

#[tokio::test]
async fn diagnostic_failure_revokes_tail_but_drains_the_owned_receipt() {
    let (result, rows, _, _) = scripted_output(
        capture(1, false, &[450., 450.]),
        config(),
        &[],
        0,
        true,
        |peer, mut calls, stop, _| async move {
            let _keep_stop = stop;
            let first = call(&mut calls).await;
            peer.apply(&first.request, true);
            tokio::time::sleep(Duration::from_millis(20)).await;
            first
                .reply
                .send(Transmission::Handled)
                .expect("runner must still own receipt after sink failure");
            assert!(
                calls.recv().await.is_none(),
                "diagnostic failure must not admit a tail"
            );
        },
    )
    .await;
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::BrokenPipe);
    assert_eq!(completed(&rows)[0]["commands"][0]["outcome"], "Handled");
    assert_eq!(rows.last().unwrap()["counters"]["submissions"], 1);
}

#[tokio::test]
async fn queued_loss_then_healthy_same_epoch_never_restores_authority() {
    let (result, rows, probes, _) = scripted(
        capture(1, false, &[450., 450.]),
        config(),
        &[],
        0,
        |peer, mut calls, stop, _| async move {
            let first = call(&mut calls).await;
            peer.edit(Continuity::Discontinuous(LossReason::Disconnected), |c| {
                c.connected = false
            });
            peer.edit(Continuity::Continuous, |c| c.connected = true);
            first.reply.send(Transmission::Handled).unwrap();
            quiet(&mut calls, 70).await;
            stop.send(()).unwrap();
        },
    )
    .await;
    result.unwrap();
    assert_eq!(probes, 1);
    assert_eq!(rows.last().unwrap()["counters"]["submissions"], 1);
    assert!(rows.iter().any(|r| r["type"] == "longcat-feedback"
        && r["ready"] == false
        && r["version"].as_u64().is_some_and(|v| v > 1)));
}
fn config() -> LongcatConfig {
    LongcatConfig {
        resize_debounce_ms: 10,
        ..LongcatConfig::default()
    }
}
fn completed(rows: &[Value]) -> Vec<&Value> {
    rows.iter()
        .filter(|r| r["type"] == "longcat-feedback-completed")
        .map(|r| &r["attempt"])
        .collect()
}

#[test]
fn stock_is_explicit_default_and_axis_minima_are_independent() {
    let cmd = command(&["--minimum-width", "83"]);
    assert_eq!(
        cmd.umbriel_control,
        crate::commands::longcat::UmbrielControl::Stock
    );
    let config = cmd.compose(LongcatConfig::default()).unwrap();
    assert_eq!((config.minimum_height, config.minimum_width), (60, 83));
    assert!(command(&["--minimum-width", "0"]).compose(config).is_err());
    assert_eq!(
        command(&["--umbriel-control", "conditional"]).umbriel_control,
        crate::commands::longcat::UmbrielControl::Conditional
    );
}

#[tokio::test]
async fn quiet_single_capture_drives_both_axes_and_restores_without_duplicate_events() {
    for horizontal in [false, true] {
        let (result, rows, probes, refreshes) = scripted(
            capture(1, horizontal, &[450., 450.]),
            config(),
            &[],
            0,
            |peer, mut calls, stop, _| async move {
                let first = call(&mut calls).await;
                let Request::Edge(edge) = &first.request else {
                    panic!()
                };
                assert_eq!(edge.travel, 16.);
                assert_eq!(
                    edge.request.axis,
                    if horizontal {
                        Axis::Horizontal
                    } else {
                        Axis::Vertical
                    }
                );
                handled(&peer, first, true);
                handled(&peer, call(&mut calls).await, true);
                quiet(&mut calls, 30).await;
                peer.focus(None);
                handled(&peer, call(&mut calls).await, true);
                quiet(&mut calls, 30).await;
                stop.send(()).unwrap();
            },
        )
        .await;
        result.unwrap();
        assert_eq!(probes, 1);
        assert_eq!(refreshes, 0);
        assert_eq!(
            completed(&rows)
                .iter()
                .filter(|a| a["result"]["end"] == "Settled")
                .count(),
            2
        );
        let final_row = rows.last().unwrap();
        assert_eq!(final_row["counters"]["guidance"], 0);
        for pref in final_row["preferences"].as_array().unwrap() {
            assert_eq!(pref["neutral_weight"], 450.);
            assert!(pref["active_share"].is_null());
        }
    }
}

#[tokio::test]
async fn response_before_reply_cannot_advance_and_focus_change_cancels_old_tail() {
    let (result, rows, _, _) = scripted(
        capture(1, false, &[450., 450.]),
        config(),
        &[],
        0,
        |peer, mut calls, stop, _| async move {
            let first = call(&mut calls).await;
            peer.apply(&first.request, true);
            quiet(&mut calls, 40).await;
            peer.focus(None);
            first.reply.send(Transmission::Handled).unwrap();
            let restore = call(&mut calls).await;
            let Request::Edge(edge) = &restore.request else {
                panic!()
            };
            assert!(edge.travel < 0., "old emphasis tail escaped after blur");
            handled(&peer, restore, true);
            quiet(&mut calls, 25).await;
            stop.send(()).unwrap();
        },
    )
    .await;
    result.unwrap();
    assert!(
        completed(&rows)
            .iter()
            .any(|a| a["result"]["end"]["Retired"] == "FocusChanged")
    );
    assert_eq!(
        rows.iter()
            .filter(|r| r["type"] == "longcat-edge-command")
            .count(),
        2
    );
}

#[tokio::test]
async fn loss_and_fresh_epoch_do_not_bypass_pending_receipt_or_replay_delta() {
    let (result, rows, probes, _) = scripted(
        capture(1, false, &[450., 450.]),
        config(),
        &[],
        0,
        |peer, mut calls, stop, probes| async move {
            let first = call(&mut calls).await;
            peer.edit(Continuity::Discontinuous(LossReason::Disconnected), |c| {
                c.connected = false
            });
            peer.edit(Continuity::Discontinuous(LossReason::EpochChanged), |c| {
                *c = capture(2, false, &[450., 450.])
            });
            quiet(&mut calls, 40).await;
            assert_eq!(
                probes.load(Ordering::SeqCst),
                1,
                "new authority before receipt drain"
            );
            first
                .reply
                .send(Transmission::PossiblySent(
                    "old connection reply lost".into(),
                ))
                .unwrap();
            let fresh = call(&mut calls).await;
            let Request::Edge(edge) = &fresh.request else {
                panic!()
            };
            assert_eq!(edge.request.target.scope.epoch, Epoch(2));
            assert_eq!(edge.travel, 16.);
            handled(&peer, fresh, true);
            handled(&peer, call(&mut calls).await, true);
            quiet(&mut calls, 25).await;
            stop.send(()).unwrap();
            handled(&peer, call(&mut calls).await, true);
        },
    )
    .await;
    result.unwrap();
    assert_eq!(probes, 2);
    assert!(
        completed(&rows)
            .iter()
            .any(|a| a["commands"][0]["outcome"]["PossiblySent"].is_object())
    );
}

#[tokio::test]
async fn silent_command_queries_once_waits_for_refresh_and_holds_limited_goal() {
    let (result, rows, _, refreshes) = scripted(
        capture(1, false, &[450., 450.]),
        config(),
        &[],
        100,
        |_, mut calls, stop, _| async move {
            call(&mut calls)
                .await
                .reply
                .send(Transmission::Handled)
                .unwrap();
            quiet(&mut calls, 220).await;
            stop.send(()).unwrap();
        },
    )
    .await;
    result.unwrap();
    assert_eq!(refreshes, 1);
    let attempts = completed(&rows);
    assert_eq!(attempts.len(), 1);
    assert!(attempts[0]["result"]["end"]["Limited"].is_object());
    assert_eq!(attempts[0]["commands"].as_array().unwrap().len(), 1);
    let refresh_done = rows
        .iter()
        .position(|r| r["type"] == "longcat-windows-refresh")
        .unwrap();
    let limited = rows
        .iter()
        .position(|r| r["type"] == "longcat-feedback-completed")
        .unwrap();
    assert!(
        refresh_done < limited,
        "classified absence before owned refresh finished"
    );
}

#[tokio::test]
async fn continuous_motion_yields_then_teaches_share_and_blur_uses_current_preferences() {
    let mut cfg = config();
    cfg.resize_debounce_ms = 30;
    let (result, rows, _, _) = scripted(
        capture(1, false, &[450., 450.]),
        cfg,
        &[],
        0,
        |peer, mut calls, stop, _| async move {
            handled(&peer, call(&mut calls).await, true);
            handled(&peer, call(&mut calls).await, true);
            quiet(&mut calls, 45).await;
            for a in [610., 590., 570., 550., 540.] {
                peer.vector(&[a, 900. - a]);
                quiet(&mut calls, 15).await;
            }
            quiet(&mut calls, 60).await;
            peer.focus(None);
            handled(&peer, call(&mut calls).await, true);
            handled(&peer, call(&mut calls).await, true);
            quiet(&mut calls, 45).await;
            peer.focus(Some(0));
            handled(&peer, call(&mut calls).await, true);
            quiet(&mut calls, 45).await;
            {
                let c = peer.current.lock().unwrap();
                assert_eq!(
                    c.windows
                        .values()
                        .next()
                        .unwrap()
                        .geometry
                        .content_size
                        .unwrap()
                        .height,
                    540.
                );
            }
            stop.send(()).unwrap();
            handled(&peer, call(&mut calls).await, true);
        },
    )
    .await;
    result.unwrap();
    let pref = &rows.last().unwrap()["preferences"][0];
    assert_eq!(pref["neutral_weight"], 450.);
    assert!((pref["active_share"].as_f64().unwrap() - 0.6).abs() < 1e-9);
    assert_eq!(rows.last().unwrap()["counters"]["guidance"], 1);
}

#[tokio::test]
async fn cumulative_late_commits_keep_budget_and_latest_motion_key_without_learning() {
    let (result, rows, _, _) = scripted(
        capture(1, false, &[400., 300., 200.]),
        config(),
        &[],
        0,
        |peer, mut calls, stop, _| async move {
            for _ in 0..3 {
                handled(&peer, call(&mut calls).await, false);
            }
            quiet(&mut calls, 20).await;
            peer.edit(Continuity::Continuous, |c| {
                for (w, h) in c.windows.values_mut().zip([416., 284., 200.]) {
                    w.geometry.content_size.as_mut().unwrap().height = h;
                }
            });
            quiet(&mut calls, 20).await;
            peer.edit(Continuity::Continuous, |c| {
                for (w, h) in c.windows.values_mut().zip([630., 162., 108.]) {
                    w.geometry.content_size.as_mut().unwrap().height = h;
                }
            });
            quiet(&mut calls, 30).await;
            stop.send(()).unwrap();
            // Shutdown is allowed to drain with an honestly uncertain/unsent tail.
            while let Some(call) = calls.recv().await {
                handled(&peer, call, true);
            }
        },
    )
    .await;
    result.unwrap();
    assert_eq!(rows.last().unwrap()["counters"]["guidance"], 0);
    let first = completed(&rows)[0];
    assert_eq!(first["result"]["end"], "Settled");
    assert_eq!(first["goal"]["inputs"]["content_budget"], 900.);
    let prefs = rows.last().unwrap()["preferences"].as_array().unwrap();
    assert_eq!(
        prefs
            .iter()
            .map(|p| p["neutral_weight"].as_f64().unwrap())
            .collect::<Vec<_>>(),
        [400., 300., 200.]
    );
}

#[tokio::test]
async fn clamped_recipient_tail_compensation_is_limited_without_teaching_siblings() {
    for reply_first in [false, true] {
        let (result, rows, _, _) = scripted(
            capture(1, false, &[400., 300., 200.]),
            config(),
            &[],
            0,
            move |peer, mut calls, stop, _| async move {
                let first = call(&mut calls).await;
                let mut reply = Some(first.reply);
                if reply_first {
                    reply.take().unwrap().send(Transmission::Handled).unwrap();
                }
                // Real native recipient maximum: its boundary does not move,
                // but the requested donor loses the 16px and the residual last
                // row gets it. This must not teach a fresh sibling ratio and
                // acquire another 16px attempt against the same native clamp.
                peer.vector(&[400., 284., 216.]);
                if let Some(reply) = reply {
                    reply.send(Transmission::Handled).unwrap();
                }
                quiet(&mut calls, 120).await;
                stop.send(()).unwrap();
                while let Some(call) = calls.recv().await {
                    handled(&peer, call, true);
                }
            },
        )
        .await;
        result.unwrap();
        assert_eq!(rows.last().unwrap()["counters"]["guidance"], 0);
        assert_eq!(rows.last().unwrap()["counters"]["submissions"], 1);
        let end = &completed(&rows)[0]["result"]["end"];
        assert!(end["Limited"]["Executor"].is_object(), "{end}");
        for (p, expected) in rows.last().unwrap()["preferences"]
            .as_array()
            .unwrap()
            .iter()
            .zip([400., 300., 200.])
        {
            assert_eq!(p["neutral_weight"], expected);
            assert!(p["active_share"].is_null());
            assert!(p["manual_floor"].is_null());
        }
    }
}

#[tokio::test]
async fn coupled_native_response_is_limited_not_guidance_or_reacquisition_loop() {
    let (result, rows, _, _) = scripted(
        capture(1, false, &[400., 300., 200.]),
        config(),
        &[],
        0,
        |peer, mut calls, stop, _| async move {
            let first = call(&mut calls).await;
            peer.vector(&[416., 292., 192.]); // requested boundary +16, another +8
            first.reply.send(Transmission::Handled).unwrap();
            quiet(&mut calls, 120).await;
            stop.send(()).unwrap();
            while let Some(call) = calls.recv().await {
                handled(&peer, call, true);
            }
        },
    )
    .await;
    result.unwrap();
    assert_eq!(completed(&rows).len(), 1);
    assert!(
        completed(&rows)[0]["result"]["end"]["Limited"]["Executor"]["reason"]
            .as_str()
            .unwrap()
            .contains("coupled native")
    );
    assert_eq!(rows.last().unwrap()["counters"]["guidance"], 0);
    assert_eq!(rows.last().unwrap()["counters"]["submissions"], 1);
}

#[tokio::test]
async fn shutdown_with_unanswered_edge_is_bounded_and_never_replays() {
    let started = tokio::time::Instant::now();
    let (result, rows, _, _) = scripted(
        capture(1, false, &[450., 450.]),
        config(),
        &[],
        0,
        |_, mut calls, stop, _| async move {
            let pending = call(&mut calls).await;
            stop.send(()).unwrap();
            tokio::time::sleep(Duration::from_millis(160)).await;
            drop(pending);
            assert!(calls.recv().await.is_none());
        },
    )
    .await;
    result.unwrap();
    assert!(started.elapsed() < Duration::from_secs(1));
    let completed = completed(&rows);
    assert_eq!(completed.len(), 1);
    assert!(completed[0]["commands"][0]["outcome"]["PossiblySent"].is_object());
    assert_eq!(rows.last().unwrap()["counters"]["submissions"], 1);
}

#[tokio::test]
async fn topology_change_stops_old_tail_and_acquires_new_order_after_receipt() {
    let (result, rows, _, _) = scripted(
        capture(1, false, &[400., 300., 200.]),
        config(),
        &[],
        0,
        |peer, mut calls, stop, _| async move {
            let first = call(&mut calls).await;
            peer.apply(&first.request, true);
            peer.edit(Continuity::Continuous, |c| {
                let group = c.groups.values_mut().next().unwrap();
                group.members.reverse();
                let mut start = 50.;
                for id in &group.members {
                    let w = c.windows.get_mut(id).unwrap();
                    w.geometry.target_position.as_mut().unwrap().y = start;
                    start += w.geometry.content_size.unwrap().height + 20.;
                }
            });
            quiet(&mut calls, 25).await;
            first.reply.send(Transmission::Handled).unwrap();
            let next = call(&mut calls).await;
            let Request::Edge(edge) = &next.request else {
                panic!()
            };
            assert_eq!(
                edge.request.following.native,
                NativeId::Text("w0".into()),
                "old ordered tail escaped"
            );
            assert_eq!(
                edge.travel.abs(),
                16.,
                "old geometry scale crossed topology change"
            );
            handled(&peer, next, true);
            stop.send(()).unwrap();
            while let Some(call) = calls.recv().await {
                handled(&peer, call, true);
            }
        },
    )
    .await;
    result.unwrap();
    let attempts = completed(&rows);
    assert_eq!(attempts[0]["commands"].as_array().unwrap().len(), 1);
    assert_eq!(attempts[0]["result"]["end"]["Retired"], "TopologyChanged");
    assert_eq!(rows.last().unwrap()["counters"]["guidance"], 0);
}

#[tokio::test]
async fn output_context_change_reacquires_but_unrelated_output_does_not_reset_quietness() {
    let (result, rows, _, _) = scripted(
        capture(1, false, &[450., 450.]),
        config(),
        &[],
        0,
        |peer, mut calls, stop, _| async move {
            let first = call(&mut calls).await;
            peer.apply(&first.request, true);
            peer.edit(Continuity::Continuous, |c| {
                c.outputs
                    .values_mut()
                    .next()
                    .unwrap()
                    .usable_area
                    .as_mut()
                    .unwrap()
                    .size
                    .height = 1200.;
            });
            first.reply.send(Transmission::Handled).unwrap();
            let next = call(&mut calls).await;
            let Request::Edge(edge) = &next.request else {
                panic!()
            };
            assert_eq!(edge.travel, 16.);
            assert!((edge.request.fraction - 16. / 1200.).abs() < 1e-9);
            // Scripted transfer scale follows this changed context for the first
            // response, then the real empirical gain is used for the remaining move.
            handled(&peer, next, true);
            handled(&peer, call(&mut calls).await, true);
            quiet(&mut calls, 25).await;
            peer.edit(Continuity::Continuous, |c| {
                let mut output = c.outputs.values().next().unwrap().clone();
                output.key.name = "UNRELATED".into();
                output.scale = Some(2.);
                c.outputs.insert(output.key.clone(), output);
            });
            quiet(&mut calls, 25).await;
            stop.send(()).unwrap();
            while let Some(call) = calls.recv().await {
                handled(&peer, call, true);
            }
        },
    )
    .await;
    result.unwrap();
    assert_eq!(rows.last().unwrap()["counters"]["guidance"], 0);
    assert!(
        completed(&rows)
            .iter()
            .any(|a| a["result"]["end"]["Retired"] == "ContextChanged")
    );
}

#[tokio::test]
async fn seventy_focus_changes_have_no_historical_lifetime_veto_or_neutral_drift() {
    let mut cfg = config();
    cfg.resize_debounce_ms = 0;
    let (result, rows, _, _) = scripted(
        capture(1, false, &[450., 450.]),
        cfg,
        &[],
        0,
        |peer, mut calls, stop, _| async move {
            handled(&peer, call(&mut calls).await, true);
            handled(&peer, call(&mut calls).await, true);
            quiet(&mut calls, 2).await;
            for i in 0..70 {
                peer.focus(Some((i + 1) % 2));
                handled(&peer, call(&mut calls).await, true);
                quiet(&mut calls, 2).await;
            }
            stop.send(()).unwrap();
            handled(&peer, call(&mut calls).await, true);
        },
    )
    .await;
    result.unwrap();
    assert!(
        rows.last().unwrap()["counters"]["settled"]
            .as_u64()
            .unwrap()
            > 64
    );
    for p in rows.last().unwrap()["preferences"].as_array().unwrap() {
        assert_eq!(p["neutral_weight"], 450.);
        assert!(p["active_share"].is_null());
    }
}

#[tokio::test]
async fn overlapping_independent_motion_is_unattributed_guidance_after_receipt_drain() {
    for reply_first in [false, true] {
        let mut cfg = config();
        cfg.resize_debounce_ms = 25;
        let (result, rows, _, _) = scripted(
            capture(1, false, &[450., 450.]),
            cfg,
            &[],
            0,
            move |peer, mut calls, stop, _| async move {
                let first = call(&mut calls).await;
                let mut reply = Some(first.reply);
                if reply_first {
                    reply.take().unwrap().send(Transmission::Handled).unwrap();
                }
                // The requested edge was positive. Reversal is independent/ambiguous,
                // not a coupled native clamp to be held indefinitely.
                peer.vector(&[420., 480.]);
                quiet(&mut calls, 40).await;
                if let Some(reply) = reply {
                    reply.send(Transmission::Handled).unwrap();
                }
                quiet(&mut calls, 50).await;
                stop.send(()).unwrap();
                while let Some(call) = calls.recv().await {
                    handled(&peer, call, true);
                }
            },
        )
        .await;
        result.unwrap();
        assert!(
            rows.iter()
                .flat_map(|r| r["guidance"].as_array().into_iter().flatten())
                .any(|g| g["reason"] == "UnattributedOverlap")
        );
        assert!(
            (rows.last().unwrap()["preferences"][0]["active_share"]
                .as_f64()
                .unwrap()
                - 420. / 900.)
                .abs()
                < 1e-9
        );
    }
}

#[test]
fn reacquiring_fresh_families_does_not_mark_the_new_epoch_lost_again() {
    let mut control = super::feedback::Control::new(&config(), 1500).unwrap();
    let initial = capture(1, false, &[450., 450.]);
    control.install(&initial, 0).unwrap();
    control.install(&initial, 10).unwrap();
    control.lose(&initial.scope, "transport loss", 11).unwrap();
    let mut next = capture(2, false, &[450., 450.]);
    next.windows_status.health = Health::Stale(LossReason::EpochChanged);
    next.workspaces_status.health = Health::Stale(LossReason::EpochChanged);
    let mut before = Arc::new(next.clone());
    control.install(&before, 12).unwrap();
    for family in [0, 1] {
        if family == 0 {
            next.windows_status.health = Health::Current;
        } else {
            next.workspaces_status.health = Health::Current;
        }
        next.version.0 += 1;
        let after = Arc::new(next.clone());
        control
            .update(
                &DesktopUpdate::new(
                    before,
                    after.clone(),
                    Continuity::Discontinuous(LossReason::EpochChanged),
                ),
                13 + family,
            )
            .unwrap();
        before = after;
    }
    assert!(control.usable(&next));
    assert_eq!(control.feedback.input().unwrap().source.epoch, 2);
}

#[test]
fn unprojectable_group_retires_its_expired_acquisition_timer_instead_of_spinning() {
    for missing_geometry in [false, true] {
        let mut control = super::feedback::Control::new(&config(), 1500).unwrap();
        let mut c = capture(1, false, &[450., 450.]);
        control.install(&c, 0).unwrap();
        assert_eq!(control.next_deadline(false), Some(10));
        c.version.0 += 1;
        if missing_geometry {
            c.windows.values_mut().next().unwrap().geometry.content_size = None;
        } else {
            c.groups.values_mut().next().unwrap().members.pop();
        }
        control.install(&c, 20).unwrap();
        assert_eq!(control.next_deadline(false), None);
    }
}

#[tokio::test]
async fn incompatible_client_commit_during_own_response_is_not_taught_as_user_share() {
    let (result, rows, _, _) = scripted(
        capture(1, false, &[450., 450.]),
        config(),
        &[],
        0,
        |peer, mut calls, stop, _| async move {
            let first = call(&mut calls).await;
            peer.apply(&first.request, false);
            quiet(&mut calls, 5).await;
            peer.edit(Continuity::Continuous, |c| {
                // Target starts remain at the owned 466/434 allocation. The client
                // instead commits outside that bounded cumulative prediction.
                for (w, h) in c.windows.values_mut().zip([500., 400.]) {
                    w.geometry.content_size.as_mut().unwrap().height = h;
                }
            });
            first.reply.send(Transmission::Handled).unwrap();
            quiet(&mut calls, 80).await;
            stop.send(()).unwrap();
            while let Some(call) = calls.recv().await {
                handled(&peer, call, true);
            }
        },
    )
    .await;
    result.unwrap();
    assert_eq!(rows.last().unwrap()["counters"]["guidance"], 0);
    assert_eq!(rows.last().unwrap()["counters"]["submissions"], 1);
    assert!(rows.last().unwrap()["preferences"][0]["active_share"].is_null());
    assert!(completed(&rows)[0]["result"]["end"]["Limited"].is_object());
}
