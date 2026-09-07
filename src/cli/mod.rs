mod color;
mod error;
mod from_config;
mod help;
mod launch;
mod parse;
mod types;
mod validate;

use std::sync::atomic::AtomicBool;

#[derive(Debug)]
pub(crate) enum CliCommand {
    Run(Box<Opts>),
    PrintShortHelp { program_name: String },
    PrintLongHelp { program_name: String },
    PrintVersion,
}

pub use crate::ui::PanelPosition;
pub use color::string_to_color;
pub use types::{DesktopIconMode, MatchMode, Opts, PinnedOrderMode, RankingMode};

pub(crate) use help::{detailed_usage, short_usage};
pub(crate) use parse::parse;

pub static DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);
