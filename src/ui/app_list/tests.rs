use super::{
    launcher_list_icon_area, launcher_visible_rows, list_areas, marker_gutter_width,
    overflow_icon_area, selection_marker_area,
};
use crate::cli::{DesktopIconMode, Opts};
use crate::ui::{HorizontalPosition, PanelPosition};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::style::Color;

#[test]
fn list_icons_reduce_visible_apps_by_configured_row_height() {
    let cli = Opts {
        desktop_icon_mode: DesktopIconMode::List,
        desktop_icon_list_height: 2,
        title_panel_height_percent: 25,
        input_panel_height: 3,
        ..Opts::default()
    };

    assert_eq!(launcher_visible_rows(Rect::new(0, 0, 80, 40), &cli), 12);
}

#[test]
fn selection_marker_stays_on_the_first_item_row() {
    let area = Rect::new(2, 4, 2, 12);

    assert_eq!(selection_marker_area(area, 1, 3), Rect::new(2, 7, 2, 1));
}

#[test]
fn selection_marker_gutter_follows_configured_glyph_width() {
    let block = Opts {
        selection_marker: "█".to_string(),
        ..Opts::default()
    };
    let wide = Opts {
        selection_marker: "界".to_string(),
        ..Opts::default()
    };
    let hidden = Opts {
        show_selection_marker: false,
        ..Opts::default()
    };

    assert_eq!(marker_gutter_width(&block), 2);
    assert_eq!(marker_gutter_width(&wide), 3);
    assert_eq!(marker_gutter_width(&hidden), 0);
}

#[test]
fn borderless_apps_panel_uses_the_released_rows() {
    let cli = Opts {
        desktop_icon_mode: DesktopIconMode::List,
        desktop_icon_list_height: 2,
        title_panel_height_percent: 25,
        input_panel_height: 3,
        show_items_border: false,
        ..Opts::default()
    };

    assert_eq!(launcher_visible_rows(Rect::new(0, 0, 80, 40), &cli), 13);
}

#[test]
fn visible_rows_saturates_when_panel_sizes_overflow() {
    let cli = Opts {
        title_panel_height_percent: u16::MAX,
        input_panel_height: u16::MAX,
        ..Opts::default()
    };

    assert_eq!(
        launcher_visible_rows(Rect::new(0, 0, 80, u16::MAX), &cli),
        0
    );
}

#[test]
fn middle_title_position_uses_the_actual_apps_pane_height() {
    let cli = Opts {
        desktop_icon_mode: DesktopIconMode::List,
        desktop_icon_list_height: 2,
        title_panel_position: Some(PanelPosition::Middle),
        title_panel_height_percent: 25,
        input_panel_height: 3,
        ..Opts::default()
    };

    assert_eq!(launcher_visible_rows(Rect::new(0, 0, 80, 40), &cli), 6);
}

#[test]
fn list_icons_can_reserve_the_right_side() {
    let cli = Opts {
        desktop_icon_mode: DesktopIconMode::Both,
        desktop_icon_position: HorizontalPosition::Right,
        desktop_icon_list_width: 4,
        ..Opts::default()
    };

    let areas = list_areas(Rect::new(10, 3, 30, 8), &cli);

    assert_eq!(areas.text, Rect::new(10, 3, 25, 8));
    assert_eq!(areas.icon, Some(Rect::new(36, 3, 4, 8)));
    assert_eq!(areas.selection, None);
}

#[test]
fn list_icons_can_reserve_the_left_side() {
    let cli = Opts {
        desktop_icon_mode: DesktopIconMode::List,
        desktop_icon_position: HorizontalPosition::Left,
        desktop_icon_list_width: 5,
        ..Opts::default()
    };

    let areas = list_areas(Rect::new(2, 4, 20, 6), &cli);

    assert_eq!(areas.text, Rect::new(8, 4, 14, 6));
    assert_eq!(areas.icon, Some(Rect::new(2, 4, 5, 6)));
    assert_eq!(areas.selection, None);
}

#[test]
fn selection_arrow_can_be_reserved_before_a_left_icon() {
    let cli = Opts {
        desktop_icon_mode: DesktopIconMode::List,
        desktop_icon_position: HorizontalPosition::Left,
        desktop_icon_list_width: 5,
        desktop_icon_arrow_before: true,
        ..Opts::default()
    };

    let areas = list_areas(Rect::new(2, 4, 20, 6), &cli);

    assert_eq!(areas.selection, Some(Rect::new(2, 4, 2, 6)));
    assert_eq!(areas.icon, Some(Rect::new(4, 4, 5, 6)));
    assert_eq!(areas.text, Rect::new(10, 4, 12, 6));
}

#[test]
fn hidden_selection_marker_releases_its_gutter() {
    let cli = Opts {
        desktop_icon_mode: DesktopIconMode::List,
        desktop_icon_position: HorizontalPosition::Left,
        desktop_icon_list_width: 5,
        desktop_icon_arrow_before: true,
        show_selection_marker: false,
        ..Opts::default()
    };

    let areas = list_areas(Rect::new(2, 4, 20, 6), &cli);

    assert_eq!(areas.selection, None);
    assert_eq!(areas.icon, Some(Rect::new(2, 4, 5, 6)));
    assert_eq!(areas.text, Rect::new(8, 4, 14, 6));
}

#[test]
fn list_icon_gap_is_reserved_between_icon_and_label() {
    let cli = Opts {
        desktop_icon_mode: DesktopIconMode::List,
        desktop_icon_position: HorizontalPosition::Left,
        desktop_icon_list_width: 5,
        desktop_icon_list_gap: 3,
        ..Opts::default()
    };

    let areas = list_areas(Rect::new(2, 4, 20, 6), &cli);

    assert_eq!(areas.icon, Some(Rect::new(2, 4, 5, 6)));
    assert_eq!(areas.text, Rect::new(10, 4, 12, 6));
}

#[test]
fn narrow_list_keeps_selection_and_label_space() {
    let cli = Opts {
        desktop_icon_mode: DesktopIconMode::List,
        desktop_icon_position: HorizontalPosition::Left,
        desktop_icon_list_width: 16,
        ..Opts::default()
    };

    let areas = list_areas(Rect::new(2, 4, 4, 6), &cli);

    assert_eq!(areas.icon, Some(Rect::new(2, 4, 1, 6)));
    assert_eq!(areas.text, Rect::new(3, 4, 3, 6));
}

#[test]
fn list_worker_area_matches_each_rendered_icon_slot() {
    let cli = Opts {
        desktop_icon_mode: DesktopIconMode::List,
        desktop_icon_list_width: 5,
        desktop_icon_list_height: 2,
        ..Opts::default()
    };

    assert_eq!(
        launcher_list_icon_area(Rect::new(0, 0, 100, 40), &cli),
        Rect::new(0, 0, 5, 2)
    );
}

#[test]
fn negative_icon_overflow_extends_above_the_item() {
    assert_eq!(
        overflow_icon_area(Rect::new(4, 12, 5, 8), 2),
        Rect::new(4, 10, 5, 10)
    );
}

#[test]
fn icon_overflow_saturates_at_the_terminal_top() {
    assert_eq!(
        overflow_icon_area(Rect::new(4, 1, 5, 8), 2),
        Rect::new(4, 0, 5, 10)
    );
}

#[test]
fn rounded_selection_uses_half_cell_caps() {
    let backend = TestBackend::new(5, 1);
    let mut terminal = Terminal::new(backend).expect("test terminal should initialize");
    terminal
        .draw(|frame| {
            super::super::render_selection_background(
                frame,
                Rect::new(0, 0, 5, 1),
                Color::Black,
                Color::Blue,
                true,
            );
        })
        .expect("selection should render");
    let buffer = terminal.backend().buffer();

    assert_eq!(buffer[(0, 0)].symbol(), "▐");
    assert_eq!(buffer[(0, 0)].fg, Color::Blue);
    assert_eq!(buffer[(0, 0)].bg, Color::Black);
    assert_eq!(buffer[(1, 0)].bg, Color::Blue);
    assert_eq!(buffer[(4, 0)].symbol(), "▌");
}
