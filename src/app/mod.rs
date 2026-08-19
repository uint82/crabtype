pub mod state;
pub mod lifecycle;
pub mod input;
pub mod words;
pub mod display;
pub mod scroll;
pub mod stats;
pub mod scoring;

#[cfg(test)]
mod absorb_test;

pub use state::App;
