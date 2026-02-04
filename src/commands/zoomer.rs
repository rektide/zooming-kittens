use crate::config::{Config, RegistryConfig, Verbosity};
use crate::kitty::resizer::KittyResizer;
use crate::niri::registry::NiriRegistry;

/// Run focus tracking for a specific app with configurable font adjustments
pub async fn run_zoomer(
    app_id: String,
    verbosity: Verbosity,
    config: Config,
) -> Result<(), Box<dyn std::error::Error>> {
    if verbosity.log_window_events() {
        eprintln!("Starting zoomer for app_id: {}", app_id);
        if let Some(zoom_type) = config.zoom.active_type() {
            match zoom_type {
                crate::config::ZoomType::Absolute => {
                    eprintln!("Zoom type: Absolute (target: {})", config.zoom.value().unwrap());
                }
                crate::config::ZoomType::Additive => {
                    eprintln!("Zoom type: Additive (amount: {})", config.zoom.value().unwrap());
                }
                crate::config::ZoomType::Multiplicative => {
                    eprintln!("Zoom type: Multiplicative (factor: {})", config.zoom.value().unwrap());
                }
            }
            eprintln!("Step size: {}", config.zoom.step_size);
        } else {
            eprintln!("Warning: No zoom configuration set, using default additive +6");
        }
    }

    let registry_config = config.to_registry_config();
    let kitty_registry = crate::kitty::KittyRegistry::with_verbosity(registry_config, verbosity);
    kitty_registry.start_reaper().await;

    let niri_registry = NiriRegistry::new_with_verbosity(verbosity).await?;
    let mut zoomer = KittyResizer::with_zoom_config(kitty_registry, config.zoom);

    let kitty_events =
        niri_registry.windows_matching(|window| window.app_id.as_deref() == Some(&app_id));

    zoomer.process_events(kitty_events).await?;

    Ok(())
}
