use super::error::CliError;
use super::launch::active_launch_method_count;
use super::types::{DesktopIconMode, Opts};

pub(super) fn validate(default: &mut Opts, cli_launch_methods: usize) -> Result<(), CliError> {
    let hidden_commands = usize::from(default.list_hidden)
        + usize::from(default.unhide.is_some())
        + usize::from(default.unhide_all);
    if hidden_commands > 1 {
        return Err(CliError::message(
            "Error: --list-hidden, --unhide, and --unhide-all cannot be combined\n",
        ));
    }
    if hidden_commands > 0
        && (default.clear_history || default.clear_cache || default.refresh_cache)
    {
        return Err(CliError::message(
            "Error: hidden-entry commands cannot be combined with cache/history maintenance\n",
        ));
    }
    if hidden_commands > 0 && (default.program.is_some() || default.search_string.is_some()) {
        return Err(CliError::message(
            "Error: hidden-entry commands cannot be combined with launch or search requests\n",
        ));
    }

    let uses_desktop_icons = !default.dmenu_mode
        && !default.cclip_mode
        && !default.stdout
        && default.program.is_none()
        && hidden_commands == 0
        && !default.clear_history
        && !default.clear_cache
        && !default.refresh_cache;
    if uses_desktop_icons
        && default.desktop_icon_mode.shows_preview()
        && !(10..=90).contains(&default.desktop_icon_preview_width_percent)
    {
        return Err(CliError::message(
            "Error: Desktop icon preview width must be between 10 and 90\n",
        ));
    }
    if uses_desktop_icons
        && default.desktop_icon_mode != DesktopIconMode::None
        && (default.desktop_icon_size == 0 || default.desktop_icon_size > 4096)
    {
        return Err(CliError::message(
            "Error: Desktop icon size must be between 1 and 4096\n",
        ));
    }
    if uses_desktop_icons
        && default.desktop_icon_mode != DesktopIconMode::None
        && (default.desktop_icon_horizontal_align_percent > 100
            || default.desktop_icon_vertical_align_percent > 100)
    {
        return Err(CliError::message(
            "Error: Desktop icon alignment must be between 0 and 100\n",
        ));
    }

    if default.program.is_some() && default.search_string.is_some() {
        return Err(CliError::message(
            "Error: Cannot use -p/--program and -ss together\n\
Use -p for direct launch or -ss for pre-filled TUI search\n",
        ));
    }

    if default.dmenu_mode {
        if hidden_commands > 0 {
            return Err(CliError::message(
                "Error: hidden-entry commands are only available in app-launcher mode\n",
            ));
        }
        if default.program.is_some() {
            return Err(CliError::message(
                "Error: --dmenu cannot be used with -p/--program\n\
Dmenu mode reads from stdin and outputs to stdout\n",
            ));
        }
        default.no_exec = true;
    }

    if default.dmenu_prompt_only && default.dmenu_mode {
        default.dmenu_show_line_numbers = false;
    }

    if default.dmenu_select.is_some() && default.dmenu_select_index.is_some() {
        return Err(CliError::message(
            "Error: Cannot use --select and --select-index together\n",
        ));
    }

    if default.dmenu_index_mode && default.dmenu_index_original_mode {
        return Err(CliError::message(
            "Error: Cannot use --index and --index-original together\n",
        ));
    }

    if default.cclip_mode {
        if hidden_commands > 0 {
            return Err(CliError::message(
                "Error: hidden-entry commands are only available in app-launcher mode\n",
            ));
        }
        if default.program.is_some() {
            return Err(CliError::message(
                "Error: --cclip cannot be used with -p/--program\n\
Cclip mode browses clipboard history and copies selection\n",
            ));
        }
        default.no_exec = true;
    }

    if (default.cclip_tag.is_some()
        || default.cclip_tag_list
        || default.cclip_clear_tags
        || default.cclip_wipe_tags)
        && !default.cclip_mode
    {
        return Err(CliError::message(
            "Error: --tag requires --cclip mode\n\
Use one of these forms:\n\
  fsel --cclip --tag <name>\n\
  fsel --cclip --tag list\n\
  fsel --cclip --tag list <name>\n\
  fsel --cclip --tag clear\n\
  fsel --cclip --tag wipe\n",
        ));
    }

    if default.dmenu_mode && default.cclip_mode {
        return Err(CliError::message(
            "Error: --dmenu and --cclip cannot be used together\n",
        ));
    }

    if default.no_exec {
        if active_launch_method_count(default) > 0 {
            eprintln!("Warning: --no-exec overrides other launch method flags");
        }
        return Ok(());
    }

    if cli_launch_methods > 1 || active_launch_method_count(default) > 1 {
        return Err(CliError::message(
            "Error: Only one launch method can be specified at a time\n\
Available methods: --launch-prefix, --systemd-run, --uwsm\n",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate;
    use crate::cli::{DesktopIconMode, Opts};

    #[test]
    fn reject_both_index_modes() {
        let mut cli = Opts {
            dmenu_index_mode: true,
            dmenu_index_original_mode: true,
            ..Default::default()
        };

        let result = validate(&mut cli, 0);
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Cannot use --index and --index-original")
        );
    }

    #[test]
    fn disabled_icon_layout_ignores_unused_dimensions() {
        let mut options = Opts {
            desktop_icon_mode: DesktopIconMode::None,
            desktop_icon_preview_width_percent: 0,
            desktop_icon_size: 0,
            ..Opts::default()
        };

        assert!(validate(&mut options, 0).is_ok());
    }

    #[test]
    fn active_icon_layout_rejects_alignment_over_one_hundred() {
        let mut options = Opts {
            desktop_icon_horizontal_align_percent: 101,
            ..Opts::default()
        };

        let error = validate(&mut options, 0).expect_err("alignment should be rejected");
        assert!(
            error
                .to_string()
                .contains("alignment must be between 0 and 100")
        );
    }

    #[test]
    fn non_launcher_modes_ignore_launcher_icon_dimensions() {
        for mut options in [
            Opts {
                dmenu_mode: true,
                desktop_icon_preview_width_percent: 0,
                desktop_icon_size: 0,
                ..Opts::default()
            },
            Opts {
                cclip_mode: true,
                desktop_icon_preview_width_percent: 0,
                desktop_icon_size: 0,
                ..Opts::default()
            },
            Opts {
                program: Some("true".to_string()),
                desktop_icon_preview_width_percent: 0,
                desktop_icon_size: 0,
                ..Opts::default()
            },
            Opts {
                stdout: true,
                desktop_icon_preview_width_percent: 0,
                desktop_icon_size: 0,
                ..Opts::default()
            },
        ] {
            assert!(validate(&mut options, 0).is_ok());
        }
    }
}
