use eyre::WrapErr;
use std::process::ExitCode;

use crate::cli;
use crate::modes;
use crate::ui::terminal;

mod cclip;
pub(crate) mod paths;

pub(crate) fn run() -> eyre::Result<ExitCode> {
    let cli = match cli::parse() {
        Ok(cli::CliCommand::Run(cli)) => *cli,
        Ok(cli::CliCommand::PrintShortHelp { program_name }) => {
            print!("{}", cli::short_usage(&program_name));
            return Ok(ExitCode::SUCCESS);
        }
        Ok(cli::CliCommand::PrintLongHelp { program_name }) => {
            print!("{}", cli::detailed_usage(&program_name));
            return Ok(ExitCode::SUCCESS);
        }
        Ok(cli::CliCommand::PrintVersion) => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            return Ok(ExitCode::SUCCESS);
        }
        Err(error) => {
            eprint!("{}", error.render());
            return Ok(error.exit_code());
        }
    };

    if cli.dmenu_mode {
        let rt = tokio::runtime::Runtime::new().wrap_err("Failed to create tokio runtime")?;
        rt.block_on(modes::dmenu::run(&cli))?;
        return Ok(ExitCode::SUCCESS);
    }

    if cli.cclip_mode {
        return cclip::run(&cli);
    }

    let rt = tokio::runtime::Runtime::new().wrap_err("Failed to create tokio runtime")?;
    rt.block_on(modes::app_launcher::run(cli))?;
    Ok(ExitCode::SUCCESS)
}

pub(crate) fn cleanup_after_error() {
    let _ = terminal::shutdown_terminal(false);
}
