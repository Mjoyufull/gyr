use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub(super) struct ThemeMetadata {
    pub(super) directories: Vec<ThemeDirectory>,
    pub(super) inherits: Vec<String>,
}

pub(super) struct ThemeDirectory {
    pub(super) path: PathBuf,
    size: Option<u16>,
    scale: u16,
    kind: DirectoryKind,
}

enum DirectoryKind {
    Fixed,
    Scalable { minimum: u16, maximum: u16 },
    Threshold { threshold: u16 },
}

impl ThemeDirectory {
    pub(super) fn score(&self, requested_size: u16) -> (u32, u8) {
        let Some(size) = self.size else {
            return (u32::MAX / 2, 3);
        };
        let scale = u32::from(self.scale.max(1));
        let requested = u32::from(requested_size);
        match self.kind {
            DirectoryKind::Fixed => (requested.abs_diff(u32::from(size) * scale), 0),
            DirectoryKind::Threshold { threshold } => {
                let minimum = u32::from(size.saturating_sub(threshold)) * scale;
                let maximum = u32::from(size.saturating_add(threshold)) * scale;
                (range_distance(requested, minimum, maximum), 1)
            }
            DirectoryKind::Scalable { minimum, maximum } => (
                range_distance(
                    requested,
                    u32::from(minimum) * scale,
                    u32::from(maximum) * scale,
                ),
                2,
            ),
        }
    }
}

#[derive(Default)]
struct DirectoryBuilder {
    size: Option<u16>,
    scale: Option<u16>,
    kind: Option<String>,
    minimum: Option<u16>,
    maximum: Option<u16>,
    threshold: Option<u16>,
}

pub(super) fn read_theme_metadata(theme_root: &Path) -> Option<ThemeMetadata> {
    let contents = fs::read_to_string(theme_root.join("index.theme")).ok()?;
    let mut section = String::new();
    let mut directory_names = Vec::new();
    let mut inherits = Vec::new();
    let mut builders: HashMap<String, DirectoryBuilder> = HashMap::new();

    for line in contents.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|line| line.strip_suffix(']'))
        {
            section = name.trim().to_string();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if section.eq_ignore_ascii_case("Icon Theme") {
            match key {
                "Directories" | "ScaledDirectories" => {
                    extend_list(&mut directory_names, value);
                }
                "Inherits" => extend_list(&mut inherits, value),
                _ => {}
            }
            continue;
        }

        let builder = builders.entry(section.clone()).or_default();
        match key {
            "Size" => builder.size = value.parse().ok(),
            "Scale" => builder.scale = value.parse().ok(),
            "Type" => builder.kind = Some(value.to_string()),
            "MinSize" => builder.minimum = value.parse().ok(),
            "MaxSize" => builder.maximum = value.parse().ok(),
            "Threshold" => builder.threshold = value.parse().ok(),
            _ => {}
        }
    }

    let directories = directory_names
        .into_iter()
        .map(|path| {
            let builder = builders.remove(&path).unwrap_or_default();
            let size = builder.size;
            let kind = match builder.kind.as_deref() {
                Some("Fixed") => DirectoryKind::Fixed,
                Some("Scalable") => DirectoryKind::Scalable {
                    minimum: builder.minimum.or(size).unwrap_or(0),
                    maximum: builder.maximum.or(size).unwrap_or(u16::MAX),
                },
                _ => DirectoryKind::Threshold {
                    threshold: builder.threshold.unwrap_or(2),
                },
            };
            ThemeDirectory {
                path: PathBuf::from(path),
                size,
                scale: builder.scale.unwrap_or(1).max(1),
                kind,
            }
        })
        .collect();

    Some(ThemeMetadata {
        directories,
        inherits,
    })
}

fn extend_list(values: &mut Vec<String>, source: &str) {
    values.extend(
        source
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(str::to_string),
    );
}

fn range_distance(value: u32, minimum: u32, maximum: u32) -> u32 {
    if value < minimum {
        minimum - value
    } else {
        value.saturating_sub(maximum)
    }
}

#[cfg(test)]
mod tests {
    use super::{DirectoryKind, ThemeDirectory};
    use std::path::PathBuf;

    #[test]
    fn exact_fixed_size_outranks_scalable_range() {
        let fixed = ThemeDirectory {
            path: PathBuf::from("128x128/apps"),
            size: Some(128),
            scale: 1,
            kind: DirectoryKind::Fixed,
        };
        let scalable = ThemeDirectory {
            path: PathBuf::from("scalable/apps"),
            size: Some(48),
            scale: 1,
            kind: DirectoryKind::Scalable {
                minimum: 16,
                maximum: 256,
            },
        };

        assert!(fixed.score(128) < scalable.score(128));
    }

    #[test]
    fn scale_is_applied_to_fixed_directory_size() {
        let high_dpi = ThemeDirectory {
            path: PathBuf::from("64x64@2/apps"),
            size: Some(64),
            scale: 2,
            kind: DirectoryKind::Fixed,
        };

        assert_eq!(high_dpi.score(128), (0, 0));
    }
}
