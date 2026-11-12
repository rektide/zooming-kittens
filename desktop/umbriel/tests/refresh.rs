mod common;
use common::*;
use serde_json::json;
use std::time::Duration;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixListener,
};
use zk_desktop::*;

#[tokio::test]
async fn refresh_installs_through_tracker_in_order_without_stalling_source_or_surviving_owner_shutdown()
 {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sock");
    let listener = UnixListener::bind(&path).unwrap();
    let (owner, mut observer) = zk_umbriel::subscribe(
        zk_umbriel::Endpoint::new(path),
        SourceId::new(Backend::Umbriel, "refresh"),
        zk_umbriel::Options::default(),
    )
    .await
    .unwrap();
    let (socket, _) = listener.accept().await.unwrap();
    let mut socket = BufReader::new(socket);
    let mut line = String::new();
    socket.read_line(&mut line).await.unwrap();
    for (family, data) in [
        (
            "windows",
            json!([window("upper", "ws", 0), window("lower", "ws", 1)]),
        ),
        ("workspaces", json!([workspace("ws", "OUT")])),
        ("outputs", json!([output("OUT")])),
    ] {
        socket
            .get_mut()
            .write_all(format!("{}\n", json!({"event":family,"data":data})).as_bytes())
            .await
            .unwrap();
    }
    while !observer.current().is_ready() {
        observer.recv().await.unwrap();
    }
    let scope = observer.current().scope.clone();
    let refresh = owner.windows_refresh().unwrap();
    let handle = refresh.clone();
    let request_scope = scope.clone();
    let pending =
        tokio::spawn(async move { handle.refresh(&request_scope, Duration::from_secs(1)).await });
    line.clear();
    socket.read_line(&mut line).await.unwrap();
    assert!(line.contains("windows"));
    assert!(
        refresh
            .refresh(&scope, Duration::from_secs(1))
            .await
            .unwrap_err()
            .contains("busy")
    );
    let mut upper = window("upper", "ws", 0);
    upper["title"] = json!("refresh result");
    let reply = json!({"ok":[upper,window("lower","ws",1)]});
    socket
        .get_mut()
        .write_all(format!("{reply}\n").as_bytes())
        .await
        .unwrap();
    let capture = pending.await.unwrap().unwrap();
    assert!(
        capture
            .windows
            .values()
            .any(|w| w.title.as_deref() == Some("refresh result"))
    );
    assert_eq!(capture.windows_status.stamp.unwrap().position, 4);
    assert_eq!(capture.workspaces_status.stamp.unwrap().position, 2);
    owner.shutdown().await.unwrap();
    assert!(
        refresh
            .refresh(&scope, Duration::from_millis(10))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn malformed_refresh_is_not_a_successful_freshness_signal() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sock");
    let listener = UnixListener::bind(&path).unwrap();
    let (owner, mut observer) = zk_umbriel::subscribe(
        zk_umbriel::Endpoint::new(path),
        SourceId::new(Backend::Umbriel, "refresh"),
        zk_umbriel::Options::default(),
    )
    .await
    .unwrap();
    let (socket, _) = listener.accept().await.unwrap();
    let mut socket = BufReader::new(socket);
    let mut line = String::new();
    socket.read_line(&mut line).await.unwrap();
    socket
        .get_mut()
        .write_all(
            format!(
                "{}\n{}\n{}\n",
                json!({"event":"windows","data":[]}),
                json!({"event":"workspaces","data":[]}),
                json!({"event":"outputs","data":[]})
            )
            .as_bytes(),
        )
        .await
        .unwrap();
    while !observer.current().is_ready() {
        observer.recv().await.unwrap();
    }
    let scope = observer.current().scope.clone();
    let refresh = owner.windows_refresh().unwrap();
    let pending =
        tokio::spawn(async move { refresh.refresh(&scope, Duration::from_secs(1)).await });
    line.clear();
    socket.read_line(&mut line).await.unwrap();
    socket
        .get_mut()
        .write_all(b"{\"ok\":[{\"bad\":true}]}\n")
        .await
        .unwrap();
    assert!(pending.await.unwrap().is_err());
    loop {
        if !observer.current().windows_status.is_current() {
            break;
        }
        observer.recv().await.unwrap();
    }
    assert!(
        observer
            .current()
            .outputs_status
            .as_ref()
            .unwrap()
            .is_current()
    );
    owner.shutdown().await.unwrap();
}
