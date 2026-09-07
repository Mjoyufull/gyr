// App launcher mode - verb-based organization

mod admin;
mod direct;
mod events;
mod icons;
pub mod launch;
pub mod run;
pub mod search;
mod session;

// Re-export the run function
pub use run::run;
