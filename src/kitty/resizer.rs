use crate::kitty::{KittyRegistry, ZoomingResult};
use crate::niri::types::NiriEvent;
use futures::{Stream, StreamExt};

pub struct KittyResizer {
    kitty_registry: KittyRegistry,
    step_size: u32,
}

impl KittyResizer {
    pub fn new(kitty_registry: KittyRegistry) -> Self {
        Self {
            kitty_registry,
            step_size: 3,
        }
    }

    pub fn with_step_size(kitty_registry: KittyRegistry, step_size: u32) -> Self {
        Self {
            kitty_registry,
            step_size,
        }
    }

    pub async fn process_events(
        &mut self,
        mut events: impl Stream<Item = NiriEvent> + std::marker::Send + std::marker::Unpin,
    ) -> Result<ZoomingResult, Box<dyn std::error::Error>> {
        while let Some(event) = events.next().await {
            match event {
                NiriEvent::Focus { window, .. } => {
                    if let Some(pid) = window.pid {
                        eprintln!(
                            "Kitty window {} gained focus (PID {}), increasing font by +{}",
                            window.id, pid, self.step_size
                        );
                        let _ = self
                            .kitty_registry
                            .increase_font_size_by(pid, self.step_size)
                            .await;
                    }
                }
                NiriEvent::Blur { window, .. } => {
                    if let Some(pid) = window.pid {
                        eprintln!(
                            "Kitty window {} lost focus (PID {}), decreasing font by -{}",
                            window.id, pid, self.step_size
                        );
                        let _ = self
                            .kitty_registry
                            .decrease_font_size_by(pid, self.step_size)
                            .await;
                    }
                }
                _ => {}
            }
        }

        Ok(ZoomingResult::NotConfigured)
    }
}
