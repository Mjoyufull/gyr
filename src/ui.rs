//! Shared terminal UI rendering, input, graphics, and layout primitives.

mod app_list;
mod app_ui;
mod dmenu_ui;
mod graphics;
mod input;
mod input_panel;
mod keybinds;
mod panel_layout;
pub(crate) mod terminal;
mod types;
mod visual;

pub(crate) use app_list::{
    app_row_height, launcher_list_content_area, launcher_list_icon_area, launcher_visible_rows,
};
pub use app_ui::{AppIcons, UI};
pub(crate) use app_ui::{ListIconPlacement, launcher_preview_icon_area};
pub use dmenu_ui::{DmenuUI, TagMode};
pub use graphics::{DISPLAY_STATE, DisplayState, GraphicsAdapter, ImageManager};
#[allow(unused_imports)]
pub use input::{AsyncInput, Config as InputConfig, Event as InputEvent, Input};
pub(crate) use input_panel::{InputPanelData, InputPanelTheme, render_panel as render_input_panel};
pub use keybinds::Keybinds;
pub(crate) use panel_layout::{
    PanelLayout, effective_content_height, items_panel_bounds, split_content_panels,
};
pub use types::*;
pub(crate) use visual::{
    PanelTheme, panel_block, render_selection_background, selection_content_area,
};
