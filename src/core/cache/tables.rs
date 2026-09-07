use redb::TableDefinition;

pub const DESKTOP_CACHE_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("desktop_cache");
pub const NAME_INDEX_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("app_name_index");
pub const FILE_LIST_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("file_list_cache");
pub const ICON_PATH_CACHE_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("icon_path_cache");
pub const HISTORY_TABLE: TableDefinition<&str, u64> = TableDefinition::new("history");
pub const PINNED_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("pinned_apps");
pub const FRECENCY_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("frecency");
pub const HIDDEN_ENTRIES_TABLE: TableDefinition<u64, &[u8]> =
    TableDefinition::new("hidden_entries");
pub const HIDDEN_ENTRY_META_TABLE: TableDefinition<&str, u64> =
    TableDefinition::new("hidden_entry_meta");
