use crate::FocusEvent;
use crate::config::ZoomConfig;
use crate::plan::{
    FontAdjustment, PlanEvent, WindowState, ZoomPlan, applied_effect, plan_transition,
};
use dashmap::DashMap;
use futures::{Stream, StreamExt};
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
        mut events: impl Stream<Item = FocusEvent> + Send + Unpin,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let window_states: DashMap<i32, WindowState> = DashMap::new();

        while let Some(event) = events.next().await {
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
                    let applied = self.apply_commands(pid, &commands).await;

                    if applied == commands.len() {
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
                        match applied_effect(&commands, &window_state, applied) {
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
        }

        Ok(())
    }

    /// Send commands until one is not acknowledged; returns the count applied.
    async fn apply_commands(&self, pid: i32, commands: &[FontAdjustment]) -> usize {
        for (index, adjustment) in commands.iter().enumerate() {
            match self.executor.adjust_font(pid, *adjustment).await {
                Ok(ZoomingResult::Success { .. }) => {}
                Ok(other) => {
                    eprintln!(
                        "Kitty PID {}: command {} ({}) not applied: {:?}",
                        pid,
                        index + 1,
                        adjustment_summary(*adjustment),
                        other
                    );
                    return index;
                }
                Err(error) => {
                    eprintln!(
                        "Kitty PID {}: command {} ({}) failed: {}",
                        pid,
                        index + 1,
                        adjustment_summary(*adjustment),
                        error
                    );
                    return index;
                }
            }
        }
        commands.len()
    }
}

fn adjustment_summary(adjustment: FontAdjustment) -> String {
    format!("{}{}", adjustment.op.as_str(), adjustment.amount)
}
