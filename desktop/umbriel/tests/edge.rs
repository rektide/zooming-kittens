mod common;
use common::*;
use serde_json::{Value, json};
use std::{path::Path, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
    sync::oneshot,
    time::timeout,
};
use zk_desktop::*;
use zk_umbriel::{
    Endpoint, Options, Source,
    edge::{AdmissionError, Transmission},
    subscribe,
};

async fn read_request(listener: &UnixListener) -> (UnixStream, Value) {
    let (socket, _) = listener.accept().await.unwrap();
    let mut reader = BufReader::new(socket);
    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();
    (reader.into_inner(), serde_json::from_str(&line).unwrap())
}
async fn reply(mut socket: UnixStream, value: Value) {
    let line = format!("{value}\n");
    socket.write_all(line.as_bytes()).await.unwrap();
    socket.shutdown().await.unwrap();
}
async fn family(socket: &mut UnixStream, name: &str, data: Value) {
    let line = format!("{}\n", json!({"event":name,"data":data}));
    socket.write_all(line.as_bytes()).await.unwrap();
}
async fn initial(listener: &UnixListener) -> UnixStream {
    let (mut socket, request) = read_request(listener).await;
    assert_eq!(
        request,
        json!({"cmd":"subscribe","events":["windows","workspaces","outputs"]})
    );
    family(
        &mut socket,
        "windows",
        json!([window("upper", "ws", 0), window("lower", "ws", 1)]),
    )
    .await;
    family(&mut socket, "workspaces", json!([workspace("ws", "OUT")])).await;
    family(&mut socket, "outputs", json!([output("OUT")])).await;
    socket
}
async fn probes(listener: &UnixListener) {
    for direction in ["height-down", "width-right"] {
        let (socket, request) = read_request(listener).await;
        assert_eq!(request["cmd"], "msg");
        assert_eq!(
            request["arg"],
            format!("window-modify-{direction} 0.001 --window \"zk-qualified-edge-check\0\"")
        );
        reply(
            socket,
            json!({"err":"unknown window: zk-qualified-edge-check\0"}),
        )
        .await;
    }
}
async fn client(path: &Path, options: Options) -> (Source, Observer) {
    let (source, mut observer) = subscribe(
        Endpoint::new(path),
        SourceId::new(Backend::Umbriel, "script"),
        options,
    )
    .await
    .unwrap();
    wait_ready(&mut observer, Epoch(1)).await;
    (source, observer)
}
async fn wait_ready(observer: &mut Observer, epoch: Epoch) {
    timeout(Duration::from_secs(2), async {
        while !(observer.current().scope.epoch == epoch && observer.current().is_ready()) {
            observer.recv().await.unwrap();
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn stale_target_is_rejected_once_without_focus_or_bare_fallback() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("stock.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let peer = tokio::spawn(async move {
        let _subscription = initial(&listener).await;
        probes(&listener).await;
        let (socket, command) = read_request(&listener).await;
        assert_eq!(
            command,
            json!({"cmd":"msg","arg":"window-modify-height-down -0.1 --window \"upper\""})
        );
        reply(socket, json!({"err":"unknown window: upper"})).await;
        assert!(
            timeout(Duration::from_millis(40), listener.accept())
                .await
                .is_err(),
            "no fallback command may be sent"
        );
    });
    let (source, observer) = client(&path, Options::default()).await;
    let actuator = source.edge_actuator().await.unwrap();
    assert_eq!(actuator.scope(), &observer.current().scope);
    let outcome = actuator.execute(request(&observer.current(), -0.1)).await;
    assert_eq!(
        outcome,
        Transmission::Rejected("unknown window: upper".into())
    );
    source.shutdown().await.unwrap();
    peer.await.unwrap();
}

#[tokio::test]
async fn unknown_grammar_or_handled_impossible_id_cannot_admit_live_sizing() {
    for response in [json!({"err":"unknown action"}), json!({"ok":null})] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("stock.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let peer = tokio::spawn(async move {
            let _subscription = initial(&listener).await;
            let (socket, command) = read_request(&listener).await;
            assert!(command["arg"].as_str().unwrap().contains("--window"));
            reply(socket, response).await;
            assert!(
                timeout(Duration::from_millis(40), listener.accept())
                    .await
                    .is_err()
            );
        });
        let (source, _) = client(&path, Options::default()).await;
        assert!(matches!(
            source.edge_actuator().await,
            Err(AdmissionError::UnsupportedReply(_))
        ));
        source.shutdown().await.unwrap();
        peer.await.unwrap();
    }
}

#[tokio::test]
async fn pending_command_keeps_observation_live_serializes_and_retains_receipt_on_loss() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("stock.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let (release, wait) = oneshot::channel();
    let peer = tokio::spawn(async move {
        let mut subscription = initial(&listener).await;
        probes(&listener).await;
        let (socket, _) = read_request(&listener).await;
        let mut a = window("upper", "ws", 0);
        let mut b = window("lower", "ws", 1);
        a["active"] = json!(false);
        b["active"] = json!(true);
        family(&mut subscription, "windows", json!([a, b])).await;
        wait.await.unwrap();
        reply(socket, json!({"ok":null})).await;
        assert!(
            timeout(Duration::from_millis(40), listener.accept())
                .await
                .is_err()
        );
    });
    let (source, mut observer) = client(&path, Options::default()).await;
    let actuator = source.edge_actuator().await.unwrap();
    let worker = actuator.clone();
    let command = request(&observer.current(), 0.1);
    let running = tokio::spawn(async move { worker.execute(command).await });
    timeout(Duration::from_secs(2), async {
        loop {
            let update = observer.recv().await.unwrap();
            if update
                .after
                .active_window
                .as_ref()
                .is_some_and(|w| w.native == NativeId::Text("lower".into()))
            {
                break;
            }
        }
    })
    .await
    .unwrap();
    assert!(!running.is_finished());
    let busy = actuator.execute(request(&observer.current(), 0.1)).await;
    assert!(matches!(busy, Transmission::Unsent(ref why) if why.contains("busy")));
    source.shutdown().await.unwrap();
    release.send(()).unwrap();
    assert_eq!(running.await.unwrap(), Transmission::Handled);
    assert!(matches!(
        actuator.execute(request(&observer.current(), 0.1)).await,
        Transmission::Unsent(_)
    ));
    peer.await.unwrap();
}

#[tokio::test]
async fn an_epoch_bound_verified_handle_never_crosses_native_reconnect() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("stock.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let (release, wait) = oneshot::channel();
    let (checked, checked_rx) = oneshot::channel();
    let peer = tokio::spawn(async move {
        let subscription = initial(&listener).await;
        probes(&listener).await;
        wait.await.unwrap();
        drop(subscription);
        let _new_subscription = initial(&listener).await;
        assert!(
            timeout(Duration::from_millis(40), listener.accept())
                .await
                .is_err(),
            "old handle cannot connect in the new epoch"
        );
        checked.send(()).unwrap();
        probes(&listener).await;
        let (socket, _) = read_request(&listener).await;
        reply(socket, json!({"ok":null})).await;
    });
    let mut options = Options::default();
    options.native.reconnect = umbriel_client::Reconnect::Retry {
        delay: Duration::from_millis(1),
        max_attempts: 1,
    };
    let (source, mut observer) = client(&path, options).await;
    let old = source.edge_actuator().await.unwrap();
    release.send(()).unwrap();
    wait_ready(&mut observer, Epoch(2)).await;
    assert!(matches!(
        old.execute(request(&observer.current(), 0.1)).await,
        Transmission::Unsent(_)
    ));
    checked_rx.await.unwrap();
    let new = source.edge_actuator().await.unwrap();
    assert_eq!(new.scope().epoch, Epoch(2));
    assert_eq!(
        new.execute(request(&observer.current(), 0.1)).await,
        Transmission::Handled
    );
    source.shutdown().await.unwrap();
    peer.await.unwrap();
}

#[tokio::test]
async fn no_reply_or_invalid_reply_is_possibly_sent_not_permission_to_replay() {
    for response in [None, Some(json!({"ok":false}))] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("stock.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let peer = tokio::spawn(async move {
            let _subscription = initial(&listener).await;
            probes(&listener).await;
            let (socket, _) = read_request(&listener).await;
            if let Some(response) = response {
                reply(socket, response).await;
            } else {
                drop(socket);
            }
            assert!(
                timeout(Duration::from_millis(40), listener.accept())
                    .await
                    .is_err()
            );
        });
        let (source, observer) = client(&path, Options::default()).await;
        let actuator = source.edge_actuator().await.unwrap();
        assert!(matches!(
            actuator.execute(request(&observer.current(), 0.1)).await,
            Transmission::PossiblySent(_)
        ));
        source.shutdown().await.unwrap();
        peer.await.unwrap();
    }
}
