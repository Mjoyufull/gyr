use crate::config::ConfigError;
use std::env;
use std::str::FromStr;

pub(super) const BOOLEAN_EXPECTED: &str = "true or false";
pub(super) const INTEGER_EXPECTED: &str = "an unsigned integer";
pub(super) const MATCH_MODE_EXPECTED: &str = "'fuzzy' or 'exact'";
pub(super) const RANKING_MODE_EXPECTED: &str = "'frecency', 'recency', or 'frequency'";
pub(super) const PINNED_ORDER_EXPECTED: &str =
    "'ranking', 'alphabetical', 'oldest', 'oldest_pinned', 'newest', or 'newest_pinned'";
pub(super) const PANEL_POSITION_EXPECTED: &str = "'top', 'middle', or 'bottom'";
pub(super) const HORIZONTAL_POSITION_EXPECTED: &str = "'left' or 'right'";
pub(super) const DESKTOP_ICON_MODE_EXPECTED: &str = "'none', 'preview', 'list', or 'both'";
pub(super) const LAUNCH_PREFIX_EXPECTED: &str = "a shell-words command prefix";

pub(super) trait OverrideSource {
    fn var(&self, key: &str) -> Result<String, env::VarError>;
}

pub(super) struct ProcessEnv;

impl OverrideSource for ProcessEnv {
    fn var(&self, key: &str) -> Result<String, env::VarError> {
        env::var(key)
    }
}

pub(super) fn set_string(source: &impl OverrideSource, key: &str, target: &mut String) {
    if let Ok(value) = source.var(key) {
        *target = value;
    }
}

pub(super) fn set_parsed<T>(
    source: &impl OverrideSource,
    key: &str,
    target: &mut T,
    expected: &'static str,
) -> Result<(), ConfigError>
where
    T: FromStr,
{
    if let Ok(value) = source.var(key) {
        match value.parse() {
            Ok(parsed) => *target = parsed,
            Err(_) => return Err(invalid_environment_override(key, value, expected)),
        }
    }

    Ok(())
}

pub(super) fn set_optional_string(
    source: &impl OverrideSource,
    key: &str,
    target: &mut Option<String>,
) {
    if let Ok(value) = source.var(key) {
        *target = Some(value);
    }
}

pub(super) fn set_optional_parsed<T>(
    source: &impl OverrideSource,
    key: &str,
    target: &mut Option<T>,
    expected: &'static str,
) -> Result<(), ConfigError>
where
    T: FromStr,
{
    if let Ok(value) = source.var(key) {
        match value.parse() {
            Ok(parsed) => *target = Some(parsed),
            Err(_) => return Err(invalid_environment_override(key, value, expected)),
        }
    }

    Ok(())
}

pub(super) fn set_optional_launch_prefix(
    source: &impl OverrideSource,
    key: &str,
    target: &mut Option<Vec<String>>,
) -> Result<(), ConfigError> {
    if let Ok(value) = source.var(key) {
        match shell_words::split(&value) {
            Ok(prefix) => *target = Some(prefix),
            Err(_) => {
                return Err(invalid_environment_override(
                    key,
                    value,
                    LAUNCH_PREFIX_EXPECTED,
                ));
            }
        }
    }

    Ok(())
}

fn invalid_environment_override(key: &str, value: String, expected: &'static str) -> ConfigError {
    ConfigError::InvalidEnvironmentOverride {
        key: key.to_string(),
        value,
        expected,
    }
}
