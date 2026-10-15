//! Read-only Umbriel desktop observation, composed over its native inventory.
#![forbid(unsafe_code)]

mod reducer;
mod source;
pub use reducer::{ReduceError, Reducer};
pub use source::{Options, Source, subscribe};
pub use umbriel_client::{Discovery, Endpoint, SubscriptionOptions};
