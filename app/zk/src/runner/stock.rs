//! Stock Feedback's running composition. Intake remains live during every owned
//! command, capability probe and ordered refresh; none is a queued native batch.
use super::{
    Executor,
    feedback::Control,
    native::{Request, Transmission},
};
use crate::{commands::longcat::LongcatCommand, config::LongcatConfig};
use futures::{FutureExt, future::BoxFuture};
use serde_json::{Value, json};
use std::{io, time::Duration};
use tokio::time::Instant;
use zk_desktop::{ObserveError, Observer, Scope};
use zk_longcat::{
    CommandOutcome,
    feedback::{CommandKey, FeedbackDecision, WaitReason},
};

type Refresh<'a> = dyn Fn(Scope, Duration) -> BoxFuture<'a, Result<(), String>> + Send + Sync + 'a;

pub(super) struct Services<'a> {
    pub verify: Box<dyn Fn() -> BoxFuture<'a, io::Result<Executor>> + Send + Sync + 'a>,
    pub refresh: Box<Refresh<'a>>,
}

impl<'a> Services<'a> {
    pub fn native(owner: &'a zk_umbriel::Source) -> Self {
        Self {
            verify: Box::new(move || {
                async move {
                    owner
                        .edge_actuator()
                        .await
                        .map(super::native::stock_executor)
                        .map_err(io::Error::other)
                }
                .boxed()
            }),
            refresh: Box::new(move |scope, timeout| {
                let refresh = owner.windows_refresh().expect("stock refresh owner");
                async move { refresh.refresh(&scope, timeout).await.map(|_| ()) }.boxed()
            }),
        }
    }
}

fn now(origin: Instant) -> u64 {
    origin.elapsed().as_millis() as u64
}

fn outcome(value: Transmission) -> CommandOutcome {
    match value {
        Transmission::Handled => CommandOutcome::Handled,
        Transmission::Unsent(_) => CommandOutcome::Unsent,
        Transmission::Rejected(message) => CommandOutcome::Rejected { message },
        Transmission::PossiblySent(message) => CommandOutcome::PossiblySent { message },
        Transmission::GroupHandled(_) => CommandOutcome::PossiblySent {
            message: "unexpected conditional receipt for stock edge".into(),
        },
    }
}

fn completions(
    control: &mut Control,
    sink: &mut impl FnMut(Value) -> io::Result<()>,
    time: u64,
) -> io::Result<FeedbackDecision> {
    loop {
        // Persist BEFORE removing the bounded mailbox. Output failure revokes
        // authority and follows the same pending-future drain path as all exits.
        if let Some(completed) = control.feedback.completed() {
            sink(json!({"type":"longcat-feedback-completed", "attempt":completed}))?;
            control.feedback.take_completed();
        }
        let decision = control
            .feedback
            .reconsider(time)
            .map_err(io::Error::other)?;
        if !matches!(
            decision,
            FeedbackDecision::Wait {
                reason: WaitReason::Completion,
                ..
            }
        ) {
            return Ok(decision);
        }
    }
}

/// Kept separate from Episode's niri/explicit-conditional runner. The services
/// boundary permits scripted peers without importing protocol types into policy.
pub(super) async fn drive(
    mut observer: Observer,
    services: Services<'_>,
    command: &LongcatCommand,
    config: &LongcatConfig,
    sink: &mut impl FnMut(Value) -> io::Result<()>,
    mut stop: BoxFuture<'static, io::Result<()>>,
) -> io::Result<()> {
    let origin = Instant::now();
    let mut control = Control::new(config, command.convergence_timeout_ms)?;
    let mut processed = observer.current();
    let mut pending: Option<(CommandKey, BoxFuture<'static, Transmission>)> = None;
    let mut refresh: Option<BoxFuture<'_, Result<(), String>>> = None;
    let mut probe: Option<(Scope, BoxFuture<'_, io::Result<Executor>>)> = None;
    let mut executor: Option<(Scope, Executor)> = None;
    let mut dirty = true;
    let mut due = None;
    let mut stopping_at = None;
    let mut emitted = 0;
    let mut closed = false;
    let mut failure = None;
    let result: io::Result<()> = async {
        loop {
            let idle = Instant::now() + Duration::from_secs(86_400);
            tokio::select! {
                biased;
                _ = tokio::time::sleep_until(stopping_at.unwrap_or(idle)), if stopping_at.is_some() => break,
                receipt = async { match &mut pending {
                    Some((_, future)) => future.await, None => std::future::pending().await,
                }} => {
                    let (key, _) = pending.take().expect("owned command");
                    let native_receipt = format!("{receipt:?}");
                    control.receipt(key, outcome(receipt))?;
                    sink(json!({"type":"longcat-edge-command", "key":key, "native_receipt":native_receipt}))?;
                    dirty = true;
                }
                result = async { match &mut refresh {
                    Some(future) => future.await, None => std::future::pending().await,
                }} => {
                    refresh = None;
                    sink(json!({"type":"longcat-windows-refresh", "result":result}))?;
                    // The source has already installed the reply. Drain its
                    // history and assess latest facts before the second timer.
                    dirty = true;
                }
                verified = async { match &mut probe {
                    Some((_, future)) => future.await, None => std::future::pending().await,
                }} => {
                    let (scope, _) = probe.take().expect("owned probe");
                    let latest = observer.current();
                    if control.usable(&latest) && latest.scope == scope {
                        executor = Some((scope, verified?));
                        sink(json!({"type":"longcat-edge-capability", "verified":true,
                            "source":format!("{:?}",latest.scope), "method":"impossible-ID down/right qualified rejection"}))?;
                    } else {
                        sink(json!({"type":"longcat-edge-capability", "verified":false,
                            "reason":"probe lost source epoch authority"}))?;
                    }
                    dirty = true;
                }
                signal = &mut stop, if !control.stopping => {
                    signal?;
                    control.restore(now(origin))?;
                    stopping_at = Some(Instant::now() + Duration::from_millis(command.shutdown_timeout_ms));
                    dirty = true;
                }
                _ = async { if dirty { tokio::task::yield_now().await; }
                    else { tokio::time::sleep_until(due.unwrap_or(idle)).await; }
                }, if dirty || due.is_some() => {
                    dirty = false;
                    due = None;
                    // Every queued update is assessed, including intervening
                    // focus/topology changes and losses hidden by healthy latest.
                    let mut caught_up = closed;
                    if !closed {
                        for _ in 0..8 {
                            match observer.try_recv() {
                                Ok(Some(update)) => {
                                    control.update(&update, now(origin))?;
                                    processed = update.after.clone();
                                }
                                Ok(None) => { caught_up = true; break; }
                                Err(error) => {
                                    control.lose(&processed.scope, &error.to_string(), now(origin))?;
                                    closed = error == ObserveError::Closed;
                                    // A local missed history cannot be repaired
                                    // by a healthy same-epoch current snapshot.
                                    if !closed { return Err(io::Error::other(error)); }
                                    failure = Some(io::Error::other(error));
                                    control.stopping = true;
                                    stopping_at.get_or_insert(Instant::now() + Duration::from_millis(command.shutdown_timeout_ms));
                                    caught_up = true;
                                    break;
                                }
                            }
                        }
                    }
                    let latest = observer.current();
                    if !caught_up || latest.scope != processed.scope || latest.version != processed.version {
                        dirty = true;
                        continue;
                    }
                    control.install(&processed, now(origin))?;
                    if !control.usable(&processed) { executor = None; }
                    let time = now(origin);
                    // Refresh is independent of the edge future; never classify
                    // absent geometry while its timed query is still outstanding.
                    if let Some(id) = control.timer(&processed, time, refresh.is_some())? {
                        let timeout = Duration::from_millis(command.convergence_timeout_ms.clamp(32, 250));
                        refresh = Some((services.refresh)(processed.scope.clone(), timeout));
                        sink(json!({"type":"longcat-windows-refresh-start", "attempt":id.attempt,"sequence":id.sequence}))?;
                    }
                    let decision = completions(&mut control, sink, time)?;
                    sink(control.diagnostics(&processed, &decision, command.dry_run))?;
                    if control.usable(&processed) { emitted += 1; }
                    if command.limit.is_some_and(|n| emitted >= n) && !control.stopping {
                        if command.dry_run { break; }
                        control.restore(time)?;
                        stopping_at = Some(Instant::now() + Duration::from_millis(command.shutdown_timeout_ms));
                        dirty = true;
                        continue;
                    }
                    if closed && pending.is_none() { break; }
                    if control.stopping && pending.is_none() && refresh.is_none()
                        && control.feedback.current().is_none()
                        && matches!(decision, FeedbackDecision::Wait { reason: WaitReason::Stable | WaitReason::Limited | WaitReason::NoEligibleGroup | WaitReason::FocusUnknown, .. }
                            | FeedbackDecision::Infeasible { .. }) { break; }
                    if !command.dry_run && control.usable(&processed) && pending.is_none()
                        && control.feedback.completed().is_none() {
                        if executor.as_ref().is_some_and(|(scope,_)| *scope != processed.scope) { executor = None; }
                        if executor.is_none() && probe.is_none() {
                            probe = Some((processed.scope.clone(), (services.verify)()));
                        }
                        if let (FeedbackDecision::Drive(goal), Some((_, execute))) = (&decision, &executor) {
                            // Publication may race diagnostics/calculation. One
                            // queued item blocks sending until history catch-up.
                            match observer.try_recv() {
                                Ok(None) => {}
                                Ok(Some(update)) => {
                                    control.update(&update, now(origin))?;
                                    processed = update.after.clone();
                                    dirty = true;
                                    continue;
                                }
                                Err(error) => return Err(io::Error::other(error)),
                            }
                            let fresh = observer.current();
                            if fresh.scope != processed.scope || fresh.version != processed.version {
                                dirty = true; continue;
                            }
                            if let Some(Request::Edge(edge)) = control.prepare(&fresh, goal, now(origin))? {
                                let key = control.feedback.current().and_then(|a|a.pending()).expect("admitted command").key;
                                sink(json!({"type":"longcat-edge-intent", "key":key,
                                    "intent":control.feedback.current().and_then(|a|a.pending()).map(|e|&e.intent),
                                    "target":super::bridge::native_id(&edge.request.target.native),
                                    "following":super::bridge::native_id(&edge.request.following.native),
                                    "axis":format!("{:?}",edge.request.axis), "fraction":edge.request.fraction}))?;
                                pending = Some((key, execute(Request::Edge(edge))));
                            } else if control.feedback.completed().is_some() || control.feedback.current().is_some_and(|a|a.stopping.is_some()) {
                                dirty = true;
                            }
                        }
                    }
                    due = control.next_deadline(refresh.is_some()).map(|ms| origin + Duration::from_millis(ms));
                }
                event = observer.recv(), if !closed => {
                    match event {
                        Ok(update) => {
                            control.update(&update, now(origin))?;
                            processed = update.after.clone();
                        }
                        Err(error) => {
                            control.lose(&processed.scope, &error.to_string(), now(origin))?;
                            if error != ObserveError::Closed { return Err(io::Error::other(error)); }
                            closed = true;
                            failure = Some(io::Error::other(error));
                            control.stopping = true;
                            stopping_at.get_or_insert(Instant::now() + Duration::from_millis(command.shutdown_timeout_ms));
                        }
                    }
                    dirty = true;
                }
            }
        }
        Ok(())
    }.await;
    // Every exit, including output failure, stops unsent work and drains the
    // already-owned future within the remaining shutdown budget. No replay.
    control.lose(&processed.scope, "runner lifecycle ended", now(origin))?;
    if let Some((key, future)) = pending {
        let deadline = stopping_at
            .unwrap_or_else(|| Instant::now() + Duration::from_millis(command.shutdown_timeout_ms));
        let receipt = match tokio::time::timeout_at(deadline, future).await {
            Ok(receipt) => outcome(receipt),
            Err(_) => CommandOutcome::PossiblySent {
                message: "runner drain deadline; no replay".into(),
            },
        };
        control.receipt(key, receipt.clone())?;
        let _ = sink(
            json!({"type":"longcat-edge-command", "key":key, "outcome":receipt,"draining":true}),
        );
    } else if let Some(key) = control
        .feedback
        .current()
        .and_then(|a| a.pending())
        .map(|e| e.key)
    {
        // Admission followed by diagnostic failure occurred before a future was
        // even constructed, so this is genuinely unsent rather than uncertain.
        control.receipt(key, CommandOutcome::Unsent)?;
    }
    let completed = completions(&mut control, sink, now(origin));
    let final_output = sink(
        json!({"type":"longcat-feedback-stopped", "counters":control.feedback.counters(),
        "preferences":control.feedback.preferences(), "pending":control.feedback.current()}),
    );
    result
        .and(failure.map_or(Ok(()), Err))
        .and(completed.map(|_| ()))
        .and(final_output)
}
