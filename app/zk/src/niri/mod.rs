pub mod registry;
pub mod types;

/// Bridge the legacy native focus adapter to the compositor-neutral zoom policy.
pub fn focus_events(
    events: impl futures::Stream<Item = types::NiriEvent>,
) -> impl futures::Stream<Item = zk_zoomer::FocusEvent> {
    use futures::StreamExt;
    events.filter_map(|event| {
        let event = match event {
            types::NiriEvent::Focus { window, .. } => Some(zk_zoomer::FocusEvent::Focus {
                window: zk_zoomer::FocusTarget {
                    id: window.id.to_string(),
                    pid: window.pid,
                },
            }),
            types::NiriEvent::Blur { window, .. } => Some(zk_zoomer::FocusEvent::Blur {
                window: zk_zoomer::FocusTarget {
                    id: window.id.to_string(),
                    pid: window.pid,
                },
            }),
            _ => None,
        };
        futures::future::ready(event)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{StreamExt, executor::block_on, stream};
    use types::{NiriEvent, WindowInfo};
    use zk_zoomer::{FocusEvent, FocusTarget};

    fn window(id: u64, pid: i32) -> WindowInfo {
        WindowInfo {
            id,
            pid: Some(pid),
            app_id: Some("kitty".into()),
            title: None,
        }
    }

    #[test]
    fn focus_bridge_preserves_original_blur_target() {
        let events = stream::iter([
            NiriEvent::Focus {
                window_id: 1,
                window: window(1, 100),
            },
            NiriEvent::Blur {
                window_id: 1,
                window: window(1, 100),
            },
            NiriEvent::Focus {
                window_id: 2,
                window: window(2, 200),
            },
        ]);
        let actual = block_on(focus_events(events).collect::<Vec<_>>());
        assert_eq!(
            actual,
            vec![
                FocusEvent::Focus {
                    window: FocusTarget {
                        id: "1".into(),
                        pid: Some(100)
                    }
                },
                FocusEvent::Blur {
                    window: FocusTarget {
                        id: "1".into(),
                        pid: Some(100)
                    }
                },
                FocusEvent::Focus {
                    window: FocusTarget {
                        id: "2".into(),
                        pid: Some(200)
                    }
                },
            ]
        );
    }

    #[test]
    fn focus_bridge_does_not_turn_lifecycle_events_into_focus() {
        let events = stream::iter([
            NiriEvent::Create {
                window_id: 1,
                window: window(1, 100),
            },
            NiriEvent::Destroy { window_id: 1 },
        ]);
        assert!(block_on(focus_events(events).collect::<Vec<_>>()).is_empty());
    }
}
