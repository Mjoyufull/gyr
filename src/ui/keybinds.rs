//! Configurable key matching and compact user-facing binding hints.

use crossterm::event::{KeyCode, KeyModifiers};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Keybinds {
    #[serde(default = "default_up")]
    pub up: Vec<KeyBind>,
    #[serde(default = "default_down")]
    pub down: Vec<KeyBind>,
    #[serde(default = "default_left")]
    pub left: Vec<KeyBind>,
    #[serde(default = "default_right")]
    pub right: Vec<KeyBind>,
    #[serde(default = "default_select")]
    pub select: Vec<KeyBind>,
    #[serde(default = "default_exit")]
    pub exit: Vec<KeyBind>,
    #[serde(default = "default_pin")]
    pub pin: Vec<KeyBind>,
    #[serde(default = "default_hide")]
    pub hide: Vec<KeyBind>,
    #[serde(default = "default_unhide_last")]
    pub unhide_last: Vec<KeyBind>,
    #[serde(default = "default_backspace")]
    pub backspace: Vec<KeyBind>,
    #[serde(default = "default_image_preview")]
    pub image_preview: Vec<KeyBind>,
    #[serde(default = "default_tag")]
    pub tag: Vec<KeyBind>,
    #[serde(default = "default_cclip_delete")]
    pub cclip_delete: Vec<KeyBind>,
}

impl Default for Keybinds {
    fn default() -> Self {
        Self {
            up: default_up(),
            down: default_down(),
            left: default_left(),
            right: default_right(),
            select: default_select(),
            exit: default_exit(),
            pin: default_pin(),
            hide: default_hide(),
            unhide_last: default_unhide_last(),
            backspace: default_backspace(),
            image_preview: default_image_preview(),
            tag: default_tag(),
            cclip_delete: default_cclip_delete(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum KeyBind {
    Simple(String),
    WithMod { key: String, modifiers: String },
}

impl KeyBind {
    pub fn matches(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        match self {
            KeyBind::Simple(key) => {
                let parsed = parse_key(key);
                key_codes_match(parsed.0, code, KeyModifiers::NONE) && mods == KeyModifiers::NONE
            }
            KeyBind::WithMod { key, modifiers } => {
                let parsed = parse_key(key);
                let parsed_mods = parse_modifiers(modifiers);
                key_codes_match(parsed.0, code, parsed_mods)
                    && modifiers_match(parsed.0, code, parsed_mods, mods)
            }
        }
    }

    fn hint(&self) -> String {
        match self {
            Self::Simple(key) => key.to_lowercase(),
            Self::WithMod { key, modifiers } => {
                format!("{}+{}", modifiers.to_lowercase(), key.to_lowercase())
            }
        }
    }
}

fn modifiers_match(
    configured_code: KeyCode,
    input_code: KeyCode,
    configured_modifiers: KeyModifiers,
    input_modifiers: KeyModifiers,
) -> bool {
    input_modifiers == configured_modifiers
        || (configured_code == KeyCode::Tab
            && input_code == KeyCode::BackTab
            && configured_modifiers.contains(KeyModifiers::SHIFT)
            && input_modifiers == configured_modifiers.difference(KeyModifiers::SHIFT))
}

fn key_codes_match(
    configured_code: KeyCode,
    input_code: KeyCode,
    configured_modifiers: KeyModifiers,
) -> bool {
    match (configured_code, input_code) {
        (KeyCode::Tab, KeyCode::BackTab) => configured_modifiers.contains(KeyModifiers::SHIFT),
        (KeyCode::Char(configured_char), KeyCode::Char(input_char)) => {
            configured_char.eq_ignore_ascii_case(&input_char)
        }
        _ => configured_code == input_code,
    }
}

fn parse_key(key: &str) -> (KeyCode, KeyModifiers) {
    match key.to_lowercase().as_str() {
        "up" => (KeyCode::Up, KeyModifiers::NONE),
        "down" => (KeyCode::Down, KeyModifiers::NONE),
        "left" => (KeyCode::Left, KeyModifiers::NONE),
        "right" => (KeyCode::Right, KeyModifiers::NONE),
        "enter" => (KeyCode::Enter, KeyModifiers::NONE),
        "esc" | "escape" => (KeyCode::Esc, KeyModifiers::NONE),
        "backspace" => (KeyCode::Backspace, KeyModifiers::NONE),
        "delete" => (KeyCode::Delete, KeyModifiers::NONE),
        "tab" => (KeyCode::Tab, KeyModifiers::NONE),
        "space" => (KeyCode::Char(' '), KeyModifiers::NONE),
        s if s.len() == 1 => (KeyCode::Char(s.chars().next().unwrap()), KeyModifiers::NONE),
        _ => (KeyCode::Null, KeyModifiers::NONE),
    }
}

fn parse_modifiers(mods: &str) -> KeyModifiers {
    let mut result = KeyModifiers::NONE;
    for part in mods.split('+') {
        match part.trim().to_lowercase().as_str() {
            "ctrl" | "control" => result |= KeyModifiers::CONTROL,
            "shift" => result |= KeyModifiers::SHIFT,
            "alt" => result |= KeyModifiers::ALT,
            _ => {}
        }
    }
    result
}

fn default_up() -> Vec<KeyBind> {
    vec![
        KeyBind::Simple("up".to_string()),
        KeyBind::WithMod {
            key: "p".to_string(),
            modifiers: "ctrl".to_string(),
        },
    ]
}

fn default_down() -> Vec<KeyBind> {
    vec![
        KeyBind::Simple("down".to_string()),
        KeyBind::WithMod {
            key: "n".to_string(),
            modifiers: "ctrl".to_string(),
        },
    ]
}

fn default_left() -> Vec<KeyBind> {
    vec![KeyBind::Simple("left".to_string())]
}

fn default_right() -> Vec<KeyBind> {
    vec![KeyBind::Simple("right".to_string())]
}

fn default_select() -> Vec<KeyBind> {
    vec![
        KeyBind::Simple("enter".to_string()),
        KeyBind::WithMod {
            key: "y".to_string(),
            modifiers: "ctrl".to_string(),
        },
    ]
}

fn default_exit() -> Vec<KeyBind> {
    vec![
        KeyBind::Simple("esc".to_string()),
        KeyBind::WithMod {
            key: "q".to_string(),
            modifiers: "ctrl".to_string(),
        },
        KeyBind::WithMod {
            key: "c".to_string(),
            modifiers: "ctrl".to_string(),
        },
    ]
}

fn default_pin() -> Vec<KeyBind> {
    vec![KeyBind::WithMod {
        key: "space".to_string(),
        modifiers: "ctrl".to_string(),
    }]
}

fn default_hide() -> Vec<KeyBind> {
    vec![KeyBind::WithMod {
        key: "delete".to_string(),
        modifiers: "alt".to_string(),
    }]
}

fn default_unhide_last() -> Vec<KeyBind> {
    vec![KeyBind::WithMod {
        key: "u".to_string(),
        modifiers: "alt".to_string(),
    }]
}

fn default_backspace() -> Vec<KeyBind> {
    vec![KeyBind::Simple("backspace".to_string())]
}

fn default_image_preview() -> Vec<KeyBind> {
    // Note: Ctrl+I is the same as Tab in terminals, so we use Alt+I instead
    vec![KeyBind::WithMod {
        key: "i".to_string(),
        modifiers: "alt".to_string(),
    }]
}

fn default_tag() -> Vec<KeyBind> {
    vec![KeyBind::WithMod {
        key: "t".to_string(),
        modifiers: "ctrl".to_string(),
    }]
}

fn default_cclip_delete() -> Vec<KeyBind> {
    vec![KeyBind::WithMod {
        key: "delete".to_string(),
        modifiers: "alt".to_string(),
    }]
}

impl Keybinds {
    pub(crate) fn select_hint(&self) -> Option<String> {
        self.select.first().map(KeyBind::hint)
    }

    pub(crate) fn exit_hint(&self) -> Option<String> {
        self.exit.first().map(KeyBind::hint)
    }

    pub fn matches_up(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.up.iter().any(|kb| kb.matches(code, mods))
    }

    pub fn matches_down(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.down.iter().any(|kb| kb.matches(code, mods))
    }

    pub fn matches_left(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.left.iter().any(|kb| kb.matches(code, mods))
    }

    pub fn matches_right(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.right.iter().any(|kb| kb.matches(code, mods))
    }

    pub fn matches_select(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.select.iter().any(|kb| kb.matches(code, mods))
    }

    pub fn matches_exit(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.exit.iter().any(|kb| kb.matches(code, mods))
    }

    pub fn matches_pin(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.pin.iter().any(|kb| kb.matches(code, mods))
    }

    pub fn matches_hide(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.hide.iter().any(|binding| binding.matches(code, mods))
    }

    pub fn matches_unhide_last(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.unhide_last
            .iter()
            .any(|binding| binding.matches(code, mods))
    }

    pub fn matches_backspace(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.backspace.iter().any(|kb| kb.matches(code, mods))
    }

    pub fn matches_image_preview(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.image_preview.iter().any(|kb| kb.matches(code, mods))
    }

    /// Tag keybind for cclip mode
    pub fn matches_tag(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.tag.iter().any(|kb| kb.matches(code, mods))
    }

    /// Tag-removal keybind for cclip mode using the configured tag key with `Alt`.
    pub fn matches_tag_removal(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        mods == KeyModifiers::ALT
            && self.tag.iter().any(|binding| match binding {
                KeyBind::Simple(key) => parse_key(key).0 == code,
                KeyBind::WithMod { key, modifiers } => {
                    parse_key(key).0 == code
                        && !parse_modifiers(modifiers).contains(KeyModifiers::ALT)
                }
            })
    }

    /// Delete keybind for cclip mode
    pub fn matches_cclip_delete(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.cclip_delete.iter().any(|kb| kb.matches(code, mods))
    }
}

#[cfg(test)]
mod tests {
    use super::{KeyBind, Keybinds};
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn alt_tag_binding_does_not_overlap_with_tag_removal() {
        let keybinds = Keybinds {
            tag: vec![KeyBind::WithMod {
                key: "t".to_string(),
                modifiers: "alt".to_string(),
            }],
            ..Keybinds::default()
        };

        assert!(keybinds.matches_tag(KeyCode::Char('t'), KeyModifiers::ALT));
        assert!(!keybinds.matches_tag_removal(KeyCode::Char('t'), KeyModifiers::ALT));
    }

    #[test]
    fn documented_tab_key_is_supported() {
        let keybinds: Keybinds = toml::from_str(r#"down = ["tab"]"#).unwrap();

        assert!(keybinds.matches_down(KeyCode::Tab, KeyModifiers::NONE));
    }

    #[test]
    fn default_launcher_hide_bindings_are_deliberate_chords() {
        let keybinds = Keybinds::default();

        assert!(keybinds.matches_hide(KeyCode::Delete, KeyModifiers::ALT));
        assert!(keybinds.matches_unhide_last(KeyCode::Char('u'), KeyModifiers::ALT));
        assert!(!keybinds.matches_hide(KeyCode::Delete, KeyModifiers::NONE));
    }

    #[test]
    fn combined_modifiers_are_matched_exactly() {
        let keybinds = Keybinds {
            down: vec![KeyBind::WithMod {
                key: "j".to_string(),
                modifiers: "ctrl+alt".to_string(),
            }],
            ..Keybinds::default()
        };

        assert!(keybinds.matches_down(
            KeyCode::Char('j'),
            KeyModifiers::CONTROL | KeyModifiers::ALT
        ));
        assert!(!keybinds.matches_down(KeyCode::Char('j'), KeyModifiers::ALT));
    }

    #[test]
    fn shifted_letters_match_uppercase_terminal_events() {
        let keybinds: Keybinds =
            toml::from_str(r#"down = [{ key = "j", modifiers = "shift" }]"#).unwrap();

        assert!(keybinds.matches_down(KeyCode::Char('J'), KeyModifiers::SHIFT));
    }

    #[test]
    fn shifted_tab_matches_crossterm_backtab_events() {
        let keybinds: Keybinds =
            toml::from_str(r#"up = [{ key = "tab", modifiers = "shift" }]"#).unwrap();

        assert!(keybinds.matches_up(KeyCode::BackTab, KeyModifiers::SHIFT));
        assert!(keybinds.matches_up(KeyCode::BackTab, KeyModifiers::NONE));
        assert!(!keybinds.matches_up(KeyCode::Tab, KeyModifiers::NONE));
    }

    #[test]
    fn footer_hints_follow_configured_bindings() {
        let keybinds: Keybinds = toml::from_str(
            r#"
select = [{ key = "y", modifiers = "ctrl" }]
exit = ["q"]
"#,
        )
        .unwrap();

        assert_eq!(keybinds.select_hint().as_deref(), Some("ctrl+y"));
        assert_eq!(keybinds.exit_hint().as_deref(), Some("q"));
    }
}
