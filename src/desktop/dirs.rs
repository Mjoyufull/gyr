use std::path::{Path, PathBuf};

/// Returns the XDG application directories that currently exist on disk.
pub(crate) fn application_dirs() -> Vec<PathBuf> {
    let xdg_data_home = std::env::var_os("XDG_DATA_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    let home_dir = directories::UserDirs::new().map(|dirs| dirs.home_dir().to_path_buf());
    let xdg_data_dirs = std::env::var("XDG_DATA_DIRS").ok();

    application_dirs_from_sources(
        xdg_data_home.as_deref(),
        home_dir.as_deref(),
        xdg_data_dirs.as_deref(),
    )
}

fn application_dirs_from_sources(
    xdg_data_home: Option<&Path>,
    home_dir: Option<&Path>,
    xdg_data_dirs: Option<&str>,
) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(xdg_data_home) = xdg_data_home {
        push_applications_dir(&mut dirs, xdg_data_home);
    } else if let Some(home_dir) = home_dir {
        push_applications_dir(&mut dirs, home_dir.join(".local/share"));
    }

    for data_dir in system_data_dirs(xdg_data_dirs) {
        push_applications_dir(&mut dirs, data_dir);
    }

    dirs
}

pub(crate) fn system_data_dirs(value: Option<&str>) -> Vec<PathBuf> {
    if let Some(value) = value.filter(|value| !value.is_empty()) {
        return value
            .split(':')
            .filter(|entry| !entry.is_empty())
            .map(PathBuf::from)
            .collect();
    }

    let paths = vec![
        PathBuf::from("/usr/local/share"),
        PathBuf::from("/usr/share"),
    ];
    #[cfg(target_os = "openbsd")]
    let paths = {
        let mut paths = paths;
        paths.push(PathBuf::from("/usr/X11R6/share"));
        paths
    };
    paths
}

fn push_applications_dir(dirs: &mut Vec<PathBuf>, base_dir: impl AsRef<Path>) {
    let mut applications_dir = base_dir.as_ref().to_path_buf();
    applications_dir.push("applications");

    if applications_dir.exists() && !dirs.contains(&applications_dir) {
        dirs.push(applications_dir);
    }
}

#[cfg(test)]
mod tests {
    use super::{application_dirs_from_sources, system_data_dirs};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "fsel-desktop-dirs-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("test dir should be created");
        dir
    }

    fn create_app_dir(base: &Path) -> PathBuf {
        let applications_dir = base.join("applications");
        fs::create_dir_all(&applications_dir).expect("applications dir should be created");
        applications_dir
    }

    #[test]
    fn uses_xdg_data_home_before_home_fallback() {
        let root = test_dir("xdg-home");
        let xdg_home = root.join("xdg-home");
        let home_dir = root.join("home");
        let expected = create_app_dir(&xdg_home);
        create_app_dir(&home_dir.join(".local/share"));

        let dirs = application_dirs_from_sources(Some(&xdg_home), Some(&home_dir), Some(""));

        assert_eq!(dirs.first(), Some(&expected));
        assert!(!dirs.contains(&home_dir.join(".local/share/applications")));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn falls_back_to_home_and_collects_system_dirs() {
        let root = test_dir("fallback");
        let home_dir = root.join("home");
        let system_one = root.join("system-one");
        let system_two = root.join("system-two");

        let home_expected = create_app_dir(&home_dir.join(".local/share"));
        let system_one_expected = create_app_dir(&system_one);
        let system_two_expected = create_app_dir(&system_two);
        let system_dirs = format!("{}:{}", system_one.display(), system_two.display());

        let dirs = application_dirs_from_sources(None, Some(&home_dir), Some(system_dirs.as_str()));

        assert_eq!(
            dirs,
            vec![home_expected, system_one_expected, system_two_expected]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn empty_system_data_dirs_uses_platform_defaults() {
        let dirs = system_data_dirs(Some(""));

        assert!(dirs.contains(&PathBuf::from("/usr/local/share")));
        assert!(dirs.contains(&PathBuf::from("/usr/share")));
        #[cfg(target_os = "openbsd")]
        assert!(dirs.contains(&PathBuf::from("/usr/X11R6/share")));
    }
}
