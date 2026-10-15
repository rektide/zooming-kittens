use crate::config::{Config, Verbosity};
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use zk_desktop::{Backend, SourceId};
use zk_zoomer::KittyResizer;

mod focus;

/// Run focus tracking for a specific app with configurable font adjustments
pub async fn run_zoomer(
    app_id: String,
    verbosity: Verbosity,
    config: Config,
) -> Result<(), Box<dyn std::error::Error>> {
    if verbosity.log_window_events() {
        eprintln!("Starting zoomer for app_id: {}", app_id);
        if let Some(zoom_type) = config.zoom.active_type() {
            match zoom_type {
                crate::config::ZoomType::Absolute => {
                    eprintln!(
                        "Zoom type: Absolute (target: {})",
                        config.zoom.value().unwrap()
                    );
                }
                crate::config::ZoomType::Additive => {
                    eprintln!(
                        "Zoom type: Additive (amount: {})",
                        config.zoom.value().unwrap()
                    );
                }
                crate::config::ZoomType::Multiplicative => {
                    eprintln!(
                        "Zoom type: Multiplicative (factor: {})",
                        config.zoom.value().unwrap()
                    );
                }
            }
            eprintln!("Step size: {}", config.zoom.step_size);
        } else {
            eprintln!("Warning: No zoom configuration set, using default additive +6");
        }
    }

    // Subscribe before starting output work, and retain the actual source owner
    // until every exit path has joined its receive task.
    let (source, observer) = zk_niri::subscribe_env(
        SourceId::new(Backend::Niri, "zoomer"),
        zk_niri::Options::default(),
    )
    .await?;
    let registry_config = config.to_registry_config();
    let kitty_registry = crate::kitty::KittyRegistry::new(registry_config);
    kitty_registry.start_reaper().await;

    let mut zoomer = KittyResizer::with_zoom_config(kitty_registry, config.zoom);

    // Each slot is one indivisible transition (at most blur + focus). Never
    // block observation behind kitty I/O or silently coalesce skipped focuses.
    let (sender, receiver) = mpsc::channel(focus::QUEUE_CAPACITY);
    let events = ReceiverStream::new(receiver).flat_map(futures::stream::iter);
    let result = tokio::select! {
        biased;
        signal = tokio::signal::ctrl_c() => signal.map_err(Into::into),
        result = focus::feed(observer, &app_id, sender) => result.map_err(Into::into),
        result = zoomer.process_events(events) => result,
    };
    // Cancellation/loss drops pending effects; neither is evidence of blur.
    // Commands already sent to kitty may have taken effect; no rollback claimed.
    let shutdown = source.shutdown().await;
    shutdown?;
    result
}
