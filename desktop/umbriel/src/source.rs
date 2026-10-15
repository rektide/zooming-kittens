use crate::Reducer;
use tokio::{sync::oneshot, task::JoinHandle, time::Instant};
use umbriel_client::{Endpoint, SubscriptionOptions};
use umbriel_protocol::envelope::Family;
use zk_desktop::*;

#[derive(Debug, Clone)]
pub struct Options {
    pub native: SubscriptionOptions,
    pub observer_capacity: usize,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            native: SubscriptionOptions::default(),
            observer_capacity: 64,
        }
    }
}
pub struct Source {
    stop: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
}
impl Source {
    pub async fn shutdown(mut self) -> Result<(), tokio::task::JoinError> {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        self.task.take().expect("source task").await
    }
}
impl Drop for Source {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}

/// The native source drains independently; this adapter owns one bounded native
/// observer and republishes semantic captures without blocking on consumers.
pub async fn subscribe(
    endpoint: Endpoint,
    source: SourceId,
    options: Options,
) -> Result<(Source, Observer), umbriel_client::Error> {
    if source.backend != Backend::Umbriel
        || options.observer_capacity == 0
        || options
            .observer_capacity
            .checked_next_power_of_two()
            .is_none()
    {
        return Err(umbriel_client::Error::InvalidOptions(
            "invalid desktop source/capacity",
        ));
    }
    let (native, mut observations) = endpoint
        .subscribe(&[Family::Windows, Family::Workspaces], options.native)
        .await?;
    let mut reducer = Reducer::new(source);
    let (publisher, observer) = Publisher::new(reducer.capture(), options.observer_capacity);
    let (stop, mut stopped) = oneshot::channel();
    let task = tokio::spawn(async move {
        let origin = Instant::now();
        let reason = loop {
            let item = tokio::select! {
                biased;
                _ = &mut stopped => break LossReason::Shutdown,
                item = observations.recv() => item,
            };
            match item {
                Ok(item) => match reducer.apply(origin.elapsed(), item) {
                    Ok(Some(update)) => publisher.publish(update),
                    Ok(None) => {}
                    Err(error) => break LossReason::Protocol(error.to_string()),
                },
                Err(umbriel_client::DeliveryEnd::Lagged { .. }) => break LossReason::ObserverLag,
                Err(umbriel_client::DeliveryEnd::Source(_)) => break LossReason::Disconnected,
                Err(umbriel_client::DeliveryEnd::DriverLost) => {
                    break LossReason::Protocol("native observation driver lost".into());
                }
            }
        };
        publisher.publish(reducer.loss(reason));
        native.shutdown().await;
    });
    Ok((
        Source {
            stop: Some(stop),
            task: Some(task),
        },
        observer,
    ))
}
