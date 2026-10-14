//! `liteweight-fps` ported to WebAssembly.
//!
//! The simulation, physics, world, UI mesh generation, and the wgpu renderer
//! are shared with the native game. Only the platform layer (window creation,
//! audio, settings storage, and the event loop entry point) is web-specific.
#![cfg_attr(
    not(target_arch = "wasm32"),
    allow(dead_code, unused_imports, unused_mut, unused_variables)
)]

pub mod engine;
pub mod persistence;
pub mod physics;
pub mod platform;
pub mod simulation;
pub mod ui;
pub mod world;

pub use world::{Scene, Vertex};

#[cfg(target_arch = "wasm32")]
pub mod app;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

/// Entry point invoked by the generated JavaScript glue via Trunk.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    app::run();
}
