//! Niri desktop observation and explicitly targeted height requests.
#![forbid(unsafe_code)]

mod reducer;
mod source;
pub use reducer::{Limits, ReduceError, Reducer};
pub use source::{
    HeightActuator, HeightRequest, Options, Source, Transmission, subscribe, subscribe_env,
};
