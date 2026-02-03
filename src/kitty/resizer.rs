use crate::registry::{KittyRegistry, ZoomingResult};
use crate::niri::types::NiriEvent;
use futures::{Stream, StreamExt};

pub struct KittyResizer {
    kitty_registry: KittyRegistry,
}

impl KittyResizer {
    pub fn new(kitty_registry: KittyRegistry) -> Self {
        Self { kitty_registry }
    }

    pub async fn process_events(
        &mut self,
        mut events: impl Stream<Item = NiriEvent> + std::marker::Send + std::marker::Unpin,
    ) -> Result<ZoomingResult, Box<dyn std::error::Error>> {
        while let Some(event) = events.next().await {
            match event {
                NiriEvent::Focus { window, .. } => {
                    if let Some(pid) = window.pid {
                        if self.kitty_registry.verbose() {
                            eprintln!("Kitty window {} gained focus, increasing font", window.id);
                        }
                        let _ = self.kitty_registry.increase_font_size(pid).await;
                    }
                }
                NiriEvent::Blur { window, .. } => {
                    if let Some(pid) = window.pid {
                        if self.kitty_registry.verbose() {
                            eprintln!("Kitty window {} lost focus, decreasing font", window.id);
                        }
                        let _ = self.kitty_registry.decrease_font_size(pid).await;
                    }
                }
                _ => {}
            }
        }

        Ok(ZoomingResult::NotConfigured)
    }
}
