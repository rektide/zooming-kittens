use crate::config::RegistryConfig;
use crate::kitty::resizer::KittyResizer;
use crate::niri::registry::NiriRegistry;

/// Run focus tracking for a specific app with +6/-6 font adjustments
pub async fn run_zoomer(app_id: String, verbose: bool, registry_config: RegistryConfig) -> Result<(), Box<dyn std::error::Error>> {
    if verbose {
        eprintln!("Starting zoomer for app_id: {}", app_id);
        eprintln!("Font adjustment: +6 on focus, -6 on blur");
    }

    let kitty_registry = crate::kitty::KittyRegistry::new(registry_config);
    kitty_registry.start_reaper().await;

    let niri_registry = NiriRegistry::new().await?;
    let mut zoomer = KittyResizer::with_step_size(kitty_registry, 6);

    let kitty_events = niri_registry.windows_matching(|window| {
        window.app_id.as_deref() == Some(&app_id)
    });

    zoomer.process_events(kitty_events).await?;

    Ok(())
}
