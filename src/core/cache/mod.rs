mod desktop;
mod history;
mod icons;
mod tables;

#[allow(unused_imports)]
pub use desktop::DesktopCache;
#[allow(unused_imports)]
pub use history::HistoryCache;
pub(crate) use icons::{IconPathCache, IconPathLookup};
#[allow(unused_imports)]
pub use tables::{
    DESKTOP_CACHE_TABLE, FILE_LIST_TABLE, FRECENCY_TABLE, HIDDEN_ENTRIES_TABLE,
    HIDDEN_ENTRY_META_TABLE, HISTORY_TABLE, ICON_PATH_CACHE_TABLE, NAME_INDEX_TABLE, PINNED_TABLE,
};
