use super::tables::{DESKTOP_CACHE_TABLE, FILE_LIST_TABLE, NAME_INDEX_TABLE};
use crate::desktop::App;
use eyre::{Result, eyre};
use redb::{Database, ReadableDatabase, ReadableTable};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

const FILE_LIST_CACHE_KEY: &str = "paths";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    app: App,
    mtime: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyCacheEntry {
    app: App,
    mtime: SystemTime,
    #[allow(dead_code)]
    path: PathBuf,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileListCache {
    paths: Vec<PathBuf>,
    dir_mtimes: HashMap<PathBuf, SystemTime>,
}

pub struct DesktopCache {
    db: Arc<Database>,
}

impl DesktopCache {
    pub fn new(db: Arc<Database>) -> Result<Self> {
        let write_txn = db.begin_write()?;
        {
            let _ = write_txn.open_table(DESKTOP_CACHE_TABLE)?;
            let _ = write_txn.open_table(NAME_INDEX_TABLE)?;
            let _ = write_txn.open_table(FILE_LIST_TABLE)?;
        }
        write_txn.commit()?;

        Ok(Self { db })
    }

    #[allow(dead_code)]
    pub fn get_file_list(&self, dirs: &[PathBuf]) -> Result<Option<Vec<PathBuf>>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(FILE_LIST_TABLE)?;

        if let Some(data) = table.get(FILE_LIST_CACHE_KEY)?
            && let Ok(cache) = postcard::from_bytes::<FileListCache>(data.value())
            && file_list_cache_is_fresh(&cache, dirs)
        {
            return Ok(Some(cache.paths));
        }

        Ok(None)
    }

    #[allow(dead_code)]
    pub fn batch_get(&self, paths: &[PathBuf]) -> Result<HashMap<PathBuf, App>> {
        let mut result = HashMap::new();
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(DESKTOP_CACHE_TABLE)?;

        for path in paths {
            if let Some(app) = read_cached_app(&table, path)? {
                result.insert(path.clone(), app);
            }
        }

        Ok(result)
    }

    #[allow(dead_code)]
    pub fn batch_set(&self, apps: Vec<(PathBuf, App)>) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut cache_table = write_txn.open_table(DESKTOP_CACHE_TABLE)?;
            let mut index_table = write_txn.open_table(NAME_INDEX_TABLE)?;

            for (path, app) in apps {
                write_cached_app(&mut cache_table, &mut index_table, &path, &app)?;
            }
        }
        write_txn.commit()?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn set_file_list(&self, paths: Vec<PathBuf>, scanned_dirs: &[PathBuf]) -> Result<()> {
        let cache = FileListCache {
            paths,
            dir_mtimes: collect_dir_mtimes(scanned_dirs),
        };
        let data = postcard::to_allocvec(&cache)?;

        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(FILE_LIST_TABLE)?;
            table.insert(FILE_LIST_CACHE_KEY, data.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn get_by_name(&self, app_name: &str) -> Result<Option<App>> {
        let read_txn = self.db.begin_read()?;
        let index_table = read_txn.open_table(NAME_INDEX_TABLE)?;

        if let Some(path_bytes) = index_table.get(app_name)? {
            let path_key = std::str::from_utf8(path_bytes.value())
                .map_err(|error| eyre!("Invalid cached path key for app '{app_name}': {error}"))?
                .to_owned();
            let cache_table = read_txn.open_table(DESKTOP_CACHE_TABLE)?;
            return read_cached_app_by_key(&cache_table, &path_key);
        }

        Ok(None)
    }

    pub fn get(&self, path: &Path) -> Result<Option<App>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(DESKTOP_CACHE_TABLE)?;
        read_cached_app(&table, path)
    }

    pub fn set(&self, path: &Path, app: App) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut cache_table = write_txn.open_table(DESKTOP_CACHE_TABLE)?;
            let mut index_table = write_txn.open_table(NAME_INDEX_TABLE)?;
            write_cached_app(&mut cache_table, &mut index_table, path, &app)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn clear(&self) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut cache_table = write_txn.open_table(DESKTOP_CACHE_TABLE)?;
            let mut index_table = write_txn.open_table(NAME_INDEX_TABLE)?;
            let mut file_list_table = write_txn.open_table(FILE_LIST_TABLE)?;

            remove_all_rows(&mut cache_table)?;
            remove_all_rows(&mut index_table)?;
            remove_all_rows(&mut file_list_table)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn clear_file_list(&self) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(FILE_LIST_TABLE)?;
            remove_all_rows(&mut table)?;
        }
        write_txn.commit()?;
        Ok(())
    }
}

fn collect_dir_mtimes(scanned_dirs: &[PathBuf]) -> HashMap<PathBuf, SystemTime> {
    let mut dir_mtimes = HashMap::new();

    for dir in scanned_dirs {
        if let Ok(metadata) = fs::metadata(dir)
            && let Ok(mtime) = metadata.modified()
        {
            dir_mtimes.insert(dir.clone(), mtime);
        }
    }

    dir_mtimes
}

fn file_list_cache_is_fresh(cache: &FileListCache, dirs: &[PathBuf]) -> bool {
    let unique_dirs: HashSet<PathBuf> = dirs.iter().cloned().collect();

    for dir in &unique_dirs {
        let Some(cached_mtime) = cache.dir_mtimes.get(dir) else {
            return false;
        };

        let Ok(metadata) = fs::metadata(dir) else {
            return false;
        };
        let Ok(current_mtime) = metadata.modified() else {
            return false;
        };

        if current_mtime != *cached_mtime {
            return false;
        }
    }

    cache.dir_mtimes.len() == unique_dirs.len()
}

fn read_cached_app(table: &redb::ReadOnlyTable<&str, &[u8]>, path: &Path) -> Result<Option<App>> {
    let path_key = crate::core::path_key::encode(path);
    if let Some(app) = read_cached_app_at_key(table, &path_key, path)? {
        return Ok(Some(app));
    }

    let legacy_path_key = path.to_string_lossy();
    read_cached_app_at_key(table, legacy_path_key.as_ref(), path)
}

fn read_cached_app_by_key(
    table: &redb::ReadOnlyTable<&str, &[u8]>,
    path_key: &str,
) -> Result<Option<App>> {
    if let Some(path) = crate::core::path_key::decode(path_key)
        && let Some(app) = read_cached_app_at_key(table, path_key, &path)?
    {
        return Ok(Some(app));
    }

    read_cached_app_at_key(table, path_key, Path::new(path_key))
}

fn read_cached_app_at_key(
    table: &redb::ReadOnlyTable<&str, &[u8]>,
    path_key: &str,
    path: &Path,
) -> Result<Option<App>> {
    if let Some(data) = table.get(path_key)?
        && let Some(entry) = deserialize_cache_entry(data.value())
        && cache_entry_is_fresh(path, &entry)
    {
        let mut app = entry.app;
        app.set_source_path(path);
        return Ok(Some(app));
    }

    Ok(None)
}

fn deserialize_cache_entry(data: &[u8]) -> Option<CacheEntry> {
    if let Ok(entry) = postcard::from_bytes::<CacheEntry>(data) {
        return Some(entry);
    }

    postcard::from_bytes::<LegacyCacheEntry>(data)
        .ok()
        .map(|entry| CacheEntry {
            app: entry.app,
            mtime: entry.mtime,
        })
}

fn cache_entry_is_fresh(path: &Path, entry: &CacheEntry) -> bool {
    // Icon values containing desktop-entry escapes predate icon normalization.
    // Reparse those rows so the resolver never receives a stale raw value.
    if entry
        .app
        .icon
        .as_deref()
        .is_some_and(|icon| icon.contains('\\'))
    {
        return false;
    }
    if let Ok(metadata) = fs::metadata(path)
        && let Ok(mtime) = metadata.modified()
    {
        return mtime == entry.mtime;
    }

    false
}

fn write_cached_app(
    cache_table: &mut redb::Table<&str, &[u8]>,
    index_table: &mut redb::Table<&str, &[u8]>,
    path: &Path,
    app: &App,
) -> Result<()> {
    let Ok(metadata) = fs::metadata(path) else {
        return Ok(());
    };
    let Ok(mtime) = metadata.modified() else {
        return Ok(());
    };

    let entry = CacheEntry {
        app: app.clone(),
        mtime,
    };
    let data = postcard::to_allocvec(&entry)?;
    let path_key = crate::core::path_key::encode(path);

    if let Some(existing) = cache_table.get(path_key.as_str())?
        && let Some(previous_entry) = deserialize_cache_entry(existing.value())
        && previous_entry.app.name != app.name
    {
        let should_remove_old_name = index_table
            .get(previous_entry.app.name.as_str())?
            .is_some_and(|index_value| index_value.value() == path_key.as_bytes());

        if should_remove_old_name {
            index_table.remove(previous_entry.app.name.as_str())?;
        }
    }

    cache_table.insert(path_key.as_str(), data.as_slice())?;
    index_table.insert(app.name.as_str(), path_key.as_bytes())?;
    Ok(())
}

fn remove_all_rows<V>(table: &mut redb::Table<&str, V>) -> Result<()>
where
    V: redb::Value + 'static,
{
    let keys: Vec<String> = table
        .iter()?
        .map(|result| result.map(|(key, _)| key.value().to_string()))
        .collect::<std::result::Result<_, _>>()?;

    for key in keys {
        table.remove(key.as_str())?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{DesktopCache, FileListCache, LegacyCacheEntry, file_list_cache_is_fresh};
    use crate::desktop::App;
    use std::collections::HashMap;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::ffi::OsStringExt;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "fsel-desktop-cache-{label}-{}-{unique}",
            crate::platform::process::get_current_pid()
        ));
        fs::create_dir_all(&dir).expect("test temp dir should be created");
        dir
    }

    fn sample_app(name: &str) -> App {
        App::parse(
            format!(
                "[Desktop Entry]\nType=Application\nName={name}\nExec=/usr/bin/{name}\nComment=sample"
            ),
            false,
        )
        .expect("desktop entry should parse")
    }

    #[test]
    fn file_list_cache_freshness_ignores_duplicate_dirs() {
        let dir = test_temp_dir("dedup");
        let mtime = fs::metadata(&dir)
            .and_then(|metadata| metadata.modified())
            .expect("directory mtime should be available");
        let cache = FileListCache {
            paths: Vec::new(),
            dir_mtimes: HashMap::from([(dir.clone(), mtime)]),
        };

        assert!(file_list_cache_is_fresh(
            &cache,
            &[dir.clone(), dir.clone()]
        ));

        let _ = fs::remove_dir_all(dir);
    }

    #[cfg(unix)]
    #[test]
    fn get_by_name_supports_non_utf8_paths() {
        let dir = test_temp_dir("non-utf8");
        let db_path = dir.join("desktop-cache.redb");
        let db = Arc::new(redb::Database::create(&db_path).expect("database should be created"));
        let cache = DesktopCache::new(Arc::clone(&db)).expect("desktop cache should initialize");

        let file_name = std::ffi::OsString::from_vec(vec![
            b'n', b'o', b'n', b'u', b't', b'f', b'8', b'-', 0xff, b'.', b'd', b'e', b's', b'k',
            b't', b'o', b'p',
        ]);
        let desktop_path = dir.join(PathBuf::from(file_name));
        fs::write(
            &desktop_path,
            "[Desktop Entry]\nType=Application\nName=CacheTest\nExec=/bin/true\n",
        )
        .expect("desktop entry should be written");

        let app = sample_app("CacheTest");
        cache
            .set(&desktop_path, app.clone())
            .expect("cache set should succeed");

        let loaded = cache
            .get_by_name("CacheTest")
            .expect("cache lookup should succeed")
            .expect("cached app should exist");

        assert_eq!(loaded.name, app.name);
        assert_eq!(loaded.command, app.command);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn legacy_cache_keys_remain_readable_after_upgrade() {
        let dir = test_temp_dir("legacy-cache");
        let db_path = dir.join("desktop-cache.redb");
        let db = Arc::new(redb::Database::create(&db_path).expect("database should be created"));
        let cache = DesktopCache::new(Arc::clone(&db)).expect("desktop cache should initialize");

        let desktop_path = dir.join("legacy.desktop");
        fs::write(
            &desktop_path,
            "[Desktop Entry]\nType=Application\nName=LegacyCache\nExec=/bin/true\n",
        )
        .expect("desktop entry should be written");

        let metadata = fs::metadata(&desktop_path).expect("desktop metadata should be readable");
        let mtime = metadata
            .modified()
            .expect("desktop mtime should be readable");
        let app = sample_app("LegacyCache");
        let legacy_key = desktop_path.to_string_lossy().to_string();
        let legacy_entry = LegacyCacheEntry {
            app: app.clone(),
            mtime,
            path: desktop_path.clone(),
        };
        let data = postcard::to_allocvec(&legacy_entry).expect("legacy cache entry should encode");

        let write_txn = db.begin_write().expect("write transaction should open");
        {
            let mut cache_table = write_txn
                .open_table(super::DESKTOP_CACHE_TABLE)
                .expect("cache table should open");
            let mut index_table = write_txn
                .open_table(super::NAME_INDEX_TABLE)
                .expect("name index should open");
            cache_table
                .insert(legacy_key.as_str(), data.as_slice())
                .expect("legacy cache row should insert");
            index_table
                .insert(app.name.as_str(), legacy_key.as_bytes())
                .expect("legacy name index should insert");
        }
        write_txn
            .commit()
            .expect("legacy cache write should commit");

        let loaded_by_name = cache
            .get_by_name("LegacyCache")
            .expect("legacy name lookup should succeed")
            .expect("legacy cached app should exist");
        let loaded_by_path = cache
            .get(&desktop_path)
            .expect("legacy path lookup should succeed")
            .expect("legacy cached app should exist by path");

        assert_eq!(loaded_by_name.name, app.name);
        assert_eq!(loaded_by_path.command, app.command);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn escaped_cached_icons_are_invalidated_for_reparsing() {
        let dir = test_temp_dir("escaped-icon");
        let db_path = dir.join("desktop-cache.redb");
        let db = Arc::new(redb::Database::create(&db_path).expect("database should be created"));
        let cache = DesktopCache::new(Arc::clone(&db)).expect("desktop cache should initialize");
        let desktop_path = dir.join("escaped.desktop");
        fs::write(
            &desktop_path,
            "[Desktop Entry]\nType=Application\nName=EscapedIcon\nExec=/bin/true\nIcon=/opt/My\\sApp/icon.png\n",
        )
        .expect("desktop entry should be written");
        let mut app = sample_app("EscapedIcon");
        app.icon = Some("/opt/My\\sApp/icon.png".to_string());
        cache
            .set(&desktop_path, app)
            .expect("cache set should succeed");

        assert!(
            cache
                .get(&desktop_path)
                .expect("cache lookup should succeed")
                .is_none()
        );

        let _ = fs::remove_dir_all(dir);
    }
}
