//! Compositor-independent focus-to-font policy, composed with kitty remote control.

pub mod config;
pub mod resizer;

pub use config::{ZoomConfig, ZoomType};
pub use resizer::KittyResizer;

/// A terminal focus target. The opaque ID is used for diagnostics; the PID is
/// resolved to a kitty master by `zk-kitty`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusTarget {
    pub id: String,
    pub pid: Option<i32>,
}

/// Blur must carry the target that lost focus, not the newly focused target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusEvent {
    Focus { window: FocusTarget },
    Blur { window: FocusTarget },
}
