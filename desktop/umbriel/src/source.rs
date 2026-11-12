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
    edge: Option<crate::edge::Factory>,
    refresh: Option<WindowsRefresh>,
}

/// Read-only handle bound to one semantic source owner, not a source keepalive.
/// Clones share a no-queue refresh permit. Dropping Source still shuts native I/O.
#[derive(Clone)]
pub struct WindowsRefresh {
    native: umbriel_client::Subscription,
    current: tokio::sync::watch::Receiver<std::sync::Arc<DesktopCapture>>,
    serial: std::sync::Arc<tokio::sync::Semaphore>,
}
impl WindowsRefresh {
    /// Timed one-off query through the native stream owner. Tracker installs its
    /// normalized reply in receive order. Return only after that position (or a
    /// later windows census) is visible; a reply alone never means installation.
    pub async fn refresh(
        &self,
        scope: &Scope,
        timeout: std::time::Duration,
    ) -> Result<std::sync::Arc<DesktopCapture>, String> {
        let _permit = self
            .serial
            .try_acquire()
            .map_err(|_| "windows refresh busy")?;
        tokio::time::timeout(timeout, async {
            if self.current.has_changed().is_err()
                || !self.current.borrow().is_ready()
                || self.current.borrow().scope != *scope
            {
                return Err("windows refresh source unavailable".into());
            }
            let reply = self
                .native
                .refresh_windows(scope.epoch.0, timeout)
                .await
                .map_err(|e| e.to_string())?;
            // Remote errors and malformed native payloads remain errors, not a
            // successful freshness signal. Tracker independently validates rows.
            reply
                .decode::<umbriel_protocol::model::Windows>()
                .map_err(|e| e.to_string())?;
            let position = reply.local_receive_position();
            let mut current = self.current.clone();
            loop {
                let capture = current.borrow_and_update().clone();
                if capture.scope != *scope || !capture.is_ready() {
                    return Err("windows refresh lost source authority".into());
                }
                if capture
                    .windows_status
                    .stamp
                    .is_some_and(|stamp| stamp.epoch == scope.epoch && stamp.position >= position)
                {
                    return Ok(capture);
                }
                current
                    .changed()
                    .await
                    .map_err(|_| "windows refresh source closed")?;
            }
        })
        .await
        .map_err(|_| "windows refresh deadline elapsed".to_string())?
    }
}
impl Source {
    pub fn windows_refresh(&self) -> Option<WindowsRefresh> {
        self.refresh.clone()
    }
    /// Explicit stock capability check using an impossible stale ID; never a
    /// focused resize. Reverify after native epoch changes. Observation itself
    /// remains read-only and never implicitly probes the command surface.
    pub async fn edge_actuator(
        &self,
    ) -> Result<crate::edge::EdgeActuator, crate::edge::AdmissionError> {
        self.edge
            .as_ref()
            .ok_or(crate::edge::AdmissionError::NotStock)?
            .verify()
            .await
    }
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
    subscribe_inner(endpoint, source, options, false).await
}

pub async fn subscribe_layout(
    endpoint: Endpoint,
    source: SourceId,
    options: Options,
) -> Result<(Source, Observer), umbriel_client::Error> {
    subscribe_inner(endpoint, source, options, true).await
}

enum Reduction {
    Legacy(Reducer),
    Layout(crate::layout::Reducer),
}
impl Reduction {
    fn capture(&self) -> std::sync::Arc<DesktopCapture> {
        match self {
            Self::Legacy(r) => r.capture(),
            Self::Layout(r) => r.capture(),
        }
    }
    fn loss(&mut self, reason: LossReason) -> DesktopUpdate {
        match self {
            Self::Legacy(r) => r.loss(reason),
            Self::Layout(r) => r.loss(reason),
        }
    }
    fn apply(
        &mut self,
        now: std::time::Duration,
        observation: umbriel_client::Observation,
    ) -> Result<Option<DesktopUpdate>, crate::ReduceError> {
        match self {
            Self::Legacy(r) => r.apply(now, observation),
            Self::Layout(r) => r.apply(now, observation),
        }
    }
}

async fn subscribe_inner(
    endpoint: Endpoint,
    source: SourceId,
    options: Options,
    layout: bool,
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
    let edge_options = options.native.io.clone();
    let (native, mut observations) = endpoint
        .subscribe(
            if layout {
                &[Family::LayoutState]
            } else {
                crate::reducer::STOCK_FAMILIES
            },
            options.native,
        )
        .await?;
    let mut reducer = if layout {
        Reduction::Layout(crate::layout::Reducer::new(source))
    } else {
        Reduction::Legacy(Reducer::new(source))
    };
    let (publisher, observer) = Publisher::new(reducer.capture(), options.observer_capacity);
    let edge = (!layout).then(|| crate::edge::Factory {
        endpoint,
        current: publisher.watch(),
        options: edge_options,
        serial: std::sync::Arc::new(tokio::sync::Semaphore::new(1)),
    });
    let refresh = (!layout).then(|| WindowsRefresh {
        native: native.clone(),
        current: publisher.watch(),
        serial: std::sync::Arc::new(tokio::sync::Semaphore::new(1)),
    });
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
            edge,
            refresh,
        },
        observer,
    ))
}
