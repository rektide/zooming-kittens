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
