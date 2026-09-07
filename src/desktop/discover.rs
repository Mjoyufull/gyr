use super::{Action, App};
use crate::core::cache::HistoryCache;
use jwalk::WalkDir;
use rayon::prelude::*;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

/// Options that control desktop application discovery.
#[derive(Clone, Copy, Debug, Default)]
pub struct DiscoverOptions {
    /// Respect OnlyShowIn/NotShowIn fields for the current desktop.
    pub filter_desktop: bool,
    /// Hide desktop action entries such as "New Window".
    pub filter_actions: bool,
    /// Include raw executables discovered from `$PATH`.
    pub list_executables: bool,
    /// Include `Hidden=true` tombstones for automatic duplicate resolution.
    pub auto_hide_duplicates: bool,
}

fn current_desktop(filter_desktop: bool) -> Option<Vec<String>> {
    if !filter_desktop {
        return None;
    }

    env::var("XDG_CURRENT_DESKTOP")
        .ok()
        .map(|desktop| desktop.split(':').map(|part| part.to_string()).collect())
}

fn walk_desktop_files(dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut desktop_files = Vec::new();
    for dir in dirs {
        for entry in WalkDir::new(dir)
            .skip_hidden(false)
            .min_depth(1)
            .max_depth(5)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| {
                !entry.file_type().is_dir()
                    && entry.path().extension().and_then(|ext| ext.to_str()) == Some("desktop")
            })
        {
            desktop_files.push(entry.path().to_path_buf());
        }
    }
    desktop_files
}

fn load_desktop_files(
    dirs: &[PathBuf],
    desktop_cache: Option<&crate::core::cache::DesktopCache>,
) -> Vec<PathBuf> {
    if let Some(cache) = desktop_cache {
        match cache.get_file_list(dirs) {
            Ok(Some(cached_paths)) => cached_paths,
            _ => {
                let desktop_files = walk_desktop_files(dirs);
                let _ = cache.set_file_list(desktop_files.clone(), dirs);
                desktop_files
            }
        }
    } else {
        walk_desktop_files(dirs)
    }
}

fn attach_desktop_id(
    app: &mut App,
    application_dirs: &[PathBuf],
    file_path: &Path,
    suffix: Option<&str>,
) {
    app.set_source_path(file_path);
    if let Some(desktop_id) = desktop_file_id(application_dirs, file_path) {
        app.desktop_id = Some(match suffix {
            Some(suffix) => format!("{desktop_id}#{suffix}"),
            None => desktop_id,
        });
    }
}

pub(crate) fn desktop_file_id(application_dirs: &[PathBuf], file_path: &Path) -> Option<String> {
    let relative_path = application_dirs
        .iter()
        .find_map(|root| file_path.strip_prefix(root).ok())?;
    Some(
        relative_path
            .components()
            .map(|component| desktop_id_component(component.as_os_str()))
            .collect::<Vec<_>>()
            .join("-"),
    )
}

fn desktop_id_component(component: &std::ffi::OsStr) -> String {
    if let Some(component) = component.to_str() {
        return component.replace('%', "%25");
    }

    format!("%00{}", crate::core::path_key::encode(Path::new(component)))
}

fn load_app_from_path(
    file_path: &Path,
    desktop_cache: Option<&crate::core::cache::DesktopCache>,
    filter_desktop: bool,
    include_hidden: bool,
) -> Option<(App, Option<String>)> {
    if let Some(cache) = desktop_cache
        && let Ok(Some(cached_app)) = cache.get(file_path)
        && (include_hidden || !cached_app.hidden)
    {
        return Some((cached_app, None));
    }

    let contents = fs::read_to_string(file_path).ok()?;
    if !contents.contains("[Desktop Entry]") {
        return None;
    }

    let app = if include_hidden {
        App::parse_including_hidden(&contents, filter_desktop).ok()?
    } else {
        App::parse(&contents, filter_desktop).ok()?
    };
    Some((app, Some(contents)))
}

fn should_keep_for_desktop(app: &App, current_desktop: Option<&[String]>) -> bool {
    let Some(desktops) = current_desktop else {
        return true;
    };

    if app.not_show_in.iter().any(|desktop| {
        desktops
            .iter()
            .any(|current| current.eq_ignore_ascii_case(desktop))
    }) {
        return false;
    }

    if app.only_show_in.is_empty() {
        return true;
    }

    app.only_show_in.iter().any(|desktop| {
        desktops
            .iter()
            .any(|current| current.eq_ignore_ascii_case(desktop))
    })
}

fn executable_app(path: &Path, file_name: &str) -> App {
    App {
        name: file_name.to_string(),
        command: path.to_string_lossy().to_string(),
        description: format!("Executable: {file_name}"),
        generic_name: None,
        keywords: vec![],
        categories: vec!["Executable".to_string()],
        mime_types: vec![],
        icon: None,
        is_terminal: false,
        path: None,
        only_show_in: vec![],
        not_show_in: vec![],
        hidden: false,
        startup_notify: false,
        startup_wm_class: None,
        try_exec: None,
        entry_type: "Application".to_string(),
        desktop_id: None,
        source_path: Some(path.to_path_buf()),
        history: 0,
        score: 0,
        pinned: false,
        last_access: None,
        breakdown: None,
        actions: None,
    }
}

fn send_path_executables(sender: &mpsc::Sender<App>, history_cache: &HistoryCache) -> Option<()> {
    let path_var = env::var("PATH").ok()?;
    let mut seen_executables = HashSet::new();

    for path_dir in path_var.split(':') {
        let Ok(entries) = fs::read_dir(path_dir) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                if let Ok(metadata) = fs::metadata(&path) {
                    let permissions = metadata.permissions();
                    if permissions.mode() & 0o111 != 0
                        && let Some(file_name) = path.file_name().and_then(|name| name.to_str())
                        && seen_executables.insert(file_name.to_string())
                    {
                        let app = executable_app(&path, file_name);
                        if sender.send(history_cache.apply_to_app(app)).is_err() {
                            return None;
                        }
                    }
                }
            }
        }
    }

    Some(())
}

/// Finds XDG applications in `dirs` and streams them back over a channel.
pub fn read_with_options(
    dirs: Vec<impl Into<PathBuf>>,
    db: &std::sync::Arc<redb::Database>,
    options: DiscoverOptions,
) -> mpsc::Receiver<App> {
    let (sender, receiver) = mpsc::channel();
    let dirs: Vec<PathBuf> = dirs.into_iter().map(Into::into).collect();
    let db_clone = std::sync::Arc::clone(db);
    let current_desktop = current_desktop(options.filter_desktop);

    let _worker = thread::spawn(move || {
        let history_cache = HistoryCache::load(&db_clone).unwrap_or_else(|_| HistoryCache {
            history: std::collections::HashMap::new(),
            pinned: std::collections::HashSet::new(),
        });
        let desktop_cache = crate::core::cache::DesktopCache::new(db_clone.clone()).ok();
        let desktop_files = load_desktop_files(&dirs, desktop_cache.as_ref());
        let history_cache_ref = &history_cache;
        let desktop_cache_ref = desktop_cache.as_ref();
        let current_desktop_ref = current_desktop.as_deref();

        let apps_to_cache: Vec<(PathBuf, App)> = desktop_files
            .into_par_iter()
            .filter_map(|file_path| {
                let file_path_ref = file_path.as_path();
                let (mut app, file_contents) = load_app_from_path(
                    file_path_ref,
                    desktop_cache_ref,
                    options.filter_desktop,
                    options.auto_hide_duplicates,
                )?;
                attach_desktop_id(&mut app, &dirs, file_path_ref, None);

                if !should_keep_for_desktop(&app, current_desktop_ref) {
                    return None;
                }

                let app_with_history = history_cache_ref.apply_to_app(app.clone());

                if !app.hidden
                    && !options.filter_actions
                    && let Some(actions) = &app.actions
                {
                    let contents = match &file_contents {
                        Some(contents) => Some(contents.clone()),
                        None => fs::read_to_string(file_path_ref).ok(),
                    };

                    if let Some(contents) = contents {
                        for action in actions {
                            let action = Action::default().name(action).from(app.name.clone());
                            if let Ok(mut action_app) =
                                App::parse_action(&contents, &action, options.filter_desktop)
                            {
                                if action_app.icon.is_none() {
                                    action_app.icon.clone_from(&app.icon);
                                }
                                attach_desktop_id(
                                    &mut action_app,
                                    &dirs,
                                    file_path_ref,
                                    Some(action.name.as_str()),
                                );
                                if sender
                                    .send(history_cache_ref.apply_to_app(action_app))
                                    .is_err()
                                {
                                    return None;
                                }
                            }
                        }
                    }
                }

                if sender.send(app_with_history).is_err() {
                    return None;
                }

                file_contents.map(|_| (file_path, app))
            })
            .collect();

        if !apps_to_cache.is_empty()
            && let Some(cache) = desktop_cache.as_ref()
        {
            let _ = cache.batch_set(apps_to_cache);
        }

        if options.list_executables {
            let _ = send_path_executables(&sender, &history_cache);
        }
    });

    receiver
}

#[cfg(test)]
mod tests {
    use super::{DiscoverOptions, desktop_file_id, read_with_options};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "fsel-discover-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    fn collect_names(filter_actions: bool) -> Vec<String> {
        let dir = test_temp_dir(if filter_actions { "filtered" } else { "all" });
        let db_path = dir.join("history.redb");
        let desktop_path = dir.join("editor.desktop");
        fs::write(
            &desktop_path,
            "[Desktop Entry]\nType=Application\nName=Editor\nExec=/usr/bin/editor\nActions=OpenWindow;\n\n[Desktop Action OpenWindow]\nName=Open Window\nExec=/usr/bin/editor --new-window\n",
        )
        .expect("desktop entry should be written");

        let db = Arc::new(redb::Database::create(&db_path).expect("database should be created"));
        let receiver = read_with_options(
            vec![dir.clone()],
            &db,
            DiscoverOptions {
                filter_desktop: false,
                filter_actions,
                list_executables: false,
                auto_hide_duplicates: false,
            },
        );
        let mut names = Vec::new();
        while let Ok(app) = receiver.recv() {
            names.push(app.name);
        }

        let _ = fs::remove_dir_all(dir);
        names
    }

    #[test]
    fn read_with_options_filters_desktop_actions_when_requested() {
        let names = collect_names(true);

        assert_eq!(names, vec!["Editor".to_string()]);
    }

    #[test]
    fn read_with_options_keeps_desktop_actions_when_disabled() {
        let mut names = collect_names(false);
        names.sort();

        assert_eq!(
            names,
            vec!["Editor".to_string(), "Editor (Open Window)".to_string()]
        );
    }

    #[test]
    fn desktop_file_id_uses_the_relative_path() {
        let root = PathBuf::from("/usr/share/applications");
        let nested = root.join("vendor/editor.desktop");

        assert_eq!(
            desktop_file_id(&[root], &nested).as_deref(),
            Some("vendor-editor.desktop")
        );
    }

    #[cfg(unix)]
    #[test]
    fn desktop_file_id_escapes_non_utf8_components_without_collisions() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let root = PathBuf::from("/usr/share/applications");
        let first = root.join(OsString::from_vec(vec![
            0xfe, b'.', b'd', b'e', b's', b'k', b't', b'o', b'p',
        ]));
        let second = root.join(OsString::from_vec(vec![
            0xff, b'.', b'd', b'e', b's', b'k', b't', b'o', b'p',
        ]));
        let escaped_literal = root.join("%00fe2e6465736b746f70");

        let first_id = desktop_file_id(std::slice::from_ref(&root), &first)
            .expect("first ID should be available");
        let second_id = desktop_file_id(std::slice::from_ref(&root), &second)
            .expect("second ID should be available");
        let literal_id =
            desktop_file_id(&[root], &escaped_literal).expect("literal ID should be available");

        assert_ne!(first_id, second_id);
        assert_ne!(first_id, literal_id);
    }

    #[test]
    fn automatic_discovery_keeps_hidden_tombstones_for_resolution() {
        let dir = test_temp_dir("tombstone");
        let db_path = dir.join("history.redb");
        fs::write(
            dir.join("editor.desktop"),
            "[Desktop Entry]\nType=Application\nName=Editor\nHidden=true\n",
        )
        .expect("desktop entry should be written");
        let db = Arc::new(redb::Database::create(db_path).expect("database should be created"));

        let receiver = read_with_options(
            vec![dir.clone()],
            &db,
            DiscoverOptions {
                filter_desktop: false,
                filter_actions: false,
                list_executables: false,
                auto_hide_duplicates: true,
            },
        );
        let apps = receiver.into_iter().collect::<Vec<_>>();

        assert_eq!(apps.len(), 1);
        assert!(apps[0].hidden);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn desktop_actions_inherit_the_parent_icon_when_unspecified() {
        let dir = test_temp_dir("action-icon");
        let db_path = dir.join("history.redb");
        fs::write(
            dir.join("editor.desktop"),
            "[Desktop Entry]\nType=Application\nName=Editor\nExec=editor\nIcon=editor\nActions=New;\n\n[Desktop Action New]\nName=New\nExec=editor --new\n",
        )
        .expect("desktop entry should be written");
        let db = Arc::new(redb::Database::create(&db_path).expect("database should be created"));
        let receiver = read_with_options(
            vec![dir.clone()],
            &db,
            DiscoverOptions {
                filter_desktop: false,
                filter_actions: false,
                list_executables: false,
                auto_hide_duplicates: false,
            },
        );
        let apps = receiver.into_iter().collect::<Vec<_>>();
        let action = apps
            .iter()
            .find(|app| app.name == "Editor (New)")
            .expect("desktop action should be discovered");

        assert_eq!(action.icon.as_deref(), Some("editor"));
        let _ = fs::remove_dir_all(dir);
    }
}
