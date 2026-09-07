use crate::core::ranking::ScoreBreakdown;
use ratatui::widgets::ListItem;
use std::fmt;
use std::path::{Path, PathBuf};

mod dirs;
mod discover;
mod icons;
mod parse;

pub(crate) use dirs::application_dirs;
pub(crate) use discover::desktop_file_id;
pub use discover::{DiscoverOptions, read_with_options};
pub(crate) use icons::IconResolver;

/// An XDG Specification app with full desktop-entry metadata.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct App {
    /// App name (Name field).
    pub name: String,
    /// Command to run (Exec field).
    pub command: String,
    /// App description/comment (Comment field).
    pub description: String,
    /// Generic name of application (GenericName field).
    pub generic_name: Option<String>,
    /// Keywords for searching (Keywords field).
    pub keywords: Vec<String>,
    /// Categories this application belongs to (Categories field).
    pub categories: Vec<String>,
    /// MIME types this application can handle (MimeType field).
    pub mime_types: Vec<String>,
    /// Icon name or path (Icon field).
    pub icon: Option<String>,
    /// Run in terminal (Terminal field).
    pub is_terminal: bool,
    /// Path from which to run the command (Path field).
    pub path: Option<String>,
    /// Show only in these DEs (OnlyShowIn field).
    pub only_show_in: Vec<String>,
    /// Hide in these DEs (NotShowIn field).
    pub not_show_in: Vec<String>,
    /// Whether the app is hidden (Hidden field).
    pub hidden: bool,
    /// Application startup notification (StartupNotify field).
    pub startup_notify: bool,
    /// WM class for startup notification (StartupWMClass field).
    pub startup_wm_class: Option<String>,
    /// Command to test if executable exists (TryExec field).
    pub try_exec: Option<String>,
    /// Desktop Entry type (usually "Application").
    pub entry_type: String,
    /// Desktop file ID for tracking.
    pub desktop_id: Option<String>,
    #[serde(skip)]
    pub(crate) source_path: Option<PathBuf>,

    /// Matching score (used in UI).
    pub score: i64,
    /// Number of times this app was run.
    pub history: u64,
    /// Whether this app is pinned/favorited.
    pub pinned: bool,
    /// Last access timestamp (Unix epoch seconds).
    pub last_access: Option<u64>,
    /// Detailed score breakdown for debugging (`-T`).
    pub breakdown: Option<ScoreBreakdown>,

    #[doc(hidden)]
    actions: Option<Vec<String>>,
}

impl App {
    /// Returns a corrected score that blends history and matching score.
    pub fn corrected_score(&self) -> i64 {
        let history = i64::try_from(self.history).unwrap_or(i64::MAX);

        if self.history < 1 {
            self.score
        } else if self.score < 1 {
            history
        } else {
            self.score.saturating_mul(history)
        }
    }

    pub(crate) fn entry_key(&self) -> Option<crate::core::hidden_entries::EntryKey> {
        let source_path = self.source_path.as_deref()?;
        Some(match self.desktop_id.as_deref() {
            Some(desktop_id) => {
                crate::core::hidden_entries::EntryKey::desktop(source_path, desktop_id)
            }
            None => crate::core::hidden_entries::EntryKey::executable(source_path),
        })
    }

    pub(crate) fn source_display(&self) -> Option<String> {
        self.source_path
            .as_deref()
            .map(|path| path.to_string_lossy().into_owned())
    }

    pub(crate) fn set_source_path(&mut self, source_path: &Path) {
        self.source_path = Some(source_path.to_path_buf());
    }

    pub(crate) fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }
}

impl PartialEq for App {
    fn eq(&self, other: &Self) -> bool {
        self.pinned == other.pinned
            && self.corrected_score() == other.corrected_score()
            && self.name.eq_ignore_ascii_case(&other.name)
    }
}

impl Eq for App {}

impl Ord for App {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self.pinned, other.pinned) {
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            _ => {}
        }

        self.corrected_score()
            .cmp(&other.corrected_score())
            .reverse()
            .then(self.name.to_lowercase().cmp(&other.name.to_lowercase()))
    }
}

impl PartialOrd for App {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for App {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl AsRef<str> for App {
    fn as_ref(&self) -> &str {
        self.name.as_ref()
    }
}

impl<'a> From<App> for ListItem<'a> {
    fn from(app: App) -> ListItem<'a> {
        ListItem::new(app.name)
    }
}

impl<'a> From<&'a App> for ListItem<'a> {
    fn from(app: &'a App) -> ListItem<'a> {
        ListItem::new(app.name.clone())
    }
}

#[derive(Default)]
struct Action {
    name: String,
    from: String,
}

impl Action {
    fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    fn from(mut self, from: impl Into<String>) -> Self {
        self.from = from.into();
        self
    }
}
