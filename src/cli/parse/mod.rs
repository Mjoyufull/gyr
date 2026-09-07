mod helpers;
mod overrides;

use super::error::CliError;
use super::{CliCommand, validate};
use crate::config::FselConfig;
use std::env;
use std::path::PathBuf;

pub fn parse() -> Result<CliCommand, CliError> {
    parse_from(env::args())
}

pub(crate) fn parse_from<I, S>(args: I) -> Result<CliCommand, CliError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args = collect_args(args);
    let config = FselConfig::new(find_config_path(&args))?;
    parse_with_config(&args, config)
}

fn parse_with_config(args: &[String], config: FselConfig) -> Result<CliCommand, CliError> {
    let program_name = args.first().cloned().unwrap_or_else(|| "fsel".to_string());
    let mut default = super::types::Opts::default();
    super::from_config::apply_config_defaults(&mut default, &config);

    if program_name.ends_with("dmenu") {
        default.dmenu_mode = true;
    }

    let mut parser = build_parser(args, &mut default);
    let cli_launch_methods =
        match overrides::parse_cli_overrides(&mut parser, &mut default, &program_name)? {
            overrides::OverridesResult::Continue(count) => count,
            overrides::OverridesResult::Command(command) => return Ok(command),
        };

    validate::validate(&mut default, cli_launch_methods)?;
    Ok(CliCommand::Run(Box::new(default)))
}

fn collect_args<I, S>(args: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args: Vec<String> = args.into_iter().map(Into::into).collect();
    if args.is_empty() {
        args.push("fsel".to_string());
    }
    args
}

fn find_config_path(args: &[String]) -> Option<PathBuf> {
    let mut index = 1;
    while index < args.len() {
        if (args[index] == "-c" || args[index] == "--config") && index + 1 < args.len() {
            return Some(PathBuf::from(&args[index + 1]));
        }
        index += 1;
    }
    None
}

fn build_parser(args: &[String], default: &mut super::types::Opts) -> lexopt::Parser {
    if let Some(search_pos) = args.iter().position(|arg| arg == "-ss") {
        default.search_string = Some(args[search_pos + 1..].join(" "));
        return lexopt::Parser::from_args(args[..search_pos].iter().skip(1).cloned());
    }

    lexopt::Parser::from_args(args.iter().skip(1).cloned())
}

#[cfg(test)]
mod tests {
    use super::{CliCommand, CliError, parse_with_config};
    use crate::cli::Opts;
    use crate::config::FselConfig;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn short_help_returns_command_without_exiting() {
        let command = parse_with_config(&args(&["fsel", "-h"]), FselConfig::default()).unwrap();
        assert!(matches!(command, CliCommand::PrintShortHelp { .. }));
    }

    #[test]
    fn version_returns_command_without_exiting() {
        let command =
            parse_with_config(&args(&["fsel", "--version"]), FselConfig::default()).unwrap();
        assert!(matches!(command, CliCommand::PrintVersion));
    }

    #[test]
    fn invalid_tag_mode_returns_typed_error() {
        let error = parse_with_config(&args(&["fsel", "--tag", "list"]), FselConfig::default())
            .unwrap_err();
        assert!(
            matches!(error, CliError::Message(message) if message.contains("--tag requires --cclip mode"))
        );
    }

    #[test]
    fn tag_list_does_not_consume_following_flag_as_tag_name() {
        let command = parse_with_config(
            &args(&["fsel", "--cclip", "--tag", "list", "--help"]),
            FselConfig::default(),
        )
        .unwrap();

        assert!(matches!(command, CliCommand::PrintLongHelp { .. }));
    }

    #[test]
    fn filter_actions_flag_parses_yes_and_no_forms() {
        let enabled =
            parse_with_config(&args(&["fsel", "--filter-actions"]), FselConfig::default()).unwrap();
        let disabled = parse_with_config(
            &args(&["fsel", "--filter-actions=no"]),
            FselConfig::default(),
        )
        .unwrap();

        let CliCommand::Run(enabled_opts) = enabled else {
            panic!("expected run command");
        };
        let CliCommand::Run(disabled_opts) = disabled else {
            panic!("expected run command");
        };

        let enabled_opts: Box<Opts> = enabled_opts;
        let disabled_opts: Box<Opts> = disabled_opts;
        assert!(enabled_opts.filter_actions);
        assert!(!disabled_opts.filter_actions);
    }

    #[test]
    fn auto_hide_duplicates_flag_parses_yes_and_no_forms() {
        let enabled = parse_with_config(
            &args(&["fsel", "--auto-hide-duplicates"]),
            FselConfig::default(),
        )
        .unwrap();
        let disabled = parse_with_config(
            &args(&["fsel", "--auto-hide-duplicates=no"]),
            FselConfig::default(),
        )
        .unwrap();

        let CliCommand::Run(enabled_opts) = enabled else {
            panic!("expected run command");
        };
        let CliCommand::Run(disabled_opts) = disabled else {
            panic!("expected run command");
        };

        assert!(enabled_opts.auto_hide_duplicates);
        assert!(!disabled_opts.auto_hide_duplicates);
    }

    #[test]
    fn hidden_entry_management_flags_parse() {
        let command =
            parse_with_config(&args(&["fsel", "--unhide", "42"]), FselConfig::default()).unwrap();
        let CliCommand::Run(opts) = command else {
            panic!("expected run command");
        };

        assert_eq!(opts.unhide, Some(42));
    }

    #[test]
    fn hidden_entry_management_flags_are_mutually_exclusive() {
        let error = parse_with_config(
            &args(&["fsel", "--list-hidden", "--unhide-all"]),
            FselConfig::default(),
        )
        .unwrap_err();

        assert!(
            matches!(error, CliError::Message(message) if message.contains("cannot be combined"))
        );
    }

    #[test]
    fn hidden_entry_management_does_not_silently_ignore_launch_requests() {
        let error = parse_with_config(
            &args(&["fsel", "--unhide", "42", "--program", "firefox"]),
            FselConfig::default(),
        )
        .unwrap_err();

        assert!(
            matches!(error, CliError::Message(message) if message.contains("launch or search"))
        );
    }

    #[test]
    fn preview_flag_enables_dmenu_mode_and_preserves_command() {
        let command = parse_with_config(
            &args(&["fsel", "--preview", "cat {}"]),
            FselConfig::default(),
        )
        .unwrap();

        let CliCommand::Run(opts) = command else {
            panic!("expected run command");
        };
        assert!(opts.dmenu_mode);
        assert_eq!(opts.dmenu_preview.as_deref(), Some("cat {}"));
    }
}
