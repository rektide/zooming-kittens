use std::time::Duration;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixListener,
    sync::oneshot,
    time::timeout,
};
use zk_desktop::*;
use zk_umbriel::{Endpoint, Options, subscribe};

#[tokio::test]
async fn native_subscription_is_read_only_and_idle_shutdown_joins_underlying_socket() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("umbriel.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let (release, wait) = oneshot::channel();
    let peer = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut line = String::new();
        BufReader::new(&mut socket)
            .read_line(&mut line)
            .await
            .unwrap();
        assert!(line.contains("subscribe"));
        assert!(line.contains("windows"));
        assert!(line.contains("workspaces"));
        socket
            .write_all(
                b"{\"event\":\"windows\",\"data\":[]}\n{\"event\":\"workspaces\",\"data\":[]}\n",
            )
            .await
            .unwrap();
        wait.await.unwrap();
        assert!(socket.write_all(b"after close\n").await.is_err());
    });
    let (source, mut observer) = subscribe(
        Endpoint::new(path),
        SourceId::new(Backend::Umbriel, "script"),
        Options::default(),
    )
    .await
    .unwrap();
    timeout(Duration::from_secs(2), async {
        loop {
            if observer.recv().await.unwrap().after.is_ready() {
                break;
            }
        }
    })
    .await
    .unwrap();
    assert!(observer.current().groups.is_empty());
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
async fn eof_and_drop_are_observable_without_polling_native_socket() {
    for drop_owner in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("umbriel.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let (release, wait) = oneshot::channel();
        let peer = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut line = String::new();
            BufReader::new(&mut socket)
                .read_line(&mut line)
                .await
                .unwrap();
            socket.write_all(b"{\"event\":\"windows\",\"data\":[]}\n{\"event\":\"workspaces\",\"data\":[]}\n").await.unwrap();
            let _ = wait.await;
        });
        let (source, mut observer) = subscribe(
            Endpoint::new(path),
            SourceId::new(Backend::Umbriel, "script"),
            Options::default(),
        )
        .await
        .unwrap();
        timeout(Duration::from_secs(2), async {
            loop {
                if observer.recv().await.unwrap().after.is_ready() {
                    break;
                }
            }
        })
        .await
        .unwrap();
        let source = if drop_owner {
            drop(source);
            Some(release)
        } else {
            release.send(()).unwrap();
            None
        };
        timeout(Duration::from_secs(2), async {
            loop {
                if !observer.recv().await.unwrap().after.connected {
                    break;
                }
            }
        })
        .await
        .unwrap();
        assert!(!observer.current().is_ready());
        drop(source);
        peer.await.unwrap();
    }
}
