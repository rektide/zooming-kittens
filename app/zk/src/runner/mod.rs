//! Bounded application composition, not a reusable controller host.
mod bridge;
mod control;
mod native;
mod output;

#[cfg(all(test, feature = "niri"))]
mod tests;
#[cfg(all(test, feature = "umbriel"))]
mod umbriel_tests;

use crate::{commands::longcat::LongcatCommand, config::LongcatConfig};
use control::{Control, Sequence};
use futures::{
    FutureExt,
    future::{BoxFuture, poll_fn},
};
use kubeless_work_schedule::{ReadyWork, ScheduleOptions, ScheduleRequest, schedule};
use native::{Request, Transmission};
use serde_json::Value;
use std::{io, sync::Arc, time::Duration};
use tokio::time::Instant;
use zk_desktop::{Continuity, ObserveError, Observer};
use zk_longcat::CommandOutcome;

type Executor = Arc<dyn Fn(Request) -> BoxFuture<'static, Transmission> + Send + Sync>;

pub async fn run(command: LongcatCommand, config: LongcatConfig) -> io::Result<()> {
    let (owner, observer, executor) = native::subscribe(&command).await?;
    // Native I/O never awaits stdout. A slow/broken diagnostic consumer ends the
    // app rather than accumulating an unbounded backlog or silently losing rows.
    let output = output::Output::stdout()?;
    let quiet = command.quiet;
    let mut sink = |record: Value| -> io::Result<()> {
        if quiet {
            return Ok(());
        }
        let mut bytes = serde_json::to_vec(&record).map_err(io::Error::other)?;
        if bytes.len() > 2 * 1024 * 1024 {
            return Err(bridge::invalid("diagnostic record exceeds 2 MiB limit"));
        }
        bytes.push(b'\n');
        output.rows.try_send(bytes).map_err(|e| {
            io::Error::other(format!("diagnostic delivery failed (bounded queue): {e}"))
        })
    };
    let result = drive(
        observer,
        executor,
        &command,
        &config,
        &mut sink,
        async { tokio::signal::ctrl_c().await }.boxed(),
    )
    .await;
    let shutdown = tokio::time::timeout(Duration::from_secs(3), owner.shutdown())
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "native source shutdown deadline"))
        .and_then(|result| result);
    let written = output.finish().await;
    result.and(shutdown).and(written)
}

async fn drive(
    mut observer: Observer,
    executor: Option<Executor>,
    command: &LongcatCommand,
    config: &LongcatConfig,
    sink: &mut impl FnMut(Value) -> io::Result<()>,
    mut stop: BoxFuture<'static, io::Result<()>>,
) -> io::Result<()> {
    let origin = Instant::now();
    let mut control = Control::new(config, command.dry_run, command.convergence_timeout_ms);
    let mut sequence: Option<Sequence> = None;
    let mut pending: Option<BoxFuture<'static, Transmission>> = None;
    let mut timer = schedule::<(), &'static str>(ScheduleOptions::default());
    timer
        .admit(ScheduleRequest {
            key: (),
            work: "startup",
            run_at: origin,
        })
        .expect("open schedule");
    let mut emitted = 0u64;
    let mut stopping_at = None;
    let mut failure = None;
    let mut observer_closed = false;
    let mut processed = observer.current();
    // One admitted event per turn; due work and native completion get priority
    // over further intake. This avoids Scheduler's greedy stream-drain issue.
    let result: io::Result<()> = async {
        loop {
            let deadline = stopping_at.unwrap_or_else(|| Instant::now() + Duration::from_secs(86_400));
            tokio::select! {
                biased;
                _ = tokio::time::sleep_until(deadline), if stopping_at.is_some() => {
                    break;
                }
                outcome = async {
                    match &mut pending { Some(future) => future.await, None => std::future::pending().await }
                } => {
                    pending = None;
                    let current = sequence.as_mut().expect("pending sequence");
                    let native_receipt = format!("{outcome:?}");
                    let mut recorded = false;
                    let outcome = match outcome {
                        Transmission::GroupHandled(receipt) => match control.accept_group(current, receipt) {
                            Ok(()) => { recorded = true; CommandOutcome::Handled },
                            Err(error) => CommandOutcome::PossiblySent { message: error.to_string() },
                        },
                        Transmission::Handled if current.group_submission => CommandOutcome::PossiblySent { message: "group handling reply lacks accepted native basis".into() },
                        Transmission::Handled => CommandOutcome::Handled,
                        Transmission::Rejected(message) => CommandOutcome::Rejected { message },
                        Transmission::PossiblySent(message) => CommandOutcome::PossiblySent { message },
                        Transmission::Unsent(message) => { drop(message); CommandOutcome::Unsent },
                    };
                    let proceed = outcome == CommandOutcome::Handled;
                    if !recorded { control.record(current, outcome)?; }
                    if current.group_submission {
                        sink(serde_json::json!({"type":"longcat-group-command", "plan_id":current.plan.id,
                            "targets":current.plan.targets, "native_receipt":native_receipt}))?;
                    } else {
                        sink(serde_json::json!({"type":"longcat-command", "plan_id":current.plan.id,
                            "index":current.next, "target":current.plan.targets[current.next], "native_receipt":native_receipt}))?;
                    }
                    current.next += 1;
                    if current.group_submission || !proceed || current.next == current.plan.targets.len()
                        || (control.stopping && current.plan.kind == zk_longcat::PlanKind::Emphasize) {
                        control.finish(current)?;
                        sequence = None;
                    }
                    timer.admit(ScheduleRequest { key: (), work: "command-evidence", run_at: Instant::now() }).expect("open schedule");
                }
                signal = &mut stop, if !control.stopping => {
                    signal?;
                    control.stopping = true;
                    stopping_at = Some(Instant::now() + Duration::from_millis(command.shutdown_timeout_ms));
                    timer.admit(ScheduleRequest { key: (), work: "shutdown", run_at: Instant::now() }).expect("open schedule");
                }
                work = poll_fn(|cx| std::pin::Pin::new(&mut timer).poll_ready(cx, |_| Some(()))) => {
                    let reason = work.expect("open schedule").work.work;
                    // At most eight history items per turn. Never use a healthy
                    // latest snapshot to skip queued discontinuity evidence.
                    let mut caught_up = observer_closed;
                    if !observer_closed {
                        for _ in 0..8 {
                            match observer.try_recv() {
                                Ok(Some(update)) => {
                                    processed = update.after.clone();
                                    if discontinuity(&control, &update) {
                                        control.lose("queued native discontinuity");
                                        control.stopping = true;
                                        failure = Some(io::Error::new(io::ErrorKind::ConnectionAborted, "queued native discontinuity"));
                                        stopping_at.get_or_insert(Instant::now() + Duration::from_millis(command.shutdown_timeout_ms));
                                    }
                                }
                                Ok(None) => { caught_up = true; break; }
                                Err(error) => {
                                    observer_closed = error == ObserveError::Closed;
                                    control.lose(&format!("observer continuity lost: {error}"));
                                    control.stopping = true;
                                    failure = Some(io::Error::other(error));
                                    stopping_at.get_or_insert(Instant::now() + Duration::from_millis(command.shutdown_timeout_ms));
                                    // No recovery/reacquisition across an unknown gap.
                                    processed = observer.current();
                                    caught_up = true;
                                    break;
                                }
                            }
                        }
                    }
                    let newest = observer.current();
                    if !caught_up || newest.version != processed.version || newest.scope != processed.scope {
                        timer.admit(ScheduleRequest { key: (), work: "history-catchup", run_at: Instant::now() }).expect("open schedule");
                        tokio::task::yield_now().await;
                        continue;
                    }
                    let capture = processed.clone();
                    if !capture.connected { control.lose("native source disconnected"); }
                    let (record, selected) = control.evaluate(&capture, origin.elapsed().as_millis() as u64,
                        sequence.as_ref(), reason)?;
                    sink(record)?;
                    if capture.is_ready() { emitted += 1; }
                    if sequence.is_none() { sequence = selected; }
                    if command.limit.is_some_and(|limit| emitted >= limit) && !control.stopping {
                        control.stopping = true;
                        stopping_at = Some(Instant::now() + Duration::from_millis(command.shutdown_timeout_ms));
                        // A limit caps ordinary diagnostics; final shutdown evidence is
                        // deliberately emitted in addition, never silently discarded.
                        if command.dry_run { break; }
                        if pending.is_none() && sequence.as_ref().is_some_and(|sequence|
                            sequence.plan.kind == zk_longcat::PlanKind::Emphasize) {
                            control.finish(sequence.as_ref().expect("selected sequence"))?;
                            sequence = None;
                        }
                    }
                    if !capture.connected && !control.stopping {
                        failure = Some(io::Error::new(io::ErrorKind::ConnectionAborted, "native source disconnected; episode authority revoked"));
                        control.stopping = true;
                        stopping_at = Some(Instant::now() + Duration::from_millis(command.shutdown_timeout_ms));
                    }
                    if control.stopping && sequence.is_none() && control.next_deadline().is_none() {
                        // Restoration needs one latched pass even if the previous
                        // ordinary decision had no timer (a settled emphasis).
                        let (record, selected) = control.evaluate(&capture, origin.elapsed().as_millis() as u64,
                            None, "conditional-shutdown")?;
                        sink(record)?;
                        sequence = selected;
                        if sequence.is_none() && control.next_deadline().is_none() { break; }
                    }
                    if pending.is_none() && let Some(current) = &sequence {
                        if matches!(control.slots[&current.key].episode.phase(),
                            zk_longcat::Phase::Lost | zk_longcat::Phase::Conflicted)
                            || (control.stopping && current.plan.kind == zk_longcat::PlanKind::Emphasize) {
                            control.finish(current)?;
                            sequence = None;
                            timer.admit(ScheduleRequest { key: (), work: "revoked-unsent-tail", run_at: Instant::now() }).expect("open schedule");
                            continue;
                        }
                        // A publication may have raced calculation/output. Any
                        // queued item blocks admission until a later bounded drain.
                        let empty = match observer.try_recv() {
                            Ok(None) => true,
                            Ok(Some(update)) => {
                                processed = update.after.clone();
                                if discontinuity(&control, &update) {
                                    control.lose("native continuity changed before send");
                                    control.stopping = true;
                                    failure = Some(io::Error::new(io::ErrorKind::ConnectionAborted, "native continuity changed before send"));
                                    stopping_at.get_or_insert(Instant::now() + Duration::from_millis(command.shutdown_timeout_ms));
                                }
                                false
                            }
                            Err(error) => {
                                observer_closed = error == ObserveError::Closed;
                                control.lose(&format!("observer lost continuity before send: {error}"));
                                control.stopping = true;
                                failure = Some(io::Error::other(error));
                                stopping_at.get_or_insert(Instant::now() + Duration::from_millis(command.shutdown_timeout_ms));
                                processed = observer.current();
                                false
                            }
                        };
                        let fresh = observer.current();
                        if !empty || fresh.version != processed.version || fresh.scope != processed.scope {
                            timer.admit(ScheduleRequest { key: (), work: "admission-history-catchup", run_at: Instant::now() }).expect("open schedule");
                            continue;
                        }
                        match control.request(&fresh, current) {
                            Ok(request) if executor.is_some() && !command.dry_run => {
                                pending = Some(executor.as_ref().expect("executor")(request));
                            }
                            result => {
                                if let Err(message) = result {
                                    sink(serde_json::json!({"type":"longcat-admission-refused", "reason":message,
                                        "plan_id":current.plan.id, "unsent_from":current.next}))?;
                                }
                                control.finish(current)?;
                                sequence = None;
                                timer.admit(ScheduleRequest { key: (), work: "unsent-tail", run_at: Instant::now() }).expect("open schedule");
                            }
                        }
                    }
                    if let Some(due) = control.next_deadline() {
                        timer.admit(ScheduleRequest { key: (), work: "convergence-timer",
                            run_at: origin + Duration::from_millis(due) }).expect("open schedule");
                    }
                }
                event = observer.recv(), if !observer_closed => {
                    match event {
                        Ok(update) => {
                            processed = update.after.clone();
                            if discontinuity(&control, &update) {
                                control.lose("native discontinuity");
                                control.stopping = true;
                                failure = Some(io::Error::new(io::ErrorKind::ConnectionAborted, "native discontinuity"));
                                stopping_at.get_or_insert(Instant::now() + Duration::from_millis(command.shutdown_timeout_ms));
                            }
                        }
                        Err(error) => {
                            observer_closed = error == ObserveError::Closed;
                            control.lose(&format!("observer continuity lost: {error}"));
                            failure = Some(io::Error::other(error));
                            control.stopping = true;
                            stopping_at.get_or_insert(Instant::now() + Duration::from_millis(command.shutdown_timeout_ms));
                            processed = observer.current();
                            if pending.is_none() && error == ObserveError::Closed { break; }
                        }
                    }
                    timer.admit(ScheduleRequest { key: (), work: "capture", run_at: Instant::now() }).expect("open schedule");
                }
            }
        }
        Ok(())
    }
    .await;
    // Every exit (including output failure) conservatively retains a polled
    // command as possibly sent before dropping it. Never submit its unsent tail.
    if let Some(current) = &sequence {
        if pending.is_some() {
            control.record(
                current,
                CommandOutcome::PossiblySent {
                    message: "runner stopped before native receipt; no replay".into(),
                },
            )?;
        }
        control.finish(current)?;
    }
    control.lose("runner lifecycle ended");
    let final_record = serde_json::json!({ "type": "longcat-stopped", "episodes": control.slots.values().map(|slot|
        serde_json::json!({"phase":slot.episode.phase(), "baseline":slot.episode.baseline(),
            "requested_effect":slot.episode.requested(), "retained_effect_count":slot.episode.effects().len()})).collect::<Vec<_>>() });
    let final_output = sink(final_record);
    result.and(failure.map_or(Ok(()), Err)).and(final_output)
}

fn discontinuity(control: &Control, update: &zk_desktop::DesktopUpdate) -> bool {
    let first_native_epoch = matches!(
        update.continuity,
        Continuity::Discontinuous(zk_desktop::LossReason::EpochChanged)
    ) && update.before.scope.epoch.0 == 0
        && update.before.windows_status.stamp.is_none()
        && update.before.workspaces_status.stamp.is_none()
        && control.slots.is_empty();
    matches!(update.continuity, Continuity::Discontinuous(_)) && !first_native_epoch
}
