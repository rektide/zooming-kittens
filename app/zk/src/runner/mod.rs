//! Bounded application composition, not a reusable controller host.
mod bridge;
mod control;
mod native;

#[cfg(test)]
mod tests;

use crate::{commands::longcat::LongcatCommand, config::LongcatConfig};
use control::{Control, Sequence};
use futures::{
    FutureExt,
    future::{BoxFuture, poll_fn},
};
use kubeless_work_schedule::{ReadyWork, ScheduleOptions, ScheduleRequest, schedule};
use serde_json::Value;
use std::{io, sync::Arc, time::Duration};
use tokio::{io::AsyncWriteExt, time::Instant};
use zk_desktop::{Continuity, ObserveError, Observer};
use zk_longcat::CommandOutcome;

type Executor =
    Arc<dyn Fn(zk_niri::HeightRequest) -> BoxFuture<'static, zk_niri::Transmission> + Send + Sync>;

pub async fn run(command: LongcatCommand, config: LongcatConfig) -> io::Result<()> {
    let (owner, observer, actuator) = native::subscribe(&command).await?;
    let executor: Option<Executor> = actuator.map(|actuator| {
        Arc::new(move |request| {
            let actuator = actuator.clone();
            async move { actuator.set_height(request).await }.boxed()
        }) as Executor
    });
    // Native I/O never awaits stdout. A slow/broken diagnostic consumer ends the
    // app rather than accumulating an unbounded backlog or silently losing rows.
    let (output, mut rows) = tokio::sync::mpsc::channel::<Vec<u8>>(16);
    let writer = tokio::spawn(async move {
        let mut stdout = tokio::io::stdout();
        while let Some(row) = rows.recv().await {
            tokio::time::timeout(Duration::from_secs(2), stdout.write_all(&row))
                .await
                .map_err(|_| {
                    io::Error::new(io::ErrorKind::TimedOut, "diagnostic stdout stalled")
                })??;
        }
        tokio::time::timeout(Duration::from_secs(2), stdout.flush())
            .await
            .map_err(|_| {
                io::Error::new(io::ErrorKind::TimedOut, "diagnostic stdout flush stalled")
            })?
    });
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
        output.try_send(bytes).map_err(|e| {
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
    drop(output);
    let shutdown = tokio::time::timeout(Duration::from_secs(3), owner.shutdown())
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "native source shutdown deadline"))?;
    let written = writer.await.map_err(io::Error::other)?;
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
    let mut pending: Option<BoxFuture<'static, zk_niri::Transmission>> = None;
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
                    let outcome = match outcome {
                        zk_niri::Transmission::Handled => CommandOutcome::Handled,
                        zk_niri::Transmission::Rejected(message) => CommandOutcome::Rejected { message },
                        zk_niri::Transmission::PossiblySent(message) => CommandOutcome::PossiblySent { message },
                        zk_niri::Transmission::Unsent(_) => CommandOutcome::Unsent,
                    };
                    let proceed = outcome == CommandOutcome::Handled;
                    control.record(current, outcome)?;
                    current.next += 1;
                    if !proceed || current.next == current.plan.targets.len() {
                        control.finish(current)?;
                        sequence = None;
                    }
                    timer.admit(ScheduleRequest { key: (), work: "command-evidence", run_at: Instant::now() }).expect("open schedule");
                }
                work = poll_fn(|cx| std::pin::Pin::new(&mut timer).poll_ready(cx, |_| Some(()))) => {
                    let reason = work.expect("open schedule").work.work;
                    let capture = observer.current();
                    if !capture.connected { control.lose("native source disconnected"); }
                    let (record, selected) = control.evaluate(&capture, origin.elapsed().as_millis() as u64,
                        sequence.as_ref(), reason)?;
                    sink(record)?;
                    emitted += 1;
                    if sequence.is_none() { sequence = selected; }
                    if command.limit.is_some_and(|limit| emitted >= limit) && !control.stopping {
                        control.stopping = true;
                        stopping_at = Some(Instant::now() + Duration::from_millis(command.shutdown_timeout_ms));
                        // A limit caps ordinary diagnostics; final shutdown evidence is
                        // deliberately emitted in addition, never silently discarded.
                        if command.dry_run { break; }
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
                        // Inspect receiver loss before granting another mutation.
                        // One item only; current() supplies the newest capture.
                        match observer.try_recv() {
                            Err(error) => {
                                control.lose(&format!("observer lost continuity: {error}"));
                                failure = Some(io::Error::other(error));
                                control.stopping = true;
                                stopping_at.get_or_insert(Instant::now() + Duration::from_millis(command.shutdown_timeout_ms));
                            }
                            Ok(Some(update)) if matches!(update.continuity, Continuity::Discontinuous(_)) => {
                                control.lose("native continuity changed before send");
                            }
                            _ => {}
                        }
                        match control.request(&observer.current(), current) {
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
                signal = &mut stop, if !control.stopping => {
                    signal?;
                    control.stopping = true;
                    stopping_at = Some(Instant::now() + Duration::from_millis(command.shutdown_timeout_ms));
                    timer.admit(ScheduleRequest { key: (), work: "shutdown", run_at: Instant::now() }).expect("open schedule");
                }
                event = observer.recv(), if !observer_closed => {
                    match event {
                        Ok(update) => {
                            let first_native_epoch = matches!(update.continuity,
                                Continuity::Discontinuous(zk_desktop::LossReason::EpochChanged))
                                && update.before.scope.epoch.0 == 0
                                && update.before.windows_status.stamp.is_none()
                                && update.before.workspaces_status.stamp.is_none()
                                && control.slots.is_empty();
                            if matches!(update.continuity, Continuity::Discontinuous(_)) && !first_native_epoch {
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
                            if pending.is_none() && error == ObserveError::Closed { break; }
                        }
                    }
                    timer.admit(ScheduleRequest { key: (), work: "capture", run_at: Instant::now() }).expect("open schedule");
                }
            }
        }
        Ok(())
    }.await;
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
