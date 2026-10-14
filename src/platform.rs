//! Platform abstractions for time and entropy.
//!
//! On `wasm32-unknown-unknown` the standard library's clock is unavailable, so
//! [`web_time`] (backed by `Performance.now()` / `Date.now()`) is re-exported
//! under the same names. On native targets these are re-exports of `std::time`.
pub use web_time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// A value that differs between runs, used to seed gameplay randomization.
///
/// The web has no process id, so a random value is used instead.
pub fn process_id() -> u32 {
    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Math::random() * f64::from(u32::MAX)) as u32
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::process::id()
    }
}
