//! Typed environment overrides and their precedence over loaded configuration.

mod app_launcher;
mod cclip;
mod dmenu;
mod general;
mod helpers;
mod layout;
mod ui;

use super::{ConfigError, FselConfig};

pub(super) fn apply_env_overrides(cfg: &mut FselConfig) -> Result<(), ConfigError> {
    apply_overrides(cfg, &helpers::ProcessEnv)
}

fn apply_overrides(
    cfg: &mut FselConfig,
    source: &impl helpers::OverrideSource,
) -> Result<(), ConfigError> {
    general::apply(cfg, source)?;
    ui::apply(cfg, source)?;
    layout::apply(cfg, source)?;
    dmenu::apply(cfg, source)?;
    cclip::apply(cfg, source)?;
    app_launcher::apply(cfg, source)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{apply_overrides, helpers};
    use crate::cli::{DesktopIconMode, MatchMode};
    use crate::config::{ConfigError, FselConfig};
    use crate::ui::{HorizontalPosition, InputPanelStyle, PanelPosition};
    use std::collections::HashMap;

    struct MapSource {
        vars: HashMap<String, String>,
    }

    impl MapSource {
        fn new(pairs: &[(&str, &str)]) -> Self {
            let vars = pairs
                .iter()
                .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                .collect();
            Self { vars }
        }
    }

    impl helpers::OverrideSource for MapSource {
        fn var(&self, key: &str) -> Result<String, std::env::VarError> {
            self.vars
                .get(key)
                .cloned()
                .ok_or(std::env::VarError::NotPresent)
        }
    }

    #[test]
    fn applies_typed_env_overrides_over_loaded_config_values() {
        let mut config: FselConfig = toml::from_str(
            r#"
match_mode = "fuzzy"

[dmenu]
title_panel_position = "top"

[app_launcher]
prefix_depth = 2
"#,
        )
        .unwrap();

        let source = MapSource::new(&[
            ("FSEL_MATCH_MODE", "exact"),
            ("FSEL_DMENU_TITLE_PANEL_POSITION", "bottom"),
            ("FSEL_APP_LAUNCHER_PREFIX_DEPTH", "8"),
            ("FSEL_APP_LAUNCHER_ICON_MODE", "preview"),
            ("FSEL_APP_LAUNCHER_ICON_POSITION", "left"),
            ("FSEL_APP_LAUNCHER_ICON_ARROW_BEFORE", "true"),
            ("FSEL_APP_LAUNCHER_ICON_HORIZONTAL_ALIGN_PERCENT", "25"),
            ("FSEL_APP_LAUNCHER_ICON_VERTICAL_ALIGN_PERCENT", "75"),
            ("FSEL_APP_LAUNCHER_ICON_LIST_GAP", "2"),
            ("FSEL_APP_LAUNCHER_ICON_LIST_VERTICAL_ALIGN_PERCENT", "-35"),
            ("FSEL_ITEMS_BACKGROUND_COLOR", "#101010"),
            ("FSEL_ITEMS_SELECTION_BACKGROUND_COLOR", "Blue"),
            ("FSEL_MAIN_BACKGROUND_COLOR", "#111111"),
            ("FSEL_INPUT_BACKGROUND_COLOR", "#121212"),
            ("FSEL_SHOW_ITEMS_BORDER", "false"),
            ("FSEL_SHOW_INPUT_PROMPT", "false"),
            ("FSEL_SELECTION_MARKER", "█"),
            ("FSEL_INPUT_PANEL_STYLE", "command"),
            ("FSEL_SHOW_SELECTION_MARKER", "false"),
            ("FSEL_ITEMS_SELECTION_ROUNDED", "true"),
        ]);

        apply_overrides(&mut config, &source).unwrap();

        assert_eq!(config.general.match_mode, MatchMode::Exact);
        assert_eq!(
            config.dmenu.title_panel_position,
            Some(PanelPosition::Bottom)
        );
        assert_eq!(config.app_launcher.prefix_depth, Some(8));
        assert_eq!(
            config.app_launcher.icon_mode,
            Some(DesktopIconMode::Preview)
        );
        assert_eq!(
            config.app_launcher.icon_position,
            Some(HorizontalPosition::Left)
        );
        assert_eq!(config.app_launcher.icon_arrow_before, Some(true));
        assert_eq!(config.app_launcher.icon_horizontal_align_percent, Some(25));
        assert_eq!(config.app_launcher.icon_vertical_align_percent, Some(75));
        assert_eq!(config.app_launcher.icon_list_gap, Some(2));
        assert_eq!(
            config.app_launcher.icon_list_vertical_align_percent,
            Some(-35)
        );
        assert_eq!(config.ui.items_background_color, "#101010");
        assert_eq!(config.ui.items_selection_background_color, "Blue");
        assert_eq!(config.ui.main_background_color, "#111111");
        assert_eq!(config.ui.input_background_color, "#121212");
        assert!(!config.ui.show_items_border);
        assert!(!config.ui.show_input_prompt);
        assert_eq!(config.ui.selection_marker, "█");
        assert_eq!(config.ui.input_panel_style, InputPanelStyle::Command);
        assert!(!config.ui.show_selection_marker);
        assert!(config.ui.items_selection_rounded);
    }

    #[test]
    fn neutral_item_environment_names_override_legacy_aliases() {
        let mut config = FselConfig::default();
        let source = MapSource::new(&[
            ("FSEL_APPS_BACKGROUND_COLOR", "Red"),
            ("FSEL_ITEMS_BACKGROUND_COLOR", "Blue"),
            ("FSEL_SHOW_APPS_BORDER", "true"),
            ("FSEL_SHOW_ITEMS_BORDER", "false"),
        ]);

        apply_overrides(&mut config, &source).unwrap();

        assert_eq!(config.ui.items_background_color, "Blue");
        assert!(!config.ui.show_items_border);
    }

    #[test]
    fn parses_app_launcher_launch_prefix_from_shell_words() {
        let mut config = FselConfig::default();
        let source =
            MapSource::new(&[("FSEL_APP_LAUNCHER_LAUNCH_PREFIX", "env TERM=xterm kitty -e")]);

        apply_overrides(&mut config, &source).unwrap();

        assert_eq!(
            config.app_launcher.launch_prefix,
            Some(vec![
                "env".to_string(),
                "TERM=xterm".to_string(),
                "kitty".to_string(),
                "-e".to_string(),
            ])
        );
    }

    #[test]
    fn reports_invalid_environment_override_with_expected_value() {
        let mut config = FselConfig::default();
        let source = MapSource::new(&[("FSEL_DMENU_TITLE_PANEL_POSITION", "left")]);

        let error = apply_overrides(&mut config, &source).unwrap_err();
        assert!(matches!(
            error,
            ConfigError::InvalidEnvironmentOverride { key, value, expected }
                if key == "FSEL_DMENU_TITLE_PANEL_POSITION"
                    && value == "left"
                    && expected == helpers::PANEL_POSITION_EXPECTED
        ));
    }

    #[test]
    fn preserves_empty_string_env_values_for_string_overrides() {
        let mut config = FselConfig::default();
        let source = MapSource::new(&[("FSEL_CURSOR", "")]);

        apply_overrides(&mut config, &source).unwrap();

        assert_eq!(config.ui.cursor, "");
    }

    #[test]
    fn applies_filter_actions_env_overrides() {
        let mut config = FselConfig::default();
        let source = MapSource::new(&[("FSEL_APP_LAUNCHER_FILTER_ACTIONS", "true")]);

        apply_overrides(&mut config, &source).unwrap();

        assert_eq!(config.app_launcher.filter_actions, Some(true));
    }

    #[test]
    fn applies_auto_hide_duplicates_env_override() {
        let mut config = FselConfig::default();
        let source = MapSource::new(&[("FSEL_APP_LAUNCHER_AUTO_HIDE_DUPLICATES", "true")]);

        apply_overrides(&mut config, &source).unwrap();

        assert_eq!(config.app_launcher.auto_hide_duplicates, Some(true));
    }
}
