// Dmenu mode - verb-based organization

mod events;
mod options;
pub mod parse;
mod preview;
mod render;
pub mod run;

// Re-export the run function
pub use run::run;
