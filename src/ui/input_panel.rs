//! Shared classic and command-style input panel rendering.

use crate::cli::Opts;
use crate::core::state::State;
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

pub(crate) struct InputPanelData<'a> {
    pub(crate) title: &'a str,
    pub(crate) classic_line: Line<'a>,
    pub(crate) command_line: Line<'a>,
    pub(crate) selected_name: &'a str,
    pub(crate) selected_count: usize,
    pub(crate) total_count: usize,
    pub(crate) primary_action: &'a str,
    pub(crate) exit_action: &'a str,
}

#[derive(Clone, Copy)]
pub(crate) struct InputPanelTheme<'a> {
    pub(crate) style: super::InputPanelStyle,
    pub(crate) show_border: bool,
    pub(crate) show_title: bool,
    pub(crate) bold_title: bool,
    pub(crate) show_count: bool,
    pub(crate) rounded_border: bool,
    pub(crate) border_color: ratatui::style::Color,
    pub(crate) background_color: ratatui::style::Color,
    pub(crate) text_color: ratatui::style::Color,
    pub(crate) highlight_color: ratatui::style::Color,
    pub(crate) title_color: ratatui::style::Color,
    pub(crate) keybinds: &'a super::Keybinds,
}

pub(super) fn render(frame: &mut Frame, state: &State, cli: &Opts, area: Rect) {
    let selected_name = state
        .selected
        .and_then(|selected| state.shown.get(selected))
        .map_or("fsel", |app| app.name.as_str());
    let data = InputPanelData {
        title: " Input ",
        classic_line: query_line(state, cli, cli.show_input_count),
        command_line: query_line(state, cli, false),
        selected_name,
        selected_count: state.selected.map_or(0, |selected| selected + 1),
        total_count: state.shown.len(),
        primary_action: "launch",
        exit_action: "close",
    };
    render_panel(frame, area, data, theme_from_cli(cli));
}

pub(crate) fn render_panel(
    frame: &mut Frame,
    area: Rect,
    data: InputPanelData<'_>,
    theme: InputPanelTheme<'_>,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    match theme.style {
        super::InputPanelStyle::Classic => render_classic(frame, area, data, theme),
        super::InputPanelStyle::Command => render_command(frame, area, data, theme),
    }
}

fn render_classic(
    frame: &mut Frame,
    area: Rect,
    data: InputPanelData<'_>,
    theme: InputPanelTheme<'_>,
) {
    let block = input_block(data.title, theme);
    let available_width = usize::from(block.inner(area).width);
    let scroll_x = horizontal_scroll(&data.classic_line, available_width);
    let input = Paragraph::new(data.classic_line)
        .block(block)
        .style(
            Style::default()
                .fg(theme.text_color)
                .bg(theme.background_color),
        )
        .scroll((0, scroll_x));
    frame.render_widget(input, area);
}

fn render_command(
    frame: &mut Frame,
    area: Rect,
    data: InputPanelData<'_>,
    theme: InputPanelTheme<'_>,
) {
    let block = input_block(data.title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let (header, footer) = command_areas(inner);
    render_command_query(frame, data.command_line.clone(), theme, header);
    if let Some(footer) = footer {
        render_command_footer(frame, &data, theme, footer);
    }
}

fn render_command_query(
    frame: &mut Frame,
    line: Line<'_>,
    theme: InputPanelTheme<'_>,
    header: Rect,
) {
    let rail_width = u16::from(header.width > 2);
    if rail_width > 0 {
        for y in header.y..header.y + header.height {
            frame.render_widget(
                Paragraph::new("▍").style(
                    Style::default()
                        .fg(theme.highlight_color)
                        .bg(theme.background_color),
                ),
                Rect::new(header.x, y, 1, 1),
            );
        }
    }

    let query_area = Rect::new(
        header.x.saturating_add(rail_width.saturating_mul(2)),
        header.y + header.height.saturating_sub(1) / 2,
        header.width.saturating_sub(rail_width.saturating_mul(2)),
        u16::from(header.height > 0),
    );
    let scroll_x = horizontal_scroll(&line, usize::from(query_area.width));
    frame.render_widget(
        Paragraph::new(line)
            .style(
                Style::default()
                    .fg(theme.text_color)
                    .bg(theme.background_color),
            )
            .scroll((0, scroll_x)),
        query_area,
    );
}

fn render_command_footer(
    frame: &mut Frame,
    data: &InputPanelData<'_>,
    theme: InputPanelTheme<'_>,
    footer: Rect,
) {
    let primary = action_hint(theme.keybinds.select_hint(), data.primary_action);
    let exit = action_hint(theme.keybinds.exit_hint(), data.exit_action);
    let actions = format!("{primary}  {exit}");
    let right = if theme.show_count {
        format!("{}/{}  {actions}", data.selected_count, data.total_count)
    } else {
        actions
    };
    let right_width = UnicodeWidthStr::width(right.as_str()).min(usize::from(footer.width)) as u16;
    let left_width = footer.width.saturating_sub(right_width.saturating_add(2));
    if left_width > 0 {
        frame.render_widget(
            Paragraph::new(data.selected_name).style(
                Style::default()
                    .fg(theme.highlight_color)
                    .bg(theme.background_color),
            ),
            Rect::new(footer.x, footer.y, left_width, footer.height),
        );
    }
    frame.render_widget(
        Paragraph::new(right).alignment(Alignment::Right).style(
            Style::default()
                .fg(theme.text_color)
                .bg(theme.background_color),
        ),
        Rect::new(
            footer.x + footer.width - right_width,
            footer.y,
            right_width,
            footer.height,
        ),
    );
}

fn action_hint(binding: Option<String>, action: &str) -> String {
    binding.map_or_else(
        || action.to_string(),
        |binding| format!("{binding} {action}"),
    )
}

fn input_block<'a>(title: &'a str, theme: InputPanelTheme<'_>) -> ratatui::widgets::Block<'a> {
    super::panel_block(
        title,
        super::PanelTheme {
            show_border: theme.show_border,
            show_title: theme.show_title,
            bold_title: theme.bold_title,
            rounded_border: theme.rounded_border,
            border_color: theme.border_color,
            background_color: theme.background_color,
            title_color: theme.title_color,
        },
    )
}

fn theme_from_cli(cli: &Opts) -> InputPanelTheme<'_> {
    InputPanelTheme {
        style: cli.input_panel_style,
        show_border: cli.show_input_border,
        show_title: cli.show_panel_titles,
        bold_title: false,
        show_count: cli.show_input_count,
        rounded_border: cli.rounded_borders,
        border_color: cli.input_border_color,
        background_color: cli.input_background_color,
        text_color: cli.input_text_color,
        highlight_color: cli.highlight_color,
        title_color: cli.header_title_color,
        keybinds: &cli.keybinds,
    }
}

fn query_line<'a>(state: &'a State, cli: &'a Opts, inline_count: bool) -> Line<'a> {
    let mut spans = Vec::new();
    if inline_count {
        spans.extend([
            Span::styled("(", Style::default().fg(cli.input_text_color)),
            Span::styled(
                state
                    .selected
                    .map_or(0, |selected| selected + 1)
                    .to_string(),
                Style::default().fg(cli.highlight_color),
            ),
            Span::styled("/", Style::default().fg(cli.input_text_color)),
            Span::styled(
                state.shown.len().to_string(),
                Style::default().fg(cli.input_text_color),
            ),
            Span::styled(") ", Style::default().fg(cli.input_text_color)),
        ]);
    }
    if cli.show_input_prompt {
        spans.extend([
            Span::styled(">", Style::default().fg(cli.highlight_color)),
            Span::styled("> ", Style::default().fg(cli.input_text_color)),
        ]);
    }
    spans.extend([
        Span::styled(&state.query, Style::default().fg(cli.input_text_color)),
        Span::styled(&cli.cursor, Style::default().fg(cli.highlight_color)),
    ]);
    Line::from(spans)
}

fn horizontal_scroll(line: &Line<'_>, available_width: usize) -> u16 {
    line.width()
        .saturating_sub(available_width)
        .min(usize::from(u16::MAX)) as u16
}

fn command_areas(inner: Rect) -> (Rect, Option<Rect>) {
    if inner.height < 2 {
        return (inner, None);
    }
    (
        Rect::new(inner.x, inner.y, inner.width, inner.height - 1),
        Some(Rect::new(
            inner.x,
            inner.y + inner.height - 1,
            inner.width,
            1,
        )),
    )
}

#[cfg(test)]
mod tests {
    use super::{command_areas, horizontal_scroll, render};
    use crate::cli::{MatchMode, Opts, PinnedOrderMode, RankingMode};
    use crate::core::state::State;
    use crate::ui::InputPanelStyle;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;
    use ratatui::text::Line;

    #[test]
    fn command_style_reserves_the_last_inner_row_for_status() {
        assert_eq!(
            command_areas(Rect::new(2, 4, 20, 3)),
            (Rect::new(2, 4, 20, 2), Some(Rect::new(2, 6, 20, 1)))
        );
        assert_eq!(
            command_areas(Rect::new(2, 4, 20, 1)),
            (Rect::new(2, 4, 20, 1), None)
        );
    }

    #[test]
    fn input_scroll_is_saturating() {
        assert_eq!(horizontal_scroll(&Line::from("abcdef"), 4), 2);
        assert_eq!(horizontal_scroll(&Line::from("abc"), 4), 0);
    }

    #[test]
    fn command_style_renders_an_accent_query_and_footer() {
        let mut state = State::new(
            Vec::new(),
            MatchMode::Fuzzy,
            Default::default(),
            3,
            RankingMode::Frecency,
            PinnedOrderMode::Ranking,
            Default::default(),
        );
        state.query = "fire".to_string();
        let cli = Opts {
            input_panel_style: InputPanelStyle::Command,
            show_input_border: false,
            show_panel_titles: false,
            show_input_prompt: false,
            keybinds: toml::from_str(
                r#"
select = [{ key = "y", modifiers = "ctrl" }]
exit = ["q"]
"#,
            )
            .expect("keybinds should parse"),
            ..Opts::default()
        };
        let backend = TestBackend::new(40, 3);
        let mut terminal = Terminal::new(backend).expect("test terminal should initialize");

        terminal
            .draw(|frame| render(frame, &state, &cli, frame.area()))
            .expect("command input should render");
        let buffer = terminal.backend().buffer();

        assert_eq!(buffer[(0, 0)].symbol(), "▍");
        assert_eq!(buffer[(0, 2)].fg, cli.highlight_color);
        assert_eq!(buffer[(2, 0)].symbol(), "f");
        assert_eq!(buffer[(0, 2)].symbol(), "f");
        let footer =
            (0..40)
                .map(|x| buffer[(x, 2)].symbol())
                .fold(String::new(), |mut line, symbol| {
                    line.push_str(symbol);
                    line
                });
        assert!(footer.contains("ctrl+y launch"));
        assert!(footer.contains("q close"));
        assert!(!footer.contains("enter launch"));
    }
}
