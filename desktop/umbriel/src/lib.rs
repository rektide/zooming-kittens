//! Umbriel stock observation and explicit native control boundaries.
#![forbid(unsafe_code)]

pub mod edge;
pub mod geometry;
pub mod layout;
mod reducer;
mod source;
pub mod stock;
pub use reducer::{ReduceError, Reducer};
pub use source::{Options, Source, WindowsRefresh, subscribe, subscribe_layout};
pub use umbriel_client::{Discovery, Endpoint, SubscriptionOptions};
