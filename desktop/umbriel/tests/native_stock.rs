//! Opt-in boundary acceptance on a harness-owned compositor, never the desktop.
use std::time::Duration;
use umbriel_protocol::{model::Window, request::Request};
use zk_desktop::*;
use zk_umbriel::{Endpoint, Options, edge::Transmission, subscribe};

async fn focused(endpoint: &Endpoint) -> Window {
    endpoint
        .query(
            Request::Raw {
                cmd: "focused-window",
                arg: None,
            },
            &umbriel_client::Options::default(),
        )
        .await
        .unwrap()
}

#[tokio::test]
#[ignore = "isolated native compositor only; run tests/native/check.sh"]
async fn real_stock_observation_probe_and_relative_edge_preserve_unrelated_focus() {
    assert_eq!(std::env::var("ZK_HEADLESS_BOUNDARY").as_deref(), Ok("1"));
    assert!(std::env::var_os("UMBRIEL_HARNESS_LIB").is_some());
    let endpoint = Endpoint::new(std::env::var_os("UMBRIEL_SOCKET").expect("isolated socket"));
    let anchor = focused(&endpoint).await;
    assert_eq!(anchor.title, "zk-boundary-anchor");
    let (source, mut observer) = subscribe(
        endpoint.clone(),
        SourceId::new(Backend::Umbriel, "headless-boundary"),
        Options::default(),
    )
    .await
    .unwrap();
    let capture = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let capture = observer.current();
            if capture.is_ready()
                && capture.groups.values().any(|g| {
                    g.members.len() == 2 && g.edge_eligibility == EdgeEligibility::Eligible
                })
            {
                break capture;
            }
            observer.recv().await.unwrap();
        }
    })
    .await
    .unwrap();
    let group = capture
        .groups
        .values()
        .find(|g| g.members.len() == 2 && g.edge_eligibility == EdgeEligibility::Eligible)
        .unwrap();
    assert_eq!(group.secondary_axis, Some(Axis::Vertical));
    assert_eq!(
        capture.windows[&group.members[0]].title.as_deref(),
        Some("zk-boundary-upper")
    );
    assert_eq!(
        capture.windows[&group.members[1]].title.as_deref(),
        Some("zk-boundary-lower")
    );
    let active = capture.active_window.clone();
    assert_eq!(
        active.as_ref().unwrap().native,
        NativeId::Text(anchor.id.as_str().to_owned())
    );
    let following = group.members[1].clone();
    let before_position = capture.windows[&following].geometry.target_position;
    let request = RelativeEdgeRequest {
        group: group.key.clone(),
        target: group.members[0].clone(),
        following: following.clone(),
        basis: capture.version,
        axis: Axis::Vertical,
        fraction: 0.01,
    };
    let actuator = source.edge_actuator().await.unwrap();
    assert_eq!(actuator.execute(request).await, Transmission::Handled);
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let capture = observer.current();
            assert_eq!(capture.active_window, active);
            if capture.windows[&following].geometry.target_position != before_position {
                break;
            }
            observer.recv().await.unwrap();
        }
    })
    .await
    .unwrap();
    let after = focused(&endpoint).await;
    assert_eq!(after.id, anchor.id);
    assert_eq!(
        (after.x, after.y, after.w, after.h),
        (anchor.x, anchor.y, anchor.w, anchor.h)
    );
    source.shutdown().await.unwrap();
}
