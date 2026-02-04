use crate::config::{ZoomConfig, ZoomType};
use crate::kitty::KittyRegistry;
use crate::kitty::conf_parser::get_baseline_font_size;
use crate::niri::types::NiriEvent;
use dashmap::DashMap;
use futures::{Stream, StreamExt};

#[derive(Debug, Clone)]
struct WindowState {
    current_font_size: Option<f64>,
}

impl WindowState {
    fn new() -> Self {
        Self {
            current_font_size: None,
        }
    }

    fn with_baseline() -> Self {
        Self {
            current_font_size: get_baseline_font_size(),
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
                                            let steps = (diff / step_size as f64).ceil() as u32;
                                            let _ = self.kitty_registry
                                                .increase_font_size_by(pid, steps * step_size)
                                                .await;
                                            window_state.current_font_size = Some(target);
                                            eprintln!(
                                                "Kitty window {} gained focus (PID {}), setting absolute font size to {}",
                                                window.id, pid, target
                                            );
                                        } else if current_font > target {
                                            let diff = current_font - target;
                                            let steps = (diff / step_size as f64).ceil() as u32;
                                            let _ = self.kitty_registry
                                                .decrease_font_size_by(pid, steps * step_size)
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
                                        let steps = (amount / step_size as f64).ceil() as u32;
                                        let _ = self.kitty_registry
                                            .increase_font_size_by(pid, steps * step_size)
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
                                        let target = baseline * factor;
                                        let mut current = current_font;

                                        if (target - current).abs() > 0.01 {
                                            while current < target {
                                                let next_val = current * step_size as f64;
                                                if next_val >= target {
                                                    let final_multiplier = (target / current).ceil() as u32;
                                                    let _ = self.kitty_registry
                                                        .execute_font_command_with_op(pid, "*", final_multiplier)
                                                        .await;
                                                    eprintln!(
                                                        "  Final step: *{} (target: {})",
                                                        final_multiplier, target
                                                    );
                                                    break;
                                                }
                                                let _ = self.kitty_registry
                                                    .multiply_font_size_by(pid, 1)
                                                    .await;
                                                current = next_val;
                                                eprintln!(
                                                    "  Step: *{} ({} -> {})",
                                                    step_size,
                                                    current / step_size as f64,
                                                    current
                                                );
                                            }
                                            window_state.current_font_size = Some(target);
                                            eprintln!(
                                                "Kitty window {} gained focus (PID {}), scaled to {} ({}x from baseline {})",
                                                window.id, pid, target, factor, baseline
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
                                        let steps = (diff / step_size as f64).ceil() as u32;
                                        let _ = self.kitty_registry
                                            .decrease_font_size_by(pid, steps * step_size)
                                            .await;
                                        window_state.current_font_size = Some(baseline);
                                        eprintln!(
                                            "Kitty window {} lost focus (PID {}), restoring baseline font size to {}",
                                            window.id, pid, baseline
                                        );
                                    } else if current_font < baseline {
                                        let diff = baseline - current_font;
                                        let steps = (diff / step_size as f64).ceil() as u32;
                                        let _ = self.kitty_registry
                                            .increase_font_size_by(pid, steps * step_size)
                                            .await;
                                        window_state.current_font_size = Some(baseline);
                                        eprintln!(
                                            "Kitty window {} lost focus (PID {}), restoring baseline font size to {}",
                                            window.id, pid, baseline
                                        );
                                    }
                                }
                                ZoomType::Additive => {
                                    if let Some(amount) = self.zoom_config.additive {
                                        let steps = (amount / step_size as f64).ceil() as u32;
                                        let _ = self.kitty_registry
                                            .decrease_font_size_by(pid, steps * step_size)
                                            .await;
                                        window_state.current_font_size = Some(current_font - amount);
                                        eprintln!(
                                            "Kitty window {} lost focus (PID {}), decreasing font by -{}",
                                            window.id, pid, amount
                                        );
                                    }
                                }
                                ZoomType::Multiplicative => {
                                    if let Some(factor) = self.zoom_config.multiplicative {
                                        let baseline = get_baseline_font_size().unwrap_or(12.0);
                                        let target = baseline / factor;
                                        let mut current = current_font;

                                        if (current - target).abs() > 0.01 {
                                            while current > target {
                                                let next_val = current / step_size as f64;
                                                if next_val <= target {
                                                    let final_divisor = (current / target).ceil() as u32;
                                                    let _ = self.kitty_registry
                                                        .execute_font_command_with_op(pid, "/", final_divisor)
                                                        .await;
                                                    eprintln!(
                                                        "  Final step: /{} (target: {})",
                                                        final_divisor, target
                                                    );
                                                    break;
                                                }
                                                let _ = self.kitty_registry
                                                    .divide_font_size_by(pid, 1)
                                                    .await;
                                                current = next_val;
                                                eprintln!(
                                                    "  Step: /{} ({} -> {})",
                                                    step_size,
                                                    current * step_size as f64,
                                                    current
                                                );
                                            }
                                            window_state.current_font_size = Some(target);
                                            eprintln!(
                                                "Kitty window {} lost focus (PID {}), scaled to {} (1/{}x from baseline {})",
                                                window.id, pid, target, factor, baseline
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
