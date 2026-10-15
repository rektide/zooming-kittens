use niri_ipc::{Event, Window, WindowLayout, Workspace};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
    sync::oneshot,
    time::timeout,
};
use zk_desktop::*;
use zk_niri::{HeightRequest, Options, Transmission, subscribe};

async fn accept(listener: &UnixListener) -> UnixStream {
    let (mut socket, _) = listener.accept().await.unwrap();
    let mut line = String::new();
    BufReader::new(&mut socket)
        .read_line(&mut line)
        .await
        .unwrap();
    assert_eq!(line.trim(), "\"EventStream\"");
    socket.write_all(b"{\"Ok\":\"Handled\"}\n").await.unwrap();
    socket
}
async fn send(socket: &mut UnixStream, event: Event) {
    let mut bytes = serde_json::to_vec(&event).unwrap();
    bytes.push(b'\n');
    socket.write_all(&bytes).await.unwrap();
}
fn source_id() -> SourceId {
    SourceId::new(Backend::Niri, "script")
}
async fn ready(observer: &mut Observer) -> Arc<DesktopCapture> {
    timeout(Duration::from_secs(5), async {
        loop {
            let update = observer.recv().await.unwrap();
            if update.after.is_ready() {
                return update.after.clone();
            }
        }
    })
    .await
    .unwrap()
}
async fn seed(socket: &mut UnixStream) {
    send(
        socket,
        Event::WindowsChanged {
            windows: vec![Window {
                id: 7,
                title: None,
                app_id: None,
                pid: None,
                workspace_id: Some(1),
                is_focused: true,
                is_floating: false,
                is_urgent: false,
                focus_timestamp: None,
                layout: WindowLayout {
                    pos_in_scrolling_layout: Some((1, 1)),
                    tile_size: (800., 600.),
                    window_size: (798, 598),
                    tile_pos_in_workspace_view: Some((0., 0.)),
                    window_offset_in_tile: (1., 1.),
                },
            }],
        },
    )
    .await;
    send(
        socket,
        Event::WorkspacesChanged {
            workspaces: vec![Workspace {
                id: 1,
                idx: 1,
                name: None,
                output: Some("test".into()),
                is_urgent: false,
                is_active: true,
                is_focused: true,
                active_window_id: Some(7),
            }],
        },
    )
    .await;
}

#[tokio::test]
async fn idle_source_shutdown_joins_and_closes_socket() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("niri.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let (release, wait) = oneshot::channel();
    let peer = tokio::spawn(async move {
        let mut socket = accept(&listener).await;
        seed(&mut socket).await;
        wait.await.unwrap();
        assert!(socket.write_all(b"after close\n").await.is_err());
    });
    let (source, mut observer) = subscribe(&path, source_id(), Options::default())
        .await
        .unwrap();
    assert!(ready(&mut observer).await.is_ready());
    timeout(Duration::from_secs(1), source.shutdown())
        .await
        .unwrap()
        .unwrap();
    assert!(!observer.current().connected);
    assert!(matches!(
        observer.current().windows_status.health,
        Health::Stale(LossReason::Shutdown)
    ));
    release.send(()).unwrap();
    peer.await.unwrap();
}

#[tokio::test]
async fn frame_bound_and_partial_eof_are_explicit_loss() {
    for payload in [vec![b'x'; 100], b"{\"WindowsChanged\":".to_vec()] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("niri.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let peer = tokio::spawn(async move {
            let mut socket = accept(&listener).await;
            socket.write_all(&payload).await.unwrap();
        });
        let (source, mut observer) = subscribe(
            &path,
            source_id(),
            Options {
                frame_limit: 64,
                ..Options::default()
            },
        )
        .await
        .unwrap();
        let loss = timeout(Duration::from_secs(1), observer.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(!loss.after.connected);
        assert!(matches!(loss.continuity, Continuity::Discontinuous(_)));
        source.shutdown().await.unwrap();
        peer.await.unwrap();
    }
}

#[tokio::test]
async fn slow_semantic_consumer_does_not_block_native_receive() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("niri.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let (release, wait) = oneshot::channel();
    let peer = tokio::spawn(async move {
        let mut socket = accept(&listener).await;
        for _ in 0..100 {
            send(&mut socket, Event::WindowsChanged { windows: vec![] }).await;
        }
        let _ = wait.await;
    });
    let (source, mut observer) = subscribe(
        &path,
        source_id(),
        Options {
            observer_capacity: 1,
            ..Options::default()
        },
    )
    .await
    .unwrap();
    timeout(Duration::from_secs(2), async {
        while observer.current().version.0 < 100 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        observer.recv().await,
        Err(ObserveError::Lagged { .. })
    ));
    source.shutdown().await.unwrap();
    let _ = release.send(());
    peer.await.unwrap();
}

#[tokio::test]
async fn by_id_native_reply_and_unknown_outcome_are_distinct_without_replay() {
    for native_reply in [
        Some(b"{\"Ok\":\"Handled\"}\n".as_slice()),
        Some(b"{\"Err\":\"denied\"}\n".as_slice()),
        None,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("niri.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let (release, wait) = oneshot::channel();
        let peer = tokio::spawn(async move {
            let mut events = accept(&listener).await;
            seed(&mut events).await;
            let (mut command, _) = listener.accept().await.unwrap();
            let mut line = String::new();
            BufReader::new(&mut command)
                .read_line(&mut line)
                .await
                .unwrap();
            let value: serde_json::Value = serde_json::from_str(&line).unwrap();
            assert_eq!(value["Action"]["SetWindowHeight"]["id"], 7);
            assert_eq!(
                value["Action"]["SetWindowHeight"]["change"]["SetFixed"],
                630
            );
            if let Some(reply) = native_reply {
                command.write_all(reply).await.unwrap();
            }
            drop(command);
            let _ = wait.await;
        });
        let (source, mut observer) = subscribe(&path, source_id(), Options::default())
            .await
            .unwrap();
        let capture = ready(&mut observer).await;
        let actuator = source.actuator();
        let request = HeightRequest {
            target: capture.active_window.clone().unwrap(),
            basis: capture.version,
            height: 630,
        };
        let result = actuator.set_height(request.clone()).await;
        match native_reply {
            Some(reply) if reply.starts_with(b"{\"Ok") => assert_eq!(result, Transmission::Handled),
            Some(_) => assert_eq!(result, Transmission::Rejected("denied".into())),
            None => assert!(matches!(result, Transmission::PossiblySent(_))),
        }
        source.shutdown().await.unwrap();
        assert!(matches!(
            actuator.set_height(request).await,
            Transmission::Unsent(_)
        ));
        let _ = release.send(());
        peer.await.unwrap();
    }
}

#[tokio::test]
async fn drop_requests_idle_shutdown_and_stale_basis_is_unsent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("niri.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let (release, wait) = oneshot::channel();
    let peer = tokio::spawn(async move {
        let mut socket = accept(&listener).await;
        seed(&mut socket).await;
        let _ = wait.await;
    });
    let (source, mut observer) = subscribe(&path, source_id(), Options::default())
        .await
        .unwrap();
    let capture = ready(&mut observer).await;
    let result = source
        .actuator()
        .set_height(HeightRequest {
            target: capture.active_window.clone().unwrap(),
            basis: ObservationVersion(0),
            height: 700,
        })
        .await;
    assert!(matches!(result, Transmission::Unsent(_)));
    drop(source);
    timeout(Duration::from_secs(1), async {
        loop {
            if !observer.recv().await.unwrap().after.connected {
                break;
            }
        }
    })
    .await
    .unwrap();
    let _ = release.send(());
    peer.await.unwrap();
}
