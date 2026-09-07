use jwalk::WalkDir;
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::{Path, PathBuf};

mod index;
mod theme;

use index::{ThemeDirectory, read_theme_metadata};
use theme::detect_icon_theme;

const ICON_EXTENSIONS: [&str; 4] = ["png", "svg", "svgz", "xpm"];
const MAX_THEME_DEPTH: usize = 16;

/// Resolves desktop-entry icon names through the active XDG icon theme.
pub(crate) struct IconResolver {
    theme: String,
    size: u16,
    icon_roots: Vec<PathBuf>,
    pixmap_roots: Vec<PathBuf>,
    cache: HashMap<String, Option<PathBuf>>,
}

impl IconResolver {
    /// Build a resolver from the process environment and optional configured theme.
    pub(crate) fn from_environment(theme: Option<&str>, size: u16) -> Self {
        let home = directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf());
        let config_home = env::var_os("XDG_CONFIG_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| home.as_ref().map(|path| path.join(".config")));
        let data_home = env::var_os("XDG_DATA_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| home.as_ref().map(|path| path.join(".local/share")));

        let mut icon_roots = Vec::new();
        if let Some(data_home) = data_home {
            push_unique(&mut icon_roots, data_home.join("icons"));
        }
        if let Some(home) = &home {
            push_unique(&mut icon_roots, home.join(".icons"));
        }

        let mut pixmap_roots = Vec::new();
        let data_dirs = env::var("XDG_DATA_DIRS").ok();
        for data_dir in super::dirs::system_data_dirs(data_dirs.as_deref()) {
            push_unique(&mut icon_roots, data_dir.join("icons"));
            push_unique(&mut pixmap_roots, data_dir.join("pixmaps"));
        }

        let config_dirs = env::var("XDG_CONFIG_DIRS")
            .ok()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "/etc/xdg".to_string())
            .split(':')
            .filter(|entry| !entry.is_empty())
            .map(PathBuf::from)
            .collect::<Vec<_>>();

        let theme = theme
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string)
            .or_else(|| detect_icon_theme(config_home.as_deref(), home.as_deref(), &config_dirs))
            .unwrap_or_else(|| "hicolor".to_string());

        Self {
            theme,
            size,
            icon_roots,
            pixmap_roots,
            cache: HashMap::new(),
        }
    }

    /// Resolve an absolute path or themed icon name to an existing image file.
    pub(crate) fn resolve(&mut self, icon: &str) -> Option<PathBuf> {
        if let Some(path) = absolute_icon_path(icon) {
            return Some(path);
        }
        if let Some(cached) = self.cache.get(icon) {
            return cached.clone();
        }
        if icon.contains(['/', '\\']) {
            self.cache.insert(icon.to_string(), None);
            return None;
        }

        let icon_name = strip_icon_extension(icon);
        let themes = self.theme_chain();
        let resolved = themes
            .iter()
            .find_map(|theme| self.find_declared_in_theme(theme, icon_name))
            .or_else(|| self.find_unthemed(icon_name))
            .or_else(|| {
                themes
                    .iter()
                    .find_map(|theme| self.find_fallback_in_theme(theme, icon_name))
            });
        self.cache.insert(icon.to_string(), resolved.clone());
        resolved
    }

    fn theme_chain(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut themes = Vec::new();
        self.append_theme_subtree(&self.theme, &mut seen, &mut themes);
        if seen.insert("hicolor".to_string()) {
            // The required fallback sits outside the inherited-theme depth cap.
            themes.push("hicolor".to_string());
        }
        themes
    }

    fn append_theme_subtree(
        &self,
        theme: &str,
        seen: &mut HashSet<String>,
        themes: &mut Vec<String>,
    ) {
        if themes.len() >= MAX_THEME_DEPTH || !seen.insert(theme.to_string()) {
            return;
        }
        themes.push(theme.to_string());

        if let Some(metadata) = self
            .icon_roots
            .iter()
            .find_map(|root| read_theme_metadata(&root.join(theme)))
        {
            for inherited in metadata.inherits {
                self.append_theme_subtree(&inherited, seen, themes);
            }
        }
    }

    fn find_declared_in_theme(&self, theme: &str, icon: &str) -> Option<PathBuf> {
        let mut candidates = Vec::new();
        for (root_rank, root) in self.icon_roots.iter().enumerate() {
            let theme_root = root.join(theme);
            if !theme_root.is_dir() {
                continue;
            }

            if let Some(metadata) = read_theme_metadata(&theme_root) {
                for directory in metadata.directories {
                    collect_named_candidates(
                        &theme_root,
                        &directory,
                        icon,
                        self.size,
                        root_rank,
                        &mut candidates,
                    );
                }
            }
        }
        best_candidate(candidates)
    }

    fn find_fallback_in_theme(&self, theme: &str, icon: &str) -> Option<PathBuf> {
        let mut candidates = Vec::new();
        for (root_rank, root) in self.icon_roots.iter().enumerate() {
            let theme_root = root.join(theme);
            if !theme_root.is_dir() {
                continue;
            }
            for entry in WalkDir::new(&theme_root)
                .min_depth(1)
                .max_depth(5)
                .into_iter()
                .filter_map(Result::ok)
            {
                let path = entry.path();
                if path.is_file() && has_icon_name(&path, icon) {
                    candidates.push(IconCandidate::from_fallback(path, self.size, root_rank));
                }
            }
        }
        best_candidate(candidates)
    }

    fn find_unthemed(&self, icon: &str) -> Option<PathBuf> {
        for root in self.icon_roots.iter().chain(&self.pixmap_roots) {
            for extension in ICON_EXTENSIONS {
                let path = root.join(format!("{icon}.{extension}"));
                if path.is_file() {
                    return Some(path);
                }
            }
        }
        None
    }
}

fn collect_named_candidates(
    theme_root: &Path,
    directory: &ThemeDirectory,
    icon: &str,
    requested_size: u16,
    root_rank: usize,
    candidates: &mut Vec<IconCandidate>,
) {
    for extension in ICON_EXTENSIONS {
        let path = theme_root
            .join(&directory.path)
            .join(format!("{icon}.{extension}"));
        if path.is_file() {
            candidates.push(IconCandidate {
                path,
                directory_score: directory.score(requested_size),
                root_rank,
            });
        }
    }
}

#[derive(Clone)]
struct IconCandidate {
    path: PathBuf,
    directory_score: (u32, u8),
    root_rank: usize,
}

impl IconCandidate {
    fn from_fallback(path: PathBuf, requested_size: u16, root_rank: usize) -> Self {
        let distance = path
            .components()
            .filter_map(|component| component.as_os_str().to_str())
            .find_map(parse_directory_size)
            .map_or(u32::MAX / 2, |size| {
                u32::from(size.abs_diff(requested_size))
            });
        Self {
            path,
            directory_score: (distance, 3),
            root_rank,
        }
    }

    fn score(&self) -> (u32, u8, usize, u8) {
        let (distance, kind_rank) = self.directory_score;
        (
            distance,
            kind_rank,
            self.root_rank,
            extension_rank(&self.path),
        )
    }
}

fn best_candidate(mut candidates: Vec<IconCandidate>) -> Option<PathBuf> {
    candidates.sort_by_key(IconCandidate::score);
    candidates
        .into_iter()
        .next()
        .map(|candidate| candidate.path)
}

fn extension_rank(path: &Path) -> u8 {
    match path.extension().and_then(|value| value.to_str()) {
        Some("png") => 0,
        Some("svg" | "svgz") => 1,
        _ => 2,
    }
}

fn parse_directory_size(component: &str) -> Option<u16> {
    let leading = component.split('x').next()?;
    leading.parse().ok()
}

fn has_icon_name(path: &Path, icon: &str) -> bool {
    path.file_stem().and_then(|stem| stem.to_str()) == Some(icon)
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| ICON_EXTENSIONS.contains(&extension))
}

fn strip_icon_extension(icon: &str) -> &str {
    ICON_EXTENSIONS
        .iter()
        .find_map(|extension| icon.strip_suffix(&format!(".{extension}")))
        .unwrap_or(icon)
}

fn absolute_icon_path(icon: &str) -> Option<PathBuf> {
    let path = PathBuf::from(icon);
    (path.is_absolute() && path.is_file()).then_some(path)
}

fn push_unique(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.contains(&path) {
        paths.push(path);
    }
}

#[cfg(test)]
mod tests {
    use super::IconResolver;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should follow the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("fsel-icons-{unique}"));
        fs::create_dir_all(&path).expect("temporary icon root should be created");
        path
    }

    #[test]
    fn resolves_inherited_theme_icons_at_the_requested_size() {
        let root = temp_dir();
        let selected_theme = root.join("Selected");
        let inherited_theme = root.join("Inherited");
        fs::create_dir_all(&selected_theme).expect("selected theme should be created");
        fs::write(
            selected_theme.join("index.theme"),
            "[Icon Theme]\nInherits=Inherited\n",
        )
        .expect("selected theme metadata should be written");
        fs::create_dir_all(inherited_theme.join("32x32/apps"))
            .expect("32px directory should be created");
        fs::create_dir_all(inherited_theme.join("128x128/apps"))
            .expect("128px directory should be created");
        fs::write(
            inherited_theme.join("index.theme"),
            "[Icon Theme]\nDirectories=32x32/apps,128x128/apps\n\
             [32x32/apps]\nSize=32\nType=Fixed\n\
             [128x128/apps]\nSize=128\nType=Fixed\n",
        )
        .expect("inherited theme metadata should be written");
        fs::write(inherited_theme.join("32x32/apps/editor.png"), b"small")
            .expect("small icon should be written");
        let expected = inherited_theme.join("128x128/apps/editor.png");
        fs::write(&expected, b"large").expect("large icon should be written");

        let mut resolver = IconResolver {
            theme: "Selected".to_string(),
            size: 128,
            icon_roots: vec![root.clone()],
            pixmap_roots: Vec::new(),
            cache: std::collections::HashMap::new(),
        };

        assert_eq!(resolver.resolve("editor"), Some(expected));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn declared_inherited_icon_precedes_selected_theme_directory_scan() {
        let root = temp_dir();
        let selected_theme = root.join("Selected");
        let inherited_theme = root.join("Inherited");
        fs::create_dir_all(selected_theme.join("undeclared/apps"))
            .expect("undeclared directory should be created");
        fs::write(
            selected_theme.join("index.theme"),
            "[Icon Theme]\nInherits=Inherited\n",
        )
        .expect("selected theme metadata should be written");
        fs::write(
            selected_theme.join("undeclared/apps/editor.png"),
            b"fallback",
        )
        .expect("fallback icon should be written");
        fs::create_dir_all(inherited_theme.join("64x64/apps"))
            .expect("inherited directory should be created");
        fs::write(
            inherited_theme.join("index.theme"),
            "[Icon Theme]\nDirectories=64x64/apps\n[64x64/apps]\nSize=64\nType=Fixed\n",
        )
        .expect("inherited theme metadata should be written");
        let expected = inherited_theme.join("64x64/apps/editor.png");
        fs::write(&expected, b"declared").expect("declared icon should be written");

        let mut resolver = IconResolver {
            theme: "Selected".to_string(),
            size: 64,
            icon_roots: vec![root.clone()],
            pixmap_roots: Vec::new(),
            cache: std::collections::HashMap::new(),
        };

        assert_eq!(resolver.resolve("editor"), Some(expected));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn exact_fixed_icon_outranks_scalable_svg() {
        let root = temp_dir();
        let theme = root.join("Selected");
        fs::create_dir_all(theme.join("scalable/apps"))
            .expect("scalable directory should be created");
        fs::create_dir_all(theme.join("128x128/apps")).expect("fixed directory should be created");
        fs::write(
            theme.join("index.theme"),
            "[Icon Theme]\nDirectories=scalable/apps,128x128/apps\n\
             [scalable/apps]\nSize=48\nType=Scalable\nMinSize=16\nMaxSize=256\n\
             [128x128/apps]\nSize=128\nType=Fixed\n",
        )
        .expect("theme metadata should be written");
        fs::write(theme.join("scalable/apps/editor.svg"), b"scalable")
            .expect("scalable icon should be written");
        let expected = theme.join("128x128/apps/editor.png");
        fs::write(&expected, b"fixed").expect("fixed icon should be written");

        let mut resolver = IconResolver {
            theme: "Selected".to_string(),
            size: 128,
            icon_roots: vec![root.clone()],
            pixmap_roots: Vec::new(),
            cache: std::collections::HashMap::new(),
        };

        assert_eq!(resolver.resolve("editor"), Some(expected));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn png_outranks_svg_at_the_same_theme_size() {
        let root = temp_dir();
        let theme = root.join("Selected");
        fs::create_dir_all(theme.join("64x64/apps")).expect("fixed directory should be created");
        fs::write(
            theme.join("index.theme"),
            "[Icon Theme]\nDirectories=64x64/apps\n[64x64/apps]\nSize=64\nType=Fixed\n",
        )
        .expect("theme metadata should be written");
        fs::write(theme.join("64x64/apps/editor.svg"), b"svg").expect("SVG icon should be written");
        let expected = theme.join("64x64/apps/editor.png");
        fs::write(&expected, b"png").expect("PNG icon should be written");
        let mut resolver = IconResolver {
            theme: "Selected".to_string(),
            size: 64,
            icon_roots: vec![root.clone()],
            pixmap_roots: Vec::new(),
            cache: std::collections::HashMap::new(),
        };

        assert_eq!(resolver.resolve("editor"), Some(expected));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn higher_priority_root_outranks_extension_preference() {
        let root = temp_dir();
        let user_root = root.join("user");
        let system_root = root.join("system");
        for icon_root in [&user_root, &system_root] {
            let theme = icon_root.join("Selected");
            fs::create_dir_all(theme.join("128x128/apps"))
                .expect("theme directory should be created");
            fs::write(
                theme.join("index.theme"),
                "[Icon Theme]\nDirectories=128x128/apps\n[128x128/apps]\nSize=128\nType=Fixed\n",
            )
            .expect("theme metadata should be written");
        }
        let expected = user_root.join("Selected/128x128/apps/editor.svg");
        fs::write(&expected, b"user SVG").expect("user icon should be written");
        fs::write(
            system_root.join("Selected/128x128/apps/editor.png"),
            b"system PNG",
        )
        .expect("system icon should be written");
        let mut resolver = IconResolver {
            theme: "Selected".to_string(),
            size: 128,
            icon_roots: vec![user_root, system_root],
            pixmap_roots: Vec::new(),
            cache: std::collections::HashMap::new(),
        };

        assert_eq!(resolver.resolve("editor"), Some(expected));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resolves_compressed_svg_theme_icons() {
        let root = temp_dir();
        let theme = root.join("Selected");
        fs::create_dir_all(theme.join("scalable/apps"))
            .expect("scalable directory should be created");
        fs::write(
            theme.join("index.theme"),
            "[Icon Theme]\nDirectories=scalable/apps\n[scalable/apps]\nSize=64\nType=Scalable\nMinSize=16\nMaxSize=256\n",
        )
        .expect("theme metadata should be written");
        let expected = theme.join("scalable/apps/editor.svgz");
        fs::write(&expected, b"compressed svg").expect("SVGZ icon should be written");
        let mut resolver = IconResolver {
            theme: "Selected".to_string(),
            size: 64,
            icon_roots: vec![root.clone()],
            pixmap_roots: Vec::new(),
            cache: std::collections::HashMap::new(),
        };

        assert_eq!(resolver.resolve("editor"), Some(expected));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn first_theme_metadata_controls_inheritance() {
        let root = temp_dir();
        let user_root = root.join("user");
        let system_root = root.join("system");
        fs::create_dir_all(user_root.join("Selected")).expect("user theme should be created");
        fs::create_dir_all(system_root.join("Selected")).expect("system theme should be created");
        fs::write(
            user_root.join("Selected/index.theme"),
            "[Icon Theme]\nInherits=UserParent\n",
        )
        .expect("user theme metadata should be written");
        fs::write(
            system_root.join("Selected/index.theme"),
            "[Icon Theme]\nInherits=SystemParent\n",
        )
        .expect("system theme metadata should be written");
        for parent in ["UserParent", "SystemParent"] {
            let theme = root.join(parent).join("64x64/apps");
            fs::create_dir_all(&theme).expect("parent theme should be created");
            fs::write(
                root.join(parent).join("index.theme"),
                "[Icon Theme]\nDirectories=64x64/apps\n[64x64/apps]\nSize=64\nType=Fixed\n",
            )
            .expect("parent metadata should be written");
            fs::write(theme.join("editor.png"), parent.as_bytes())
                .expect("parent icon should be written");
        }
        let expected = root.join("UserParent/64x64/apps/editor.png");
        let mut resolver = IconResolver {
            theme: "Selected".to_string(),
            size: 64,
            icon_roots: vec![user_root, system_root, root.clone()],
            pixmap_roots: Vec::new(),
            cache: std::collections::HashMap::new(),
        };

        assert_eq!(resolver.resolve("editor"), Some(expected));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn chooses_best_theme_candidate_across_icon_roots() {
        let root = temp_dir();
        let user_root = root.join("user");
        let system_root = root.join("system");
        for (icon_root, size) in [(&user_root, 16), (&system_root, 128)] {
            let theme = icon_root.join("Selected");
            fs::create_dir_all(theme.join(format!("{size}x{size}/apps")))
                .expect("theme directory should be created");
            fs::write(
                theme.join("index.theme"),
                format!(
                    "[Icon Theme]\nDirectories={size}x{size}/apps\n[{size}x{size}/apps]\nSize={size}\nType=Fixed\n"
                ),
            )
            .expect("theme metadata should be written");
            fs::write(
                theme.join(format!("{size}x{size}/apps/editor.png")),
                b"icon",
            )
            .expect("icon should be written");
        }
        let expected = system_root.join("Selected/128x128/apps/editor.png");
        let mut resolver = IconResolver {
            theme: "Selected".to_string(),
            size: 128,
            icon_roots: vec![user_root, system_root],
            pixmap_roots: Vec::new(),
            cache: std::collections::HashMap::new(),
        };

        assert_eq!(resolver.resolve("editor"), Some(expected));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn inherited_themes_are_traversed_depth_first() {
        let root = temp_dir();
        for (theme, inherits) in [("Selected", "A,B"), ("A", "C"), ("B", ""), ("C", "")] {
            let theme_root = root.join(theme);
            fs::create_dir_all(theme_root.join("64x64/apps"))
                .expect("theme directory should be created");
            fs::write(
                theme_root.join("index.theme"),
                format!(
                    "[Icon Theme]\nInherits={inherits}\nDirectories=64x64/apps\n[64x64/apps]\nSize=64\nType=Fixed\n"
                ),
            )
            .expect("theme metadata should be written");
        }
        fs::write(root.join("B/64x64/apps/editor.png"), b"sibling")
            .expect("sibling icon should be written");
        let expected = root.join("C/64x64/apps/editor.png");
        fs::write(&expected, b"descendant").expect("descendant icon should be written");
        let mut resolver = IconResolver {
            theme: "Selected".to_string(),
            size: 64,
            icon_roots: vec![root.clone()],
            pixmap_roots: Vec::new(),
            cache: std::collections::HashMap::new(),
        };

        assert_eq!(resolver.resolve("editor"), Some(expected));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn hicolor_is_appended_after_the_inheritance_depth_cap() {
        let root = temp_dir();
        for index in 0..super::MAX_THEME_DEPTH {
            let theme = root.join(format!("Theme{index}"));
            fs::create_dir_all(&theme).expect("theme should be created");
            let inherits = if index + 1 < super::MAX_THEME_DEPTH {
                format!("Theme{}", index + 1)
            } else {
                String::new()
            };
            fs::write(
                theme.join("index.theme"),
                format!("[Icon Theme]\nInherits={inherits}\n"),
            )
            .expect("theme metadata should be written");
        }
        let resolver = IconResolver {
            theme: "Theme0".to_string(),
            size: 64,
            icon_roots: vec![root.clone()],
            pixmap_roots: Vec::new(),
            cache: std::collections::HashMap::new(),
        };

        let chain = resolver.theme_chain();
        assert_eq!(chain.len(), super::MAX_THEME_DEPTH + 1);
        assert_eq!(chain.last().map(String::as_str), Some("hicolor"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn finds_unthemed_icons_in_icon_roots() {
        let root = temp_dir();
        let expected = root.join("editor.png");
        fs::write(&expected, b"icon").expect("unthemed icon should be written");
        let mut resolver = IconResolver {
            theme: "Missing".to_string(),
            size: 64,
            icon_roots: vec![root.clone()],
            pixmap_roots: Vec::new(),
            cache: std::collections::HashMap::new(),
        };

        assert_eq!(resolver.resolve("editor"), Some(expected));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn finds_xpm_icons_in_pixmap_roots() {
        let root = temp_dir();
        let expected = root.join("editor.xpm");
        fs::write(&expected, b"XPM icon").expect("XPM icon should be written");
        let mut resolver = IconResolver {
            theme: "Missing".to_string(),
            size: 64,
            icon_roots: Vec::new(),
            pixmap_roots: vec![root.clone()],
            cache: std::collections::HashMap::new(),
        };

        assert_eq!(resolver.resolve("editor"), Some(expected));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_relative_icon_paths() {
        let mut resolver = IconResolver {
            theme: "hicolor".to_string(),
            size: 64,
            icon_roots: Vec::new(),
            pixmap_roots: Vec::new(),
            cache: std::collections::HashMap::new(),
        };

        assert_eq!(resolver.resolve("../outside"), None);
        assert_eq!(resolver.resolve("folder/icon"), None);
        assert_eq!(resolver.resolve("folder\\icon"), None);
    }

    #[test]
    fn absolute_paths_bypass_theme_lookup() {
        let root = temp_dir();
        let icon = root.join("custom.png");
        fs::write(&icon, b"icon").expect("absolute icon should be written");
        let mut resolver = IconResolver {
            theme: "hicolor".to_string(),
            size: 64,
            icon_roots: Vec::new(),
            pixmap_roots: Vec::new(),
            cache: std::collections::HashMap::new(),
        };

        assert_eq!(resolver.resolve(icon.to_str().unwrap()), Some(icon.clone()));
        let _ = fs::remove_dir_all(root);
    }
}
