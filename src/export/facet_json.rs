// pattern: Imperative Shell

//! Experimental facet-rs export backend scaffold.
//!
//! `facet-json` requires exported values to implement `facet::Facet`. The
//! upstream `niri_ipc` types currently derive serde traits, but not `Facet`, so
//! the existing `ExportRecord` cannot be written by facet-json without local
//! proxy/DTO types for every nested niri IPC payload. Keep this module as the
//! feature-gated boundary for that future backend rather than mixing the
//! experiment into the serde writer.

#[allow(dead_code)]
pub fn backend_status() -> &'static str {
    "facet export backend requires local Facet proxy types for niri_ipc payloads"
}
