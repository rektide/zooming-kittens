use crate::FocusEvent;
use crate::config::ZoomConfig;
use crate::plan::{
    FontAdjustment, FontOp, PlanEvent, WindowState, ZoomPlan, applied_effect, plan_transition,
};
use dashmap::DashMap;
use futures::{
    Stream, StreamExt,
    future::{Either, select},
};
use std::{collections::BTreeMap, pin::Pin};
use zk_kitty::KittyRegistry;
use zk_kitty::ZoomingResult;
use zk_kitty::conf_parser::get_baseline_font_size;

/// The command transport the resizer needs: apply one adjustment to one
/// terminal and report whether kitty acknowledged it.
///
/// Implemented for [`KittyRegistry`]; tests script results without any live
/// kitty connection.
pub trait FontPort {
    fn adjust_font(
        &self,
        pid: i32,
        adjustment: FontAdjustment,
    ) -> impl Future<Output = Result<ZoomingResult, Box<dyn std::error::Error + Send + Sync>>> + Send;
}

impl FontPort for KittyRegistry {
    async fn adjust_font(
        &self,
        pid: i32,
        adjustment: FontAdjustment,
    ) -> Result<ZoomingResult, Box<dyn std::error::Error + Send + Sync>> {
        self.execute_font_command_with_op(pid, adjustment.op.as_str(), adjustment.amount)
            .await
            .map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e.to_string()))
    }
}

/// Focus-to-font policy over any [`FontPort`] transport.
///
/// Events are planned purely (see [`crate::plan`]) and applied command by
/// command. The per-window state advances only by the portion of commands
/// kitty actually acknowledged, so a failed transition leaves honest local
/// state instead of the pre-transition claim of success.
pub struct ZoomResizer<E> {
    executor: E,
    zoom_config: ZoomConfig,
}

/// The kitty-backed resizer; the name kept from the pre-extraction application.
pub type KittyResizer = ZoomResizer<KittyRegistry>;

impl<E: FontPort> ZoomResizer<E> {
    pub fn new(executor: E) -> Self {
        Self {
            executor,
            zoom_config: ZoomConfig::default(),
        }
    }

    pub fn with_zoom_config(executor: E, zoom_config: ZoomConfig) -> Self {
        Self {
            executor,
            zoom_config,
        }
    }

    pub async fn process_events(
        &mut self,
        events: impl Stream<Item = FocusEvent> + Send + Unpin,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.process_events_until(events, std::future::pending())
            .await
    }

    /// Explicit orderly stop: finish the in-flight command (not its whole
    /// animation), then reverse only acknowledged effects. Stream loss/closure
    /// is not a shutdown request and does not synthesize blur or restoration.
    /// The caller owns the shutdown deadline; expiry can leave unknown effects.
    pub async fn process_events_until(
        &mut self,
        mut events: impl Stream<Item = FocusEvent> + Send + Unpin,
        stop: impl Future<Output = ()>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let window_states: DashMap<i32, WindowState> = DashMap::new();
        let mut effects = BTreeMap::<i32, OwnedEffect>::new();
        futures::pin_mut!(stop);
        let mut stopping = false;

        loop {
            let event = match select(stop.as_mut(), events.next()).await {
                Either::Left(_) => {
                    stopping = true;
                    break;
                }
                Either::Right((Some(event), _)) => event,
                Either::Right((None, _)) => break,
            };
            let (event_kind, window) = match event {
                FocusEvent::Focus { window } => (PlanEvent::Focus, window),
                FocusEvent::Blur { window } => (PlanEvent::Blur, window),
            };
            let Some(pid) = window.pid else {
                continue;
            };

            let baseline = get_baseline_font_size();
            let mut window_state = window_states
                .entry(pid)
                .or_insert_with(|| WindowState::with_baseline(baseline));

            let plan = plan_transition(&self.zoom_config, event_kind, &window_state, baseline);
            match plan {
                ZoomPlan::Noop => {}
                ZoomPlan::Invalid(reason) => {
                    eprintln!(
                        "Kitty window {} (PID {}): skipping zoom transition: {}",
                        window.id, pid, reason
                    );
                }
                ZoomPlan::Transition {
                    commands,
                    from,
                    target,
                    target_factor,
                } => {
                    let (applied, stopped) = self
                        .apply_commands(pid, &commands, &mut effects, stop.as_mut())
                        .await;
                    stopping = stopped;
                    // The planner's `from` includes its documented baseline
                    // fallback, including when kitty.conf had no font size.
                    let before = WindowState {
                        font: Some(from),
                        ..*window_state
                    };

                    if applied == commands.len() {
                        if let Some(effect) = applied_effect(&commands, &before, applied) {
                            window_state.linear_offset = effect.linear_offset;
                        }
                        window_state.font = Some(target);
                        if let Some(factor) = target_factor {
                            window_state.factor = factor;
                        }
                        let verb = match event_kind {
                            PlanEvent::Focus => "gained",
                            PlanEvent::Blur => "lost",
                        };
                        eprintln!(
                            "Kitty window {} (PID {}) {} focus, font {} -> {} in {} command(s)",
                            window.id, pid, verb, from, target, applied
                        );
                    } else if applied > 0 {
                        // Partially applied: commit only the acknowledged
                        // portion so the next event plans from the real size.
                        match applied_effect(&commands, &before, applied) {
                            Some(effect) => {
                                eprintln!(
                                    "Kitty window {} (PID {}): transition interrupted after {} of {} command(s), font tracked at {}",
                                    window.id,
                                    pid,
                                    applied,
                                    commands.len(),
                                    effect.font
                                );
                                window_state.font = Some(effect.font);
                                window_state.factor = effect.factor;
                                window_state.linear_offset = effect.linear_offset;
                            }
                            None => {
                                eprintln!(
                                    "Kitty window {} (PID {}): transition interrupted after {} of {} command(s); keeping previous tracked size",
                                    window.id,
                                    pid,
                                    applied,
                                    commands.len()
                                );
                            }
                        }
                    }
                }
            }
            if stopping {
                break;
            }
        }

        if stopping {
            self.restore_effects(effects).await?;
        }
        Ok(())
    }

    /// Send commands until one is not acknowledged; returns the count applied.
    async fn apply_commands<S: Future<Output = ()>>(
        &self,
        pid: i32,
        commands: &[FontAdjustment],
        effects: &mut BTreeMap<i32, OwnedEffect>,
        mut stop: Pin<&mut S>,
    ) -> (usize, bool) {
        for (index, adjustment) in commands.iter().enumerate() {
            // Never cancel a sent command on orderly stop: await its receipt
            // before deciding what belongs in the restoration ledger.
            let command = self.executor.adjust_font(pid, *adjustment);
            futures::pin_mut!(command);
            let mut started = false;
            let receipt = {
                let tracked = futures::future::poll_fn(|cx| {
                    started = true;
                    command.as_mut().poll(cx)
                });
                futures::pin_mut!(tracked);
                match select(stop.as_mut(), tracked).await {
                    Either::Left(_) => None,
                    Either::Right((result, _)) => Some(result),
                }
            };
            let (result, stopping) = match receipt {
                Some(result) => (result, false),
                None if !started => return (index, true),
                None => (command.await, true),
            };
            match result {
                Ok(ZoomingResult::Success { .. }) => {
                    effects.entry(pid).or_default().record(*adjustment);
                    if stopping {
                        return (index + 1, true);
                    }
                }
                Ok(other) => {
                    eprintln!(
                        "Kitty PID {}: command {} ({}) not applied: {:?}",
                        pid,
                        index + 1,
                        adjustment_summary(*adjustment),
                        other
                    );
                    return (index, stopping);
                }
                Err(error) => {
                    eprintln!(
                        "Kitty PID {}: command {} ({}) failed: {}",
                        pid,
                        index + 1,
                        adjustment_summary(*adjustment),
                        error
                    );
                    return (index, stopping);
                }
            }
        }
        (commands.len(), false)
    }

    async fn restore_effects(
        &self,
        effects: BTreeMap<i32, OwnedEffect>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut failed = false;
        for (pid, effect) in effects {
            for adjustment in effect.inverse() {
                match self.executor.adjust_font(pid, adjustment).await {
                    Ok(ZoomingResult::Success { .. }) => {
                        eprintln!(
                            "Kitty PID {pid}: shutdown restored {}",
                            adjustment_summary(adjustment)
                        );
                    }
                    _ => {
                        eprintln!(
                            "Kitty PID {pid}: shutdown restoration not acknowledged; manual correction may be needed"
                        );
                        failed = true;
                    }
                }
            }
        }
        if failed {
            return Err("kitty shutdown restoration incomplete".into());
        }
        Ok(())
    }
}

/// Net effects owned by this run, not a remembered user font setting. Policy
/// uses either linear or multiplicative adjustments, never a mixture. The
/// inverse is relative, so additive zoom doesn't reset a manual size change.
struct OwnedEffect {
    additive: f64,
    factor: f64,
}

impl Default for OwnedEffect {
    fn default() -> Self {
        Self {
            additive: 0.0,
            factor: 1.0,
        }
    }
}

impl OwnedEffect {
    fn record(&mut self, adjustment: FontAdjustment) {
        match adjustment.op {
            FontOp::Add => self.additive += adjustment.amount,
            FontOp::Sub => self.additive -= adjustment.amount,
            FontOp::Mul => self.factor *= adjustment.amount,
            FontOp::Div => self.factor /= adjustment.amount,
        }
    }

    fn inverse(&self) -> Vec<FontAdjustment> {
        let mut commands = Vec::with_capacity(2);
        if self.additive.abs() > f64::EPSILON {
            commands.push(FontAdjustment {
                op: if self.additive > 0.0 {
                    FontOp::Sub
                } else {
                    FontOp::Add
                },
                amount: self.additive.abs(),
            });
        }
        if (self.factor - 1.0).abs() > f64::EPSILON {
            commands.push(FontAdjustment {
                op: FontOp::Div,
                amount: self.factor,
            });
        }
        commands
    }
}

fn adjustment_summary(adjustment: FontAdjustment) -> String {
    format!("{}{}", adjustment.op.as_str(), adjustment.amount)
}
