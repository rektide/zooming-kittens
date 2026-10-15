//! Compositor-independent focus-to-font policy, composed with kitty remote control.

pub mod config;
pub mod plan;
pub mod resizer;

pub use config::{ZoomConfig, ZoomType};
pub use plan::{
    AppliedEffect, FontAdjustment, FontOp, MAX_PLAN_STEPS, PlanEvent, WindowState, ZoomPlan,
};
pub use resizer::{FontPort, KittyResizer, ZoomResizer};

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
