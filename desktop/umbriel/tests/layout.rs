use std::time::Duration;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixListener,
};
use umbriel_protocol::model::*;
use zk_desktop::*;

fn column() -> LayoutColumn {
    let members = ["a", "b"]
        .into_iter()
        .map(|id| LayoutMember {
            id: id.into(),
            title_bar_height: 20,
            constraints: LayoutConstraints {
                min_width: 1,
                min_height: 70,
                max_width: 0,
                max_height: 520,
            },
            fullscreen: false,
            maximized: false,
            maximized_to_edges: false,
            floating: false,
        })
        .collect::<Vec<_>>();
    let geometry = members
        .iter()
        .enumerate()
        .map(|(row, m)| LayoutGeometry {
            id: m.id.clone(),
            target_tile: LayoutBox {
                x: 10,
                y: 20 + row as i32 * 341,
                width: 624,
                height: 329,
            },
            target_content: LayoutBox {
                x: 10,
                y: 40 + row as i32 * 341,
                width: 624,
                height: 309,
            },
            committed_content: LayoutBox {
                x: 0,
                y: 0,
                width: 624,
                height: 309,
            },
        })
        .collect();
    LayoutColumn {
        basis: LayoutBasis {
            instance: "instance".into(),
            workspace: "ws".into(),
            output: "HEADLESS-1".into(),
            column: 0,
            axis: SecondaryAxis::Vertical,
            members,
            weights: LayoutWeights {
                rows: vec![329.0, 329.0],
                top: 10.0,
                bottom: 20.0,
            },
            area: LayoutBox {
                x: 0,
                y: 0,
                width: 1280,
                height: 720,
            },
            edge_pad: 10,
            gap: 12,
            width_fraction: 0.5,
            primary_extent: 624,
            tabbed: false,
            supported: true,
        },
        geometry,
        cross_span: 688,
        row_budget: 658,
        top_gap: 10,
        bottom_gap: 20,
    }
}

#[test]
fn native_projection_converts_tile_bounds_and_preserves_restore_payload() {
    let column = column();
    let projected = zk_umbriel::layout::allocation(&column).unwrap();
    assert_eq!(projected.content_budget, 618);
    assert_eq!(projected.members[0].minimum, 50);
    assert_eq!(projected.members[0].maximum, Some(500));
    let scope = Scope {
        source: SourceId::new(Backend::Umbriel, "test"),
        epoch: Epoch(1),
    };
    let request = GroupAllocationRequest {
        group: GroupKey {
            workspace: WorkspaceKey {
                scope: scope.clone(),
                native: NativeId::Text("ws".into()),
            },
            position: NativeId::Number(0),
        },
        expected: projected.basis.clone(),
        restore_payload: None,
        targets: vec![400, 218],
        edge_gaps: projected.edge_gaps,
        expected_active: Some(WindowKey {
            scope,
            native: NativeId::Text("a".into()),
        }),
    };
    let wire = zk_umbriel::layout::compile(&request).unwrap();
    assert_eq!(wire.weights.rows, [420.0, 238.0]);
    assert_eq!((wire.weights.top, wire.weights.bottom), (10.0, 20.0));
    assert_eq!(wire.expected_active.as_deref(), Some("a"));
    let restore = zk_umbriel::layout::compile(&GroupAllocationRequest {
        restore_payload: Some(projected.restore_payload),
        targets: vec![309, 309],
        expected_active: None,
        ..request
    })
    .unwrap();
    assert_eq!(restore.weights, column.basis.weights);
    assert_eq!(restore.expected_active, None);
}

#[tokio::test]
async fn layout_source_is_coherent_and_ignores_other_family_focus() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("socket");
    let listener = UnixListener::bind(&path).unwrap();
    let (finish, wait) = tokio::sync::oneshot::channel();
    let peer = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let (read, mut write) = socket.into_split();
        let mut line = String::new();
        BufReader::new(read).read_line(&mut line).await.unwrap();
        let request: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(request["events"], serde_json::json!(["layout-state"]));
        let state = LayoutState {
            version: 1,
            instance: "instance".into(),
            active: Some("a".into()),
            columns: vec![column()],
            unsupported_workspaces: vec![],
        };
        let frame = serde_json::json!({"event":"layout-state","data":state}).to_string() + "\n";
        write.write_all(frame.as_bytes()).await.unwrap();
        write
            .write_all(b"{\"event\":\"windows\",\"data\":[]}\n")
            .await
            .unwrap();
        let mut changed = state;
        changed.active = Some("outside-column".into());
        write
            .write_all(
                (serde_json::json!({"event":"layout-state","data":changed}).to_string() + "\n")
                    .as_bytes(),
            )
            .await
            .unwrap();
        let _ = wait.await;
    });
    let (source, mut observer) = zk_umbriel::subscribe_layout(
        zk_umbriel::Endpoint::new(path),
        SourceId::new(Backend::Umbriel, "test"),
        zk_umbriel::Options::default(),
    )
    .await
    .unwrap();
    let capture = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let capture = observer.recv().await.unwrap().after.clone();
            if capture.is_ready()
                && capture
                    .active_window
                    .as_ref()
                    .is_some_and(|w| w.native == NativeId::Text("outside-column".into()))
            {
                break capture;
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(capture.windows.len(), 2);
    assert_eq!(capture.allocations.len(), 1);
    assert!(capture.control_status.as_ref().unwrap().is_current());
    assert!(!capture.windows_status.is_current());
    source.shutdown().await.unwrap();
    let _ = finish.send(());
    peer.await.unwrap();
}
