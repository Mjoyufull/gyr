use super::helpers::{parse_column_list, value_as_string};
use crate::cli::error::CliError;
use crate::cli::help::unknown_argument_help;
use crate::cli::launch::{parse_launch_prefix, set_launch_prefix, set_systemd_run, set_uwsm};
use crate::cli::{CliCommand, DesktopIconMode, MatchMode, Opts};
use lexopt::prelude::*;

pub(super) enum OverridesResult {
    Continue(usize),
    Command(CliCommand),
}

pub(super) fn parse_cli_overrides(
    parser: &mut lexopt::Parser,
    default: &mut Opts,
    program_name: &str,
) -> Result<OverridesResult, CliError> {
    let mut cli_launch_methods = 0;

    while let Some(arg) = parser.next()? {
        match arg {
            Long("panel-edit") => {
                default.dmenu_panel_edit = true;
                default.dmenu_mode = true;
            }
            Long("panel") => {
                let spec = value_as_string(parser, "Invalid panel specification")?;
                default.dmenu_panels.push(
                    crate::modes::dmenu::panels::DmenuPanel::parse(&spec)
                        .map_err(CliError::message)?,
                );
                default.dmenu_mode = true;
            }
            Long("info-position") => {
                default.panels.info_position = Some(
                    value_as_string(parser, "Invalid panel side")?
                        .parse()
                        .map_err(CliError::message)?,
                );
            }
            Long("input-position") => {
                default.panels.input_position = Some(
                    value_as_string(parser, "Invalid panel side")?
                        .parse()
                        .map_err(CliError::message)?,
                );
            }
            Long("info-size") => {
                default.panels.info_size = Some(
                    value_as_string(parser, "Invalid info size")?
                        .parse()
                        .map_err(|_| CliError::message("info size must be an integer"))?,
                );
            }
            Long("input-size") => {
                default.panels.input_size = Some(
                    value_as_string(parser, "Invalid input size")?
                        .parse()
                        .map_err(|_| CliError::message("input size must be an integer"))?,
                );
            }
            Long("layout-rotation") => {
                default.panels.rotation = value_as_string(parser, "Invalid rotation")?
                    .parse()
                    .map_err(|_| CliError::message("layout rotation must be an integer"))?;
            }
            Long("item-width") => {
                default.panels.item_width = value_as_string(parser, "Invalid item width")?
                    .parse()
                    .map_err(|_| CliError::message("item width must be an integer"))?;
            }
            Short('t') | Long("tty") => {
                default.tty = true;
                default.terminal_launcher.clear();
            }
            Short('r') | Long("replace") => {
                default.replace = true;
            }
            Short('c') | Long("config") => {
                let _ = parser.value()?;
            }
            Long("clear-history") => {
                default.clear_history = true;
            }
            Long("clear-cache") => {
                default.clear_cache = true;
            }
            Long("refresh-cache") => {
                default.refresh_cache = true;
            }
            Long("list-hidden") => {
                default.list_hidden = true;
            }
            Long("unhide") => {
                let id = value_as_string(parser, "Hidden entry ID must be valid UTF-8")?;
                default.unhide = Some(
                    id.parse::<u64>()
                        .map_err(|_| CliError::message("Hidden entry ID must be an integer"))?,
                );
            }
            Long("unhide-all") => {
                default.unhide_all = true;
            }
            Long("no-exec") => {
                default.no_exec = true;
            }
            Long("stdout") => {
                default.stdout = true;
            }
            Long("launch-prefix") => {
                cli_launch_methods += 1;
                let prefix = value_as_string(parser, "Launch prefix must be valid UTF-8")?;
                set_launch_prefix(
                    default,
                    parse_launch_prefix(&prefix).map_err(CliError::message)?,
                );
            }
            Long("systemd-run") => {
                cli_launch_methods += 1;
                set_systemd_run(default);
            }
            Long("uwsm") => {
                cli_launch_methods += 1;
                set_uwsm(default);
            }
            Short('d') | Long("detach") => {
                default.detach = true;
            }
            Long("dmenu") => {
                default.dmenu_mode = true;
            }
            Long("cclip") => {
                default.cclip_mode = true;
            }
            Long("tag") => parse_tag(parser, default)?,
            Long("cclip-show-tag-color-names") => {
                default.cclip_show_tag_color_names = Some(true);
            }
            Long("dmenu0") => {
                default.dmenu_mode = true;
                default.dmenu_null_separated = true;
            }
            Long("preview") => {
                default.dmenu_mode = true;
                default.dmenu_preview = Some(value_as_string(
                    parser,
                    "Preview command must be valid UTF-8",
                )?);
            }
            Long("password") => {
                default.dmenu_password_mode = true;
                if let Some(value) = parser.optional_value() {
                    default.dmenu_password_character = value
                        .into_string()
                        .map_err(|_| CliError::message("Password character must be valid UTF-8"))?;
                }
            }
            Long("index") => {
                default.dmenu_index_mode = true;
            }
            Long("index-original") => {
                default.dmenu_index_original_mode = true;
            }
            Long("accept-nth") => {
                default.dmenu_accept_nth =
                    Some(parse_column_list(parser, "Invalid column specification")?);
            }
            Long("match-nth") => {
                default.dmenu_match_nth =
                    Some(parse_column_list(parser, "Invalid column specification")?);
            }
            Long("only-match") => {
                default.dmenu_only_match = true;
            }
            Long("exit-if-empty") => {
                default.dmenu_exit_if_empty = true;
            }
            Long("select") => {
                default.dmenu_select = Some(value_as_string(
                    parser,
                    "Select string must be valid UTF-8",
                )?);
            }
            Long("select-index") => {
                let index = value_as_string(parser, "Index must be valid UTF-8")?;
                default.dmenu_select_index = Some(
                    index
                        .parse::<usize>()
                        .map_err(|_| CliError::message("Invalid index"))?,
                );
            }
            Long("auto-select") => {
                default.dmenu_auto_select = true;
            }
            Long("prompt-only") => {
                default.dmenu_prompt_only = true;
            }
            Long("hide-before-typing") => {
                default.hide_before_typing = true;
            }
            Long("filter-desktop") => {
                if let Some(value) = parser.optional_value() {
                    let value = value.into_string().map_err(|_| {
                        CliError::message("filter-desktop value must be valid UTF-8")
                    })?;
                    default.filter_desktop = value != "no";
                } else {
                    default.filter_desktop = true;
                }
            }
            Long("filter-actions") => {
                if let Some(value) = parser.optional_value() {
                    let value = value.into_string().map_err(|_| {
                        CliError::message("filter-actions value must be valid UTF-8")
                    })?;
                    default.filter_actions = value != "no";
                } else {
                    default.filter_actions = true;
                }
            }
            Long("auto-hide-duplicates") => {
                if let Some(value) = parser.optional_value() {
                    let value = value.into_string().map_err(|_| {
                        CliError::message("auto-hide-duplicates value must be valid UTF-8")
                    })?;
                    default.auto_hide_duplicates = value != "no";
                } else {
                    default.auto_hide_duplicates = true;
                }
            }
            Long("list-executables-in-path") => {
                default.list_executables_in_path = true;
            }
            Long("desktop-icons") => {
                default.desktop_icon_mode = match parser.optional_value() {
                    Some(value) => value
                        .into_string()
                        .map_err(|_| CliError::message("Desktop icon mode must be valid UTF-8"))?
                        .parse::<DesktopIconMode>()
                        .map_err(CliError::message)?,
                    None => DesktopIconMode::Preview,
                };
            }
            Long("icon-position") => {
                default.desktop_icon_position =
                    value_as_string(parser, "Desktop icon position must be valid UTF-8")?
                        .parse()
                        .map_err(CliError::message)?;
            }
            Long("icon-preview-width") => {
                default.desktop_icon_preview_width_percent =
                    value_as_string(parser, "Desktop icon preview width must be valid UTF-8")?
                        .parse::<u16>()
                        .map_err(|_| {
                            CliError::message("Desktop icon preview width must be an integer")
                        })?;
            }
            Long("icon-list-width") => {
                default.desktop_icon_list_width =
                    value_as_string(parser, "Desktop list icon width must be valid UTF-8")?
                        .parse::<u16>()
                        .map_err(|_| {
                            CliError::message("Desktop list icon width must be an integer")
                        })?;
            }
            Long("icon-list-height") => {
                default.desktop_icon_list_height =
                    value_as_string(parser, "Desktop list icon height must be valid UTF-8")?
                        .parse::<u16>()
                        .map_err(|_| {
                            CliError::message("Desktop list icon height must be an integer")
                        })?;
            }
            Long("icon-list-gap") => {
                default.desktop_icon_list_gap =
                    value_as_string(parser, "Desktop list icon gap must be valid UTF-8")?
                        .parse::<u16>()
                        .map_err(|_| {
                            CliError::message("Desktop list icon gap must be an integer")
                        })?;
            }
            Long("icon-list-vertical-align") => {
                default.desktop_icon_list_vertical_align_percent = value_as_string(
                    parser,
                    "Desktop list icon vertical alignment must be valid UTF-8",
                )?
                .parse::<i16>()
                .map_err(|_| {
                    CliError::message("Desktop list icon vertical alignment must be an integer")
                })?;
            }
            Long("icon-arrow-before") => {
                default.desktop_icon_arrow_before = true;
            }
            Long("icon-size") => {
                default.desktop_icon_size =
                    value_as_string(parser, "Desktop icon size must be valid UTF-8")?
                        .parse::<u16>()
                        .map_err(|_| CliError::message("Desktop icon size must be an integer"))?;
            }
            Long("icon-horizontal-align") => {
                default.desktop_icon_horizontal_align_percent = value_as_string(
                    parser,
                    "Desktop icon horizontal alignment must be valid UTF-8",
                )?
                .parse::<u16>()
                .map_err(|_| {
                    CliError::message("Desktop icon horizontal alignment must be an integer")
                })?;
            }
            Long("icon-vertical-align") => {
                default.desktop_icon_vertical_align_percent = value_as_string(
                    parser,
                    "Desktop icon vertical alignment must be valid UTF-8",
                )?
                .parse::<u16>()
                .map_err(|_| {
                    CliError::message("Desktop icon vertical alignment must be an integer")
                })?;
            }
            Long("icon-theme") => {
                default.desktop_icon_theme = Some(value_as_string(
                    parser,
                    "Desktop icon theme must be valid UTF-8",
                )?);
            }
            Long("match-mode") => {
                let mode = value_as_string(parser, "Match mode must be valid UTF-8")?;
                default.match_mode = mode
                    .parse::<MatchMode>()
                    .map_err(|_| CliError::message("Invalid match mode. Use 'exact' or 'fuzzy'"))?;
            }
            Long("prefix-depth") => {
                let depth = value_as_string(parser, "Prefix depth must be valid UTF-8")?;
                default.prefix_depth = depth
                    .parse::<usize>()
                    .map_err(|_| CliError::message("Invalid prefix depth"))?;
            }
            Short('T') | Long("test") => {
                default.test_mode = true;
                default.verbose = Some(3);
            }
            Long("with-nth") => {
                default.dmenu_with_nth = Some(parse_column_list(
                    parser,
                    "Invalid column specification. Use comma-separated numbers like: 1,2,4",
                )?);
            }
            Long("delimiter") => {
                default.dmenu_delimiter = value_as_string(parser, "Delimiter must be valid UTF-8")?;
            }
            Short('p') | Long("program") => {
                default.program =
                    Some(value_as_string(parser, "Program name must be valid UTF-8")?);
            }
            Short('v') | Long("verbose") => {
                default.verbose = Some(default.verbose.unwrap_or(0) + 1);
            }
            Short('h') => {
                return Ok(OverridesResult::Command(CliCommand::PrintShortHelp {
                    program_name: program_name.to_string(),
                }));
            }
            Short('H') | Long("help") => {
                return Ok(OverridesResult::Command(CliCommand::PrintLongHelp {
                    program_name: program_name.to_string(),
                }));
            }
            Short('V') | Long("version") => {
                return Ok(OverridesResult::Command(CliCommand::PrintVersion));
            }
            Value(_) => return Err(arg.unexpected().into()),
            _ => return Err(report_unknown_argument(arg)),
        }
    }

    Ok(OverridesResult::Continue(cli_launch_methods))
}

fn parse_tag(parser: &mut lexopt::Parser, default: &mut Opts) -> Result<(), CliError> {
    let tag_arg = value_as_string(parser, "Tag argument must be valid UTF-8")?;
    match tag_arg.as_str() {
        "list" => {
            default.cclip_tag_list = true;
            if let Some(value) = parser.optional_value() {
                default.cclip_tag = Some(
                    value
                        .into_string()
                        .map_err(|_| CliError::message("Tag name must be valid UTF-8"))?,
                );
            }
        }
        "clear" => {
            default.cclip_clear_tags = true;
        }
        "wipe" => {
            default.cclip_wipe_tags = true;
        }
        _ => {
            default.cclip_tag = Some(tag_arg);
        }
    }

    Ok(())
}

fn report_unknown_argument(arg: lexopt::Arg<'_>) -> CliError {
    let error_msg = match arg {
        Long(name) => match name {
            "clip" => "Unknown option '--clip'. Did you mean '--cclip'?",
            "menu" => "Unknown option '--menu'. Did you mean '--dmenu'?",
            "dme" | "dmen" => "Unknown option. Did you mean '--dmenu'?",
            "cc" | "ccli" => "Unknown option. Did you mean '--cclip'?",
            _ => "Unknown option. Use '-h' or '--help' to see available options.",
        },
        Short(c) => match c {
            'C' => "Unknown option '-C'. Did you mean '-c' for --config?",
            'P' => "Unknown option '-P'. Did you mean '-p' for --program?",
            'R' => "Unknown option '-R'. Did you mean '-r' for --replace?",
            _ => "Unknown option. Use '-h' or '--help' to see available options.",
        },
        Value(_) => unreachable!(),
    };

    CliError::message(unknown_argument_help(error_msg))
}
