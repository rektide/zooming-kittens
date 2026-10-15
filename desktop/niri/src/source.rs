use crate::{Limits, Reducer};
use niri_ipc::{Action, Event, Reply, Request, Response, SizeChange};
use std::{
    io,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    sync::{Semaphore, oneshot, watch},
    task::JoinHandle,
    time::timeout,
};
use zk_desktop::*;

#[derive(Debug, Clone)]
pub struct Options {
    pub initial_epoch: Epoch,
    pub frame_limit: usize,
    pub observer_capacity: usize,
    pub io_timeout: Duration,
    pub inventory: Limits,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            initial_epoch: Epoch(1),
            frame_limit: 8 * 1024 * 1024,
            observer_capacity: 64,
            io_timeout: Duration::from_secs(10),
            inventory: Limits::default(),
        }
    }
}

pub struct Source {
    stop: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
    actuator: HeightActuator,
}
impl Source {
    pub fn actuator(&self) -> HeightActuator {
        self.actuator.clone()
    }
    /// Waits for actual receive-task and socket cleanup, including an idle peer.
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

pub async fn subscribe_env(source: SourceId, options: Options) -> io::Result<(Source, Observer)> {
    let path = std::env::var_os("NIRI_SOCKET")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "NIRI_SOCKET is not set"))?;
    subscribe(path, source, options).await
}

/// No automatic reconnect. A fresh subscription must use a distinct source
/// instance name or a strictly newer `Options::initial_epoch` for the same name.
pub async fn subscribe(
    path: impl AsRef<Path>,
    source: SourceId,
    options: Options,
) -> io::Result<(Source, Observer)> {
    if source.backend != Backend::Niri
        || options.frame_limit == 0
        || options.observer_capacity == 0
        || options
            .observer_capacity
            .checked_next_power_of_two()
            .is_none()
        || options.inventory.windows == 0
        || options.inventory.workspaces == 0
        || options.io_timeout.is_zero()
        || tokio::time::Instant::now()
            .checked_add(options.io_timeout)
            .is_none()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid niri subscription options/source",
        ));
    }
    let path = path.as_ref().to_path_buf();
    let mut socket = timeout(options.io_timeout, async {
        let mut socket = BufReader::new(UnixStream::connect(&path).await?);
        socket.get_mut().write_all(b"\"EventStream\"\n").await?;
        let bytes = read_line(&mut socket, options.frame_limit).await?;
        let reply: Reply = serde_json::from_slice(&bytes).map_err(invalid_data)?;
        if !matches!(reply, Ok(Response::Handled)) {
            return Err(invalid_data(format!(
                "event subscription rejected: {reply:?}"
            )));
        }
        socket.get_mut().shutdown().await?;
        Ok::<_, io::Error>(socket)
    })
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "niri subscription deadline"))??;
    let mut reducer = Reducer::new(source, options.initial_epoch, options.inventory);
    let scope = reducer.capture().scope.clone();
    let (publisher, observer) = Publisher::new(reducer.capture(), options.observer_capacity);
    let current = publisher.watch();
    let (stop, mut stopped) = oneshot::channel();
    let actuator = HeightActuator {
        path,
        scope,
        current,
        options: options.clone(),
        serial: Arc::new(Semaphore::new(1)),
    };
    let task = tokio::spawn(async move {
        let mut turn = 0;
        let reason = loop {
            let result = tokio::select! {
                biased;
                _ = &mut stopped => break LossReason::Shutdown,
                line = read_line(&mut socket, options.frame_limit) => line,
            };
            let bytes = match result {
                Ok(bytes) => bytes,
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                    break LossReason::Disconnected;
                }
                Err(e) => break LossReason::Protocol(e.to_string()),
            };
            let event: Event = match serde_json::from_slice(&bytes) {
                Ok(event) => event,
                Err(e) => break LossReason::Protocol(e.to_string()),
            };
            match reducer.apply(event) {
                Ok(update) => publisher.publish(update),
                Err(e) => break LossReason::Protocol(e.to_string()),
            }
            turn += 1;
            if turn == 32 {
                turn = 0;
                tokio::task::yield_now().await;
            }
        };
        publisher.publish(reducer.loss(reason));
        // Owned Tokio stream is dropped here; no blocked worker survives shutdown.
    });
    Ok((
        Source {
            stop: Some(stop),
            task: Some(task),
            actuator,
        },
        observer,
    ))
}

/// Newline framing with a hard pre-allocation payload bound. Partial EOF is loss,
/// not a complete frame. Each call owns its buffer and is cancellation-safe for
/// source shutdown (cancellation closes the entire source, never resumes a line).
async fn read_line<R: AsyncBufRead + Unpin>(reader: &mut R, limit: usize) -> io::Result<Vec<u8>> {
    let mut line = Vec::new();
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "niri frame ended before newline",
            ));
        }
        let newline = available.iter().position(|b| *b == b'\n');
        let take = newline.map_or(available.len(), |i| i + 1);
        if line.len().checked_add(take).is_none_or(|n| n > limit) {
            return Err(invalid_data("niri frame limit exceeded"));
        }
        line.extend_from_slice(&available[..take]);
        reader.consume(take);
        if newline.is_some() {
            return Ok(line);
        }
    }
}
fn invalid_data(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

#[derive(Debug, Clone)]
pub struct HeightRequest {
    pub target: WindowKey,
    pub basis: ObservationVersion,
    /// Native fixed logical-pixel window-height request. This is not a promise
    /// about the resulting committed content or decorated tile geometry.
    pub height: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transmission {
    Unsent(String),
    Rejected(String),
    /// Native handling only, not geometry convergence or causal attribution.
    Handled,
    PossiblySent(String),
}

#[derive(Clone)]
pub struct HeightActuator {
    path: PathBuf,
    scope: Scope,
    current: watch::Receiver<Arc<DesktopCapture>>,
    options: Options,
    serial: Arc<Semaphore>,
}
impl HeightActuator {
    /// One command at a time, with no waiting queue and no retries. Callers must
    /// retain this future once admitted: cancelling it after transmission discards
    /// their receipt, NOT the possible native effect. The runner owns that ledger.
    pub async fn set_height(&self, request: HeightRequest) -> Transmission {
        let Ok(_permit) = self.serial.try_acquire() else {
            return Transmission::Unsent("actuator busy".into());
        };
        let id = match request.target.native {
            NativeId::Number(id) => id,
            _ => return Transmission::Unsent("not a niri numeric ID".into()),
        };
        let validate = || {
            let capture = self.current.borrow();
            self.current.has_changed().is_ok()
                && capture.is_ready()
                && capture.scope == self.scope
                && request.target.scope == self.scope
                && request.basis == capture.version
                && request.height > 0
                && capture
                    .windows
                    .get(&request.target)
                    .is_some_and(|w| w.placement == Placement::Tiled)
        };
        if !validate() {
            return Transmission::Unsent(
                "stale/unready basis, missing tiled target or invalid height".into(),
            );
        }
        let mut socket =
            match timeout(self.options.io_timeout, UnixStream::connect(&self.path)).await {
                Ok(Ok(socket)) => socket,
                Ok(Err(e)) => return Transmission::Unsent(e.to_string()),
                Err(_) => return Transmission::Unsent("connect deadline".into()),
            };
        if !validate() {
            return Transmission::Unsent("capture changed while connecting; replan".into());
        }
        let native = Request::Action(Action::SetWindowHeight {
            id: Some(id),
            change: SizeChange::SetFixed(request.height),
        });
        let mut bytes = serde_json::to_vec(&native).expect("native request serialization");
        bytes.push(b'\n');
        let mut sent = 0;
        let result = timeout(self.options.io_timeout, async {
            while sent < bytes.len() {
                let n = socket.write(&bytes[sent..]).await?;
                if n == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "niri command write returned zero",
                    ));
                }
                sent += n;
            }
            let mut reader = BufReader::new(socket);
            let reply = read_line(&mut reader, self.options.frame_limit).await?;
            serde_json::from_slice::<Reply>(&reply).map_err(invalid_data)
        })
        .await;
        match result {
            Ok(Ok(Ok(Response::Handled))) => Transmission::Handled,
            Ok(Ok(Err(message))) => Transmission::Rejected(message),
            Ok(Ok(Ok(other))) => Transmission::PossiblySent(format!("unexpected reply: {other:?}")),
            other => {
                let message = match other {
                    Ok(Err(e)) => e.to_string(),
                    Err(_) => "command deadline".into(),
                    _ => unreachable!(),
                };
                if sent == 0 {
                    Transmission::Unsent(message)
                } else {
                    Transmission::PossiblySent(message)
                }
            }
        }
    }
}
