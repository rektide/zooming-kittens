use crate::config::{ZoomConfig, ZoomType};
use crate::kitty::KittyRegistry;
use crate::kitty::conf_parser::get_baseline_font_size;
use crate::niri::types::NiriEvent;
use dashmap::DashMap;
use futures::{Stream, StreamExt};

#[derive(Debug, Clone)]
struct WindowState {
    current_font_size: Option<f64>,
    current_zoom_factor: f64,
}

impl WindowState {
    fn new() -> Self {
        Self {
            current_font_size: None,
            current_zoom_factor: 1.0,
        }
    }

    fn with_baseline() -> Self {
        Self {
            current_font_size: get_baseline_font_size(),
            current_zoom_factor: 1.0,
        }
    }
}

pub struct KittyResizer {
    kitty_registry: KittyRegistry,
    zoom_config: ZoomConfig,
}

impl KittyResizer {
    pub fn new(kitty_registry: KittyRegistry) -> Self {
        Self {
            kitty_registry,
            zoom_config: ZoomConfig::default(),
        }
    }

    pub fn with_zoom_config(kitty_registry: KittyRegistry, zoom_config: ZoomConfig) -> Self {
        Self {
            kitty_registry,
            zoom_config,
        }
    }

    pub async fn process_events(
        &mut self,
        mut events: impl Stream<Item = NiriEvent> + std::marker::Send + std::marker::Unpin,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let window_states: DashMap<i32, WindowState> = DashMap::new();

        while let Some(event) = events.next().await {
            match event {
                NiriEvent::Focus { window, .. } => {
                    if let Some(pid) = window.pid {
                        if let Some(zoom_type) = self.zoom_config.active_type() {
                            let step_size = self.zoom_config.step_size;
                            let mut window_state = window_states.entry(pid).or_insert_with(|| {
                                WindowState::with_baseline()
                            });

                            let current_font = window_state.current_font_size
                                .or(get_baseline_font_size())
                                .unwrap_or(12.0);

                            match zoom_type {
                                ZoomType::Absolute => {
                                    if let Some(target) = self.zoom_config.absolute {
                                        if current_font < target {
                                            let diff = target - current_font;
                                            let steps = (diff / step_size).ceil() as u32;
                                            let _ = self.kitty_registry
                                                .increase_font_size_by(pid, steps * step_size as u32)
                                                .await;
                                            window_state.current_font_size = Some(target);
                                            eprintln!(
                                                "Kitty window {} gained focus (PID {}), setting absolute font size to {}",
                                                window.id, pid, target
                                            );
                                        } else if current_font > target {
                                            let diff = current_font - target;
                                            let steps = (diff / step_size).ceil() as u32;
                                            let _ = self.kitty_registry
                                                .decrease_font_size_by(pid, steps * step_size as u32)
                                                .await;
                                            window_state.current_font_size = Some(target);
                                            eprintln!(
                                                "Kitty window {} gained focus (PID {}), setting absolute font size to {}",
                                                window.id, pid, target
                                            );
                                        }
                                    }
                                }
                                ZoomType::Additive => {
                                    if let Some(amount) = self.zoom_config.additive {
                                        let steps = (amount / step_size).ceil() as u32;
                                        let _ = self.kitty_registry
                                            .increase_font_size_by(pid, steps * step_size as u32)
                                            .await;
                                        window_state.current_font_size = Some(current_font + amount);
                                        eprintln!(
                                            "Kitty window {} gained focus (PID {}), increasing font by +{}",
                                            window.id, pid, amount
                                        );
                                    }
                                }
                                ZoomType::Multiplicative => {
                                    if let Some(factor) = self.zoom_config.multiplicative {
                                        let baseline = get_baseline_font_size().unwrap_or(12.0);
                                        let target_factor = factor;
                                        let current_factor = window_state.current_zoom_factor;

                                        if (target_factor - current_factor).abs() > 0.001 {
                                            let multiply = target_factor > current_factor;
                                            let op = if multiply { "*" } else { "/" };
                                            let step_factor = step_size;

                                            let mut zoom_factor = current_factor;
                                            let mut steps_applied = 0;

                                            while (multiply && zoom_factor < target_factor) || (!multiply && zoom_factor > target_factor) {
                                                let next_factor = if multiply {
                                                    zoom_factor * step_factor
                                                } else {
                                                    zoom_factor / step_factor
                                                };

                                                let should_apply = if multiply {
                                                    next_factor <= target_factor
                                                } else {
                                                    next_factor >= target_factor
                                                };

                                                if should_apply {
                                                    let _ = self.kitty_registry
                                                        .execute_font_command_with_op(pid, op, step_factor)
                                                        .await;
                                                    zoom_factor = next_factor;
                                                    steps_applied += 1;
                                                } else {
                                                    let final_factor = target_factor / zoom_factor;
                                                    let _ = self.kitty_registry
                                                        .execute_font_command_with_op(pid, op, final_factor)
                                                        .await;
                                                    steps_applied += 1;
                                                    break;
                                                }
                                            }

                                            window_state.current_zoom_factor = target_factor;
                                            window_state.current_font_size = Some(baseline * target_factor);
                                            eprintln!(
                                                "Kitty window {} gained focus (PID {}), scaling from {:.2}x to {:.2}x ({} steps)",
                                                window.id, pid, current_factor, target_factor, steps_applied
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                NiriEvent::Blur { window, .. } => {
                    if let Some(pid) = window.pid {
                        if let Some(zoom_type) = self.zoom_config.active_type() {
                            let step_size = self.zoom_config.step_size;
                            let mut window_state = window_states.entry(pid).or_insert_with(|| {
                                WindowState::with_baseline()
                            });

                            let current_font = window_state.current_font_size
                                .or(get_baseline_font_size())
                                .unwrap_or(12.0);

                            match zoom_type {
                                ZoomType::Absolute => {
                                    let baseline = get_baseline_font_size().unwrap_or(12.0);
                                    if current_font > baseline {
                                        let diff = current_font - baseline;
                                        let steps = (diff / step_size).ceil() as u32;
                                        let _ = self.kitty_registry
                                            .decrease_font_size_by(pid, steps * step_size as u32)
                                            .await;
                                        window_state.current_font_size = Some(baseline);
                                        window_state.current_zoom_factor = 1.0;
                                        eprintln!(
                                            "Kitty window {} lost focus (PID {}), restoring baseline font size to {}",
                                            window.id, pid, baseline
                                        );
                                    } else if current_font < baseline {
                                        let diff = baseline - current_font;
                                        let steps = (diff / step_size).ceil() as u32;
                                        let _ = self.kitty_registry
                                            .increase_font_size_by(pid, steps * step_size as u32)
                                            .await;
                                        window_state.current_font_size = Some(baseline);
                                        window_state.current_zoom_factor = 1.0;
                                        eprintln!(
                                            "Kitty window {} lost focus (PID {}), restoring baseline font size to {}",
                                            window.id, pid, baseline
                                        );
                                    }
                                }
                                ZoomType::Additive => {
                                    if let Some(amount) = self.zoom_config.additive {
                                        let steps = (amount / step_size).ceil() as u32;
                                        let _ = self.kitty_registry
                                            .decrease_font_size_by(pid, steps * step_size as u32)
                                            .await;
                                        window_state.current_font_size = Some(current_font - amount);
                                        eprintln!(
                                            "Kitty window {} lost focus (PID {}), decreasing font by -{}",
                                            window.id, pid, amount
                                        );
                                    }
                                }
                                ZoomType::Multiplicative => {
                                    if let Some(_factor) = self.zoom_config.multiplicative {
                                        let target_factor = 1.0;
                                        let current_factor = window_state.current_zoom_factor;

                                        if (target_factor - current_factor).abs() > 0.001 {
                                            let multiply = target_factor > current_factor;
                                            let op = if multiply { "*" } else { "/" };
                                            let step_factor = step_size;

                                            let mut zoom_factor = current_factor;
                                            let mut steps_applied = 0;

                                            while (multiply && zoom_factor < target_factor) || (!multiply && zoom_factor > target_factor) {
                                                let next_factor = if multiply {
                                                    zoom_factor * step_factor
                                                } else {
                                                    zoom_factor / step_factor
                                                };

                                                let should_apply = if multiply {
                                                    next_factor <= target_factor
                                                } else {
                                                    next_factor >= target_factor
                                                };

                                                if should_apply {
                                                    let _ = self.kitty_registry
                                                        .execute_font_command_with_op(pid, op, step_factor)
                                                        .await;
                                                    zoom_factor = next_factor;
                                                    steps_applied += 1;
                                                } else {
                                                    let final_factor = target_factor / zoom_factor;
                                                    let _ = self.kitty_registry
                                                        .execute_font_command_with_op(pid, op, final_factor)
                                                        .await;
                                                    break;
                                                }
                                            }

                                            window_state.current_zoom_factor = target_factor;
                                            window_state.current_font_size = get_baseline_font_size();
                                            eprintln!(
                                                "Kitty window {} lost focus (PID {}), scaling from {:.2}x to {:.2}x ({} steps)",
                                                window.id, pid, current_factor, target_factor, steps_applied
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}
