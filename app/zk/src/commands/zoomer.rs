use super::backend::Backend;
use crate::config::{Config, Verbosity};
use futures::StreamExt;
use std::{io, time::Duration};
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::ReceiverStream;
use zk_desktop::{Observer, SourceId};
use zk_zoomer::KittyResizer;

mod focus;

enum Source {
    #[cfg(feature = "niri")]
    Niri(zk_niri::Source),
    #[cfg(feature = "umbriel")]
    Umbriel(zk_umbriel::Source),
}

impl Source {
    async fn subscribe(backend: Backend) -> Result<(Self, Observer), Box<dyn std::error::Error>> {
        backend.require()?;
        match backend {
            #[cfg(feature = "niri")]
            Backend::Niri => {
                let (source, observer) = zk_niri::subscribe_env(
                    SourceId::new(zk_desktop::Backend::Niri, "zoomer"),
                    zk_niri::Options::default(),
                )
                .await?;
                Ok((Self::Niri(source), observer))
            }
            #[cfg(feature = "umbriel")]
            Backend::Umbriel => {
                let endpoint = zk_umbriel::Discovery::from_env().resolve()?;
                let (source, observer) = zk_umbriel::subscribe(
                    endpoint,
                    SourceId::new(zk_desktop::Backend::Umbriel, "zoomer"),
                    zk_umbriel::Options::default(),
                )
                .await?;
                Ok((Self::Umbriel(source), observer))
            }
            #[allow(unreachable_patterns)]
            _ => unreachable!("backend checked before discovery"),
        }
    }

    async fn shutdown(self) -> Result<(), tokio::task::JoinError> {
        match self {
            #[cfg(feature = "niri")]
            Self::Niri(source) => source.shutdown().await,
            #[cfg(feature = "umbriel")]
            Self::Umbriel(source) => source.shutdown().await,
        }
    }
}

/// Run focus tracking for a specific app with configurable font adjustments
pub async fn run_zoomer(
    backend: Backend,
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
            eprintln!("No zoom configuration set; font adjustments disabled");
        }
    }

    // Subscribe before starting output work, and retain the actual source owner
    // until every exit path has joined its receive task.
    let (source, observer) = Source::subscribe(backend).await?;
    let registry_config = config.to_registry_config();
    let kitty_registry = crate::kitty::KittyRegistry::new(registry_config);
    kitty_registry.start_reaper().await;

    let mut zoomer = KittyResizer::with_zoom_config(kitty_registry, config.zoom);

    // Each slot is one indivisible transition (at most blur + focus). Never
    // block observation behind kitty I/O or silently coalesce skipped focuses.
    let (sender, receiver) = mpsc::channel(focus::QUEUE_CAPACITY);
    let events = ReceiverStream::new(receiver).flat_map(futures::stream::iter);
    let (stop, stopped) = oneshot::channel();
    let result = async {
        let processing = zoomer.process_events_until(events, async {
            let _ = stopped.await;
        });
        tokio::pin!(processing);
        tokio::select! {
            biased;
            signal = tokio::signal::ctrl_c() => {
                signal?;
                // Explicit shutdown, not invented desktop blur. Stop intake,
                // settle the one in-flight command, reverse acknowledged effects.
                let _ = stop.send(());
                tokio::time::timeout(Duration::from_secs(15), &mut processing)
                    .await
                    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut,
                        "zoomer restore deadline elapsed; font state may need manual correction"))?
            }
            result = focus::feed(observer, &app_id, sender) => result.map_err(Into::into),
            result = &mut processing => result,
        }
    }
    .await;
    // Source loss still drops pending effects, without speculative restoration.
    let shutdown = tokio::time::timeout(Duration::from_secs(3), source.shutdown())
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "focus source shutdown deadline"))?;
    result.and(shutdown.map_err(Into::into))
}
