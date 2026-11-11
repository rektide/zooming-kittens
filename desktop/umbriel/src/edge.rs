//! Focus-free stock relative-edge boundary. Geometry eligibility is separate
//! from server grammar verification. No bare fallback, replay or convergence
//! promise is provided here; the application owns each admitted future/receipt.
use std::sync::Arc;
use tokio::sync::{Semaphore, watch};
use umbriel_client::{Endpoint, Error, Options};
use umbriel_protocol::action::{Action, ExtentDelta, WindowEdge};
use zk_desktop::*;

// Foreign toplevel identifiers are native C strings: this nonempty ID with an
// embedded NUL cannot equal any mapped identifier. Native splitActionString
// preserves the full token (covered by the real parser conformance test).
const CAPABILITY_TARGET: &str = "zk-qualified-edge-check\0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionError {
    NotStock,
    Unavailable,
    Busy,
    UnsupportedReply(String),
    Probe(String),
}
impl std::fmt::Display for AdmissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "qualified edge admission: {self:?}")
    }
}
impl std::error::Error for AdmissionError {}

pub(crate) struct Factory {
    pub endpoint: Endpoint,
    pub current: watch::Receiver<Arc<DesktopCapture>>,
    pub options: Options,
    pub serial: Arc<Semaphore>,
}
impl Factory {
    pub async fn verify(&self) -> Result<EdgeActuator, AdmissionError> {
        let _permit = self
            .serial
            .try_acquire()
            .map_err(|_| AdmissionError::Busy)?;
        if self.current.has_changed().is_err() || !self.current.borrow().is_ready() {
            return Err(AdmissionError::Unavailable);
        }
        let scope = self.current.borrow().scope.clone();
        for edge in [WindowEdge::Down, WindowEdge::Right] {
            let action = Action::window_modify_edge_for_window(
                edge,
                ExtentDelta::new(0.001).expect("valid probe delta"),
                CAPABILITY_TARGET,
            )
            .expect("nonempty probe ID");
            let result = self
                .endpoint
                .request_if(action.request(), &self.options, || self.live_scope(&scope))
                .await;
            match result.and_then(|reply| reply.decode::<()>()) {
                Err(Error::Remote(message))
                    if message == format!("unknown window: {CAPABILITY_TARGET}") => {}
                Err(Error::Remote(message)) => {
                    return Err(AdmissionError::UnsupportedReply(message));
                }
                Ok(()) => {
                    return Err(AdmissionError::UnsupportedReply(
                        "server handled an impossible window ID".into(),
                    ));
                }
                Err(error) => return Err(AdmissionError::Probe(error.to_string())),
            }
        }
        if !self.live_scope(&scope) {
            return Err(AdmissionError::Unavailable);
        }
        Ok(EdgeActuator {
            endpoint: self.endpoint.clone(),
            current: self.current.clone(),
            options: self.options.clone(),
            scope,
            serial: self.serial.clone(),
        })
    }
    fn live_scope(&self, scope: &Scope) -> bool {
        self.current.has_changed().is_ok() && {
            let capture = self.current.borrow();
            capture.is_ready() && &capture.scope == scope
        }
    }
}

/// Pure request compilation from the exact latest semantic capture. A command
/// must use target's cross-end and an actual next row; last/outer edges are refused.
pub fn prepare(
    capture: &DesktopCapture,
    request: &RelativeEdgeRequest,
) -> Result<Action<'static>, String> {
    if !capture.is_ready()
        || capture.scope.source.backend != Backend::Umbriel
        || request.group.workspace.scope != capture.scope
        || request.target.scope != capture.scope
        || request.following.scope != capture.scope
        || request.basis != capture.version
    {
        return Err("stale or unavailable stock edge basis".into());
    }
    let group = capture
        .groups
        .get(&request.group)
        .ok_or("unknown edge group")?;
    if group.membership != Evidence::Reported
        || group.edge_eligibility != EdgeEligibility::Eligible
        || group.secondary_axis != Some(request.axis)
        || !group
            .members
            .windows(2)
            .any(|pair| pair[0] == request.target && pair[1] == request.following)
    {
        return Err("edge must be an eligible internal secondary boundary".into());
    }
    let NativeId::Text(id) = &request.target.native else {
        return Err("not an Umbriel text window ID".into());
    };
    let delta = ExtentDelta::new(request.fraction).map_err(|e| e.to_string())?;
    let edge = match request.axis {
        Axis::Vertical => WindowEdge::Down,
        Axis::Horizontal => WindowEdge::Right,
    };
    Action::window_modify_edge_for_window(edge, delta, id).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transmission {
    Unsent(String),
    Rejected(String),
    /// Native handling, not target-position progress or committed convergence.
    Handled,
    PossiblySent(String),
}

/// Verified for one source epoch. A reconnect requires a fresh handle/probe;
/// retaining this handle cannot prolong source ownership or replay an old delta.
#[derive(Clone)]
pub struct EdgeActuator {
    endpoint: Endpoint,
    current: watch::Receiver<Arc<DesktopCapture>>,
    options: Options,
    scope: Scope,
    serial: Arc<Semaphore>,
}
impl EdgeActuator {
    pub fn scope(&self) -> &Scope {
        &self.scope
    }

    /// One owned command, no waiting queue or retry. Keep polling this future
    /// after admission even if observation changes; retain its lasting outcome.
    /// Local freshness is rechecked after connect, but is not server atomicity.
    pub async fn execute(&self, request: RelativeEdgeRequest) -> Transmission {
        let Ok(_permit) = self.serial.try_acquire() else {
            return Transmission::Unsent("edge actuator busy".into());
        };
        let action = match prepare(&self.current.borrow(), &request) {
            Ok(action) => action,
            Err(error) => return Transmission::Unsent(error),
        };
        let result = self
            .endpoint
            .request_if(action.request(), &self.options, || {
                self.current.has_changed().is_ok() && {
                    let capture = self.current.borrow();
                    capture.is_ready()
                        && capture.scope == self.scope
                        && capture.version == request.basis
                }
            })
            .await
            .and_then(|reply| reply.decode::<()>());
        match result {
            Ok(()) => Transmission::Handled,
            Err(Error::Remote(message)) => Transmission::Rejected(message),
            Err(
                error @ (Error::Precondition
                | Error::Connect(_)
                | Error::ConnectTimeout
                | Error::InvalidOptions(_)
                | Error::Admission(_)
                | Error::TimedOut {
                    possibly_sent: false,
                    ..
                }),
            ) => Transmission::Unsent(error.to_string()),
            Err(error) => Transmission::PossiblySent(error.to_string()),
        }
    }
}
