use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum MatchMode {
    Exact,
    #[default]
    Fuzzy,
}

impl FromStr for MatchMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_lowercase().as_str() {
            "exact" => Ok(Self::Exact),
            "fuzzy" => Ok(Self::Fuzzy),
            _ => Err(format!("Invalid match mode: '{value}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum RankingMode {
    #[default]
    Frecency,
    Recency,
    Frequency,
}

impl RankingMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Frecency => "frecency",
            Self::Recency => "recency",
            Self::Frequency => "frequency",
        }
    }
}

impl FromStr for RankingMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_lowercase().as_str() {
            "frecency" => Ok(Self::Frecency),
            "recency" => Ok(Self::Recency),
            "frequency" => Ok(Self::Frequency),
            _ => Err(format!("Invalid ranking mode: '{value}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PinnedOrderMode {
    #[default]
    Ranking,
    Alphabetical,
    #[serde(alias = "oldest")]
    OldestPinned,
    #[serde(alias = "newest", alias = "last_pinned")]
    NewestPinned,
}

impl PinnedOrderMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ranking => "ranking",
            Self::Alphabetical => "alphabetical",
            Self::OldestPinned => "oldest_pinned",
            Self::NewestPinned => "newest_pinned",
        }
    }
}

impl FromStr for PinnedOrderMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_lowercase().as_str() {
            "ranking" => Ok(Self::Ranking),
            "alphabetical" => Ok(Self::Alphabetical),
            "oldest_pinned" | "oldest" => Ok(Self::OldestPinned),
            "newest_pinned" | "newest" | "last_pinned" => Ok(Self::NewestPinned),
            _ => Err(format!("Invalid pinned order mode: '{value}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum DesktopIconMode {
    None,
    #[default]
    Preview,
    List,
    Both,
}

impl DesktopIconMode {
    pub fn shows_preview(self) -> bool {
        matches!(self, Self::Preview | Self::Both)
    }

    pub fn shows_list(self) -> bool {
        matches!(self, Self::List | Self::Both)
    }
}

impl FromStr for DesktopIconMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_lowercase().as_str() {
            "none" | "no" | "off" | "false" => Ok(Self::None),
            "preview" | "yes" | "on" | "true" => Ok(Self::Preview),
            "list" => Ok(Self::List),
            "both" => Ok(Self::Both),
            _ => Err(format!(
                "Invalid desktop icon mode: '{value}'. Valid options: none, preview, list, both"
            )),
        }
    }
}
