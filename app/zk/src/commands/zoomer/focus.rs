//! App-owned semantic focus projection. Loss stops control, not a synthetic blur.
use std::io;
use tokio::sync::mpsc;
use zk_desktop::{
    Backend, Continuity, DesktopCapture, DesktopUpdate, Epoch, Health, LossReason, NativeId,
    Observer, Scope, WindowKey,
};
use zk_zoomer::{FocusEvent, FocusTarget};

pub(super) const QUEUE_CAPACITY: usize = 32;

#[derive(Default)]
struct FocusProjection {
    scope: Option<Scope>,
    focused: Option<(WindowKey, FocusTarget)>,
}

impl FocusProjection {
    fn apply(&mut self, capture: &DesktopCapture, app_id: &str) -> io::Result<Vec<FocusEvent>> {
        if !capture.connected
            || matches!(capture.windows_status.health, Health::Stale(_))
            || self
                .scope
                .as_ref()
                .is_some_and(|scope| scope != &capture.scope)
        {
            return Err(io::Error::other(
                "focus source lost freshness; stopping zoomer",
            ));
        }
        if !capture.windows_status.is_current() {
            // Initial acquisition is unknown, not known-empty/no-focus.
            return Ok(Vec::new());
        }
        // Umbriel starts at an unseen epoch 0, before its first native epoch.
        // Bind only a usable windows census, never that acquisition placeholder.
        self.scope.get_or_insert_with(|| capture.scope.clone());
        let next = capture
            .active_window
            .as_ref()
            .and_then(|key| capture.windows.get(key))
            .filter(|window| window.app_id.as_deref() == Some(app_id));
        if self.focused.as_ref().map(|(key, _)| key) == next.map(|window| &window.key) {
            return Ok(Vec::new());
        }
        let mut events = Vec::with_capacity(2);
        if let Some((_, window)) = self.focused.take() {
            events.push(FocusEvent::Blur { window });
        }
        if let Some(window) = next {
            let target = FocusTarget {
                id: match &window.key.native {
                    NativeId::Number(id) => id.to_string(),
                    NativeId::Text(id) => id.clone(),
                },
                pid: window.pid,
            };
            self.focused = Some((window.key.clone(), target.clone()));
            events.push(FocusEvent::Focus { window: target });
        }
        Ok(events)
    }
}

fn deliver(sender: &mpsc::Sender<Vec<FocusEvent>>, events: Vec<FocusEvent>) -> io::Result<()> {
    if !events.is_empty() {
        sender.try_send(events).map_err(|error| {
            io::Error::other(format!("focus delivery failed; stopping zoomer: {error}"))
        })?;
    }
    Ok(())
}

pub(super) async fn feed(
    mut observer: Observer,
    app_id: &str,
    sender: mpsc::Sender<Vec<FocusEvent>>,
) -> io::Result<()> {
    let mut projection = FocusProjection::default();
    let initial = observer.current();
    let mut version = initial.version;
    let mut initial_epoch_seen = false;
    let mut pending = None;
    // Check queued continuity evidence BEFORE acting on current(). A healthy
    // latest capture must not hide a loss/reconnect that happened during setup.
    // Native semantic capacity is 64; bound even this synchronous startup pass.
    for index in 0..=64 {
        let Some(update) = observer.try_recv().map_err(io::Error::other)? else {
            break;
        };
        if index == 64 {
            return Err(io::Error::other(
                "focus startup delivery exceeded bounded history",
            ));
        }
        check_continuity(&update, &mut initial_epoch_seen)?;
        if update.after.version > initial.version {
            pending = Some(update);
            break;
        }
    }
    deliver(&sender, projection.apply(&initial, app_id)?)?;
    loop {
        // Lag/closure is terminal: do not replay retained stale focus as fresh.
        let update = if let Some(update) = pending.take() {
            update
        } else {
            let update = observer.recv().await.map_err(io::Error::other)?;
            check_continuity(&update, &mut initial_epoch_seen)?;
            update
        };
        // current() may already include updates queued before feed started.
        if update.after.scope == initial.scope && update.after.version <= version {
            continue;
        }
        deliver(&sender, projection.apply(&update.after, app_id)?)?;
        version = update.after.version;
    }
}

fn check_continuity(update: &DesktopUpdate, initial_epoch_seen: &mut bool) -> io::Result<()> {
    if let Continuity::Discontinuous(reason) = &update.continuity {
        let before = &update.before;
        let after = &update.after;
        let first_umbriel_epoch = !*initial_epoch_seen
            && *reason == LossReason::EpochChanged
            && before.scope.source.backend == Backend::Umbriel
            && before.scope.source == after.scope.source
            && before.scope.epoch == Epoch(0)
            && before.connected
            && after.connected
            && before.windows_status == Default::default()
            && before.windows.is_empty()
            && before.active_window.is_none()
            && after.scope.epoch != Epoch(0);
        if !first_umbriel_epoch {
            return Err(io::Error::other(format!(
                "focus continuity lost: {reason:?}"
            )));
        }
        *initial_epoch_seen = true;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use zk_desktop::*;

    fn capture(active: Option<u64>) -> DesktopCapture {
        let scope = Scope {
            source: SourceId::new(Backend::Niri, "fixture"),
            epoch: Epoch(1),
        };
        let mut capture = DesktopCapture::empty(scope.clone(), Capabilities::READ_ONLY);
        capture.windows_status = FamilyStatus {
            stamp: Some(FamilyStamp {
                epoch: scope.epoch,
                position: 1,
            }),
            health: Health::Current,
        };
        for id in 1..=3 {
            let key = WindowKey {
                scope: scope.clone(),
                native: NativeId::Number(id),
            };
            capture.windows.insert(
                key.clone(),
                WindowView {
                    key: key.clone(),
                    workspace: None,
                    group: None,
                    output: None,
                    presentation: Default::default(),
                    placement: Placement::Tiled,
                    title: None,
                    app_id: Some(if id == 3 { "browser" } else { "kitty" }.into()),
                    pid: Some(id as i32 * 100),
                    active: active == Some(id),
                    geometry: Geometry::default(),
                },
            );
            if active == Some(id) {
                capture.active_window = Some(key);
            }
        }
        capture
    }

    fn target(id: u64) -> FocusTarget {
        FocusTarget {
            id: id.to_string(),
            pid: Some(id as i32 * 100),
        }
    }

    fn umbriel(active: Option<u64>) -> DesktopCapture {
        let mut capture = capture(active);
        capture.scope.source.backend = Backend::Umbriel;
        capture.windows = std::mem::take(&mut capture.windows)
            .into_values()
            .map(|mut window| {
                window.key.scope = capture.scope.clone();
                let NativeId::Number(id) = window.key.native else {
                    unreachable!()
                };
                window.key.native = NativeId::Text(format!("wayland:0x{id:x}"));
                (window.key.clone(), window)
            })
            .collect();
        capture.active_window = capture
            .windows
            .values()
            .find(|w| w.active)
            .map(|w| w.key.clone());
        capture
    }

    fn acquiring() -> DesktopCapture {
        DesktopCapture::empty(
            Scope {
                epoch: Epoch(0),
                ..umbriel(None).scope
            },
            Capabilities::READ_ONLY,
        )
    }

    fn epoch_start(before: Arc<DesktopCapture>) -> Arc<DesktopUpdate> {
        let mut after = DesktopCapture::empty(
            Scope {
                epoch: Epoch(1),
                ..before.scope.clone()
            },
            Capabilities::READ_ONLY,
        );
        after.version = ObservationVersion(1);
        Arc::new(DesktopUpdate::new(
            before,
            Arc::new(after),
            Continuity::Discontinuous(LossReason::EpochChanged),
        ))
    }

    #[test]
    fn umbriel_text_ids_preserve_pid_and_original_blur_target() {
        let mut projection = FocusProjection::default();
        assert!(projection.apply(&acquiring(), "kitty").unwrap().is_empty());
        assert!(
            projection.scope.is_none(),
            "unseen epoch zero must not bind"
        );
        let first = FocusTarget {
            id: "wayland:0x1".into(),
            pid: Some(100),
        };
        let second = FocusTarget {
            id: "wayland:0x2".into(),
            pid: Some(200),
        };
        assert_eq!(
            projection.apply(&umbriel(Some(1)), "kitty").unwrap(),
            vec![FocusEvent::Focus {
                window: first.clone()
            }]
        );
        assert_eq!(
            projection.apply(&umbriel(Some(2)), "kitty").unwrap(),
            vec![
                FocusEvent::Blur { window: first },
                FocusEvent::Focus {
                    window: second.clone()
                }
            ]
        );
        assert_eq!(
            projection.apply(&umbriel(None), "kitty").unwrap(),
            vec![FocusEvent::Blur { window: second }]
        );
    }

    #[tokio::test]
    async fn umbriel_initial_epoch_acquires_live_or_already_queued() {
        for queued in [false, true] {
            let initial = Arc::new(acquiring());
            let (publisher, observer) = Publisher::new(initial.clone(), 8);
            let (sender, mut receiver) = mpsc::channel(QUEUE_CAPACITY);
            let epoch = epoch_start(initial);
            let mut ready = umbriel(Some(1));
            ready.version = ObservationVersion(2);
            let ready = Arc::new(ready);
            let publish = || {
                publisher.publish((*epoch).clone());
                publisher.publish(DesktopUpdate::new(
                    epoch.after.clone(),
                    ready.clone(),
                    Continuity::Continuous,
                ));
            };
            if queued {
                publish();
            }
            let task = tokio::spawn(feed(observer, "kitty", sender));
            if !queued {
                tokio::task::yield_now().await;
                publish();
            }
            assert_eq!(
                tokio::time::timeout(std::time::Duration::from_secs(1), receiver.recv())
                    .await
                    .unwrap()
                    .unwrap(),
                vec![FocusEvent::Focus {
                    window: FocusTarget {
                        id: "wayland:0x1".into(),
                        pid: Some(100)
                    }
                }]
            );
            let mut lost = (*ready).clone();
            lost.connected = false;
            lost.version = ObservationVersion(3);
            publisher.publish(DesktopUpdate::new(
                ready,
                Arc::new(lost),
                Continuity::Discontinuous(LossReason::Disconnected),
            ));
            assert!(task.await.unwrap().is_err());
            assert!(
                receiver.recv().await.is_none(),
                "loss must not synthesize blur"
            );
        }
    }

    #[tokio::test]
    async fn queued_loss_reconnect_never_focuses_a_healthy_latest_capture() {
        let initial = Arc::new(acquiring());
        let (publisher, observer) = Publisher::new(initial.clone(), 8);
        let epoch = epoch_start(initial);
        publisher.publish((*epoch).clone());
        let mut lost = (*epoch.after).clone();
        lost.connected = false;
        lost.version = ObservationVersion(2);
        let lost = Arc::new(lost);
        publisher.publish(DesktopUpdate::new(
            epoch.after.clone(),
            lost.clone(),
            Continuity::Discontinuous(LossReason::Disconnected),
        ));
        let mut latest = umbriel(Some(1));
        latest.version = ObservationVersion(3);
        publisher.publish(DesktopUpdate::new(
            lost,
            Arc::new(latest),
            Continuity::Discontinuous(LossReason::EpochChanged),
        ));
        let (sender, mut receiver) = mpsc::channel(QUEUE_CAPACITY);
        assert!(feed(observer, "kitty", sender).await.is_err());
        assert!(receiver.recv().await.is_none());
    }

    #[test]
    fn only_one_pristine_umbriel_acquisition_marker_is_permitted() {
        let initial = Arc::new(acquiring());
        let epoch = epoch_start(initial);
        let mut seen = false;
        check_continuity(&epoch, &mut seen).unwrap();
        assert!(check_continuity(&epoch, &mut seen).is_err());
        for reason in [
            LossReason::Disconnected,
            LossReason::ObserverLag,
            LossReason::InvalidSnapshot,
        ] {
            let mut update = (*epoch).clone();
            update.continuity = Continuity::Discontinuous(reason);
            assert!(check_continuity(&update, &mut false).is_err());
        }
        let mut update = (*epoch).clone();
        let mut before = (*update.before).clone();
        before.scope.epoch = Epoch(1);
        update.before = Arc::new(before);
        assert!(check_continuity(&update, &mut false).is_err());
    }

    #[test]
    fn a_to_b_blurs_original_before_focus() {
        let mut reducer = FocusProjection::default();
        assert_eq!(
            reducer.apply(&capture(Some(1)), "kitty").unwrap(),
            vec![FocusEvent::Focus { window: target(1) }]
        );
        assert_eq!(
            reducer.apply(&capture(Some(2)), "kitty").unwrap(),
            vec![
                FocusEvent::Blur { window: target(1) },
                FocusEvent::Focus { window: target(2) },
            ]
        );
    }

    #[test]
    fn filter_boundary_and_return_preserve_blur_target() {
        let mut reducer = FocusProjection::default();
        reducer.apply(&capture(Some(1)), "kitty").unwrap();
        assert_eq!(
            reducer.apply(&capture(Some(3)), "kitty").unwrap(),
            vec![FocusEvent::Blur { window: target(1) }]
        );
        assert_eq!(reducer.apply(&capture(Some(3)), "kitty").unwrap(), vec![]);
        assert_eq!(
            reducer.apply(&capture(Some(2)), "kitty").unwrap(),
            vec![FocusEvent::Focus { window: target(2) }]
        );
    }

    #[test]
    fn explicit_none_and_deleted_focused_window_blur_once() {
        for deleted in [false, true] {
            let mut reducer = FocusProjection::default();
            let focused = capture(Some(1));
            reducer.apply(&focused, "kitty").unwrap();
            let mut after = capture(None);
            if deleted {
                // Also tolerate a dangling active key in a diagnostic capture.
                after.active_window = focused.active_window.clone();
                after
                    .windows
                    .remove(focused.active_window.as_ref().unwrap());
            }
            assert_eq!(
                reducer.apply(&after, "kitty").unwrap(),
                vec![FocusEvent::Blur { window: target(1) }]
            );
            assert!(reducer.apply(&after, "kitty").unwrap().is_empty());
        }
    }

    #[test]
    fn metadata_and_geometry_do_not_repeat_focus_or_rewrite_blur_target() {
        let mut reducer = FocusProjection::default();
        let mut current = capture(Some(1));
        reducer.apply(&current, "kitty").unwrap();
        let key = current.active_window.clone().unwrap();
        let window = current.windows.get_mut(&key).unwrap();
        window.title = Some("changed".into());
        window.pid = Some(999);
        window.geometry.content_size = Some(Size {
            width: 100.,
            height: 200.,
        });
        assert!(reducer.apply(&current, "kitty").unwrap().is_empty());
        assert_eq!(
            reducer.apply(&capture(None), "kitty").unwrap(),
            vec![FocusEvent::Blur { window: target(1) }]
        );
    }

    #[test]
    fn unknown_is_not_none_and_loss_never_invents_blur() {
        let mut reducer = FocusProjection::default();
        let mut current = capture(Some(1));
        current.windows_status = FamilyStatus::default();
        assert!(reducer.apply(&current, "kitty").unwrap().is_empty());
        current.windows_status.health = Health::Current;
        assert_eq!(
            reducer.apply(&current, "kitty").unwrap(),
            vec![FocusEvent::Focus { window: target(1) }]
        );
        for mode in 0..3 {
            let mut lost = current.clone();
            match mode {
                0 => lost.connected = false,
                1 => lost.windows_status.health = Health::Stale(LossReason::Disconnected),
                _ => lost.scope.epoch = Epoch(2),
            }
            assert!(reducer.apply(&lost, "kitty").is_err());
            assert_eq!(reducer.focused.as_ref().unwrap().1, target(1));
        }
    }

    #[tokio::test]
    async fn bounded_delivery_reports_overflow_and_closed_consumer() {
        let (sender, mut receiver) = mpsc::channel(1);
        let events = vec![
            FocusEvent::Blur { window: target(1) },
            FocusEvent::Focus { window: target(2) },
        ];
        deliver(&sender, events.clone()).unwrap();
        assert!(deliver(&sender, events.clone()).is_err());
        assert_eq!(receiver.recv().await.unwrap(), events);
        drop(receiver);
        assert!(deliver(&sender, events).is_err());
    }

    #[tokio::test]
    async fn observer_lag_and_closure_are_explicit_errors() {
        let initial = Arc::new(capture(None));
        let (publisher, observer) = Publisher::new(initial.clone(), 1);
        for version in 1..=4 {
            let mut after = capture(Some(1));
            after.version = ObservationVersion(version);
            publisher.publish(DesktopUpdate::new(
                initial.clone(),
                Arc::new(after),
                Continuity::Continuous,
            ));
        }
        let (sender, _receiver) = mpsc::channel(QUEUE_CAPACITY);
        assert!(
            feed(observer, "kitty", sender)
                .await
                .unwrap_err()
                .to_string()
                .contains("Lagged")
        );
        let (publisher, observer) = Publisher::new(initial, 1);
        drop(publisher);
        let (sender, _receiver) = mpsc::channel(QUEUE_CAPACITY);
        assert!(
            feed(observer, "kitty", sender)
                .await
                .unwrap_err()
                .to_string()
                .contains("Closed")
        );
    }

    #[tokio::test]
    async fn startup_current_does_not_replay_older_queued_focus() {
        let initial = Arc::new(capture(None));
        let (publisher, observer) = Publisher::new(initial.clone(), 8);
        let mut before = initial;
        for (version, id) in [(1, 1), (2, 2)] {
            let mut after = capture(Some(id));
            after.version = ObservationVersion(version);
            let after = Arc::new(after);
            publisher.publish(DesktopUpdate::new(
                before,
                after.clone(),
                Continuity::Continuous,
            ));
            before = after;
        }
        let (sender, mut receiver) = mpsc::channel(QUEUE_CAPACITY);
        let task = tokio::spawn(feed(observer, "kitty", sender));
        assert_eq!(
            receiver.recv().await.unwrap(),
            vec![FocusEvent::Focus { window: target(2) }]
        );
        drop(publisher);
        assert!(task.await.unwrap().is_err());
        assert!(receiver.recv().await.is_none());
    }

    #[tokio::test]
    async fn waiting_output_does_not_pause_observation_and_loss_invents_no_effect() {
        let initial = Arc::new(capture(Some(1)));
        let (publisher, observer) = Publisher::new(initial.clone(), 8);
        let (sender, mut receiver) = mpsc::channel(QUEUE_CAPACITY);
        let task = tokio::spawn(feed(observer, "kitty", sender));
        assert_eq!(
            receiver.recv().await.unwrap(),
            vec![FocusEvent::Focus { window: target(1) }]
        );
        // Output is now busy handling focus A: it does not poll receiver again.
        let mut after = capture(Some(2));
        after.version = ObservationVersion(1);
        let after = Arc::new(after);
        publisher.publish(DesktopUpdate::new(
            initial,
            after.clone(),
            Continuity::Continuous,
        ));
        let mut lost = (*after).clone();
        lost.version = ObservationVersion(2);
        lost.connected = false;
        publisher.publish(DesktopUpdate::new(
            after,
            Arc::new(lost),
            Continuity::Discontinuous(LossReason::Disconnected),
        ));
        // Feed still reaches loss without waiting for output to finish.
        assert!(
            tokio::time::timeout(std::time::Duration::from_secs(1), task)
                .await
                .unwrap()
                .unwrap()
                .is_err()
        );
        assert_eq!(
            receiver.recv().await.unwrap(),
            vec![
                FocusEvent::Blur { window: target(1) },
                FocusEvent::Focus { window: target(2) },
            ]
        );
        assert!(receiver.recv().await.is_none());
    }
}
