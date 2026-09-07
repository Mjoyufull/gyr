//! Classic and command input text for filtering and tag prompts.

use super::super::state::CclipOptions;
use crate::ui::{DmenuUI, TagMode};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

pub(super) struct InputLines {
    pub(super) classic: Line<'static>,
    pub(super) command: Line<'static>,
    pub(super) title: &'static str,
    pub(super) primary_action: &'static str,
    pub(super) exit_action: &'static str,
}

pub(super) fn input_lines(ui: &DmenuUI<'_>, options: &CclipOptions) -> InputLines {
    let (primary_action, exit_action) = command_actions(&ui.tag_mode);
    match &ui.tag_mode {
        TagMode::PromptingTagName { input, .. } => prompt_lines(
            "Tag: ",
            input,
            options,
            None,
            " Tag Name ",
            primary_action,
            exit_action,
        ),
        TagMode::PromptingTagEmoji { input, .. } => prompt_lines(
            "Emoji: ",
            input,
            options,
            Some(" (or blank)"),
            " Tag Emoji ",
            primary_action,
            exit_action,
        ),
        TagMode::PromptingTagColor { input, .. } => prompt_lines(
            "Color: ",
            input,
            options,
            Some(" (hex/name or blank)"),
            " Tag Color ",
            primary_action,
            exit_action,
        ),
        TagMode::RemovingTag { input, .. } => prompt_lines(
            "Remove: ",
            input,
            options,
            Some(" (blank = all)"),
            " Remove Tag ",
            primary_action,
            exit_action,
        ),
        TagMode::Normal => InputLines {
            classic: filter_line(ui, options, true),
            command: filter_line(ui, options, false),
            title: " Filter ",
            primary_action,
            exit_action,
        },
    }
}

fn command_actions(tag_mode: &TagMode) -> (&'static str, &'static str) {
    let primary = match tag_mode {
        TagMode::Normal => "copy",
        TagMode::PromptingTagName { .. } | TagMode::PromptingTagEmoji { .. } => "continue",
        TagMode::PromptingTagColor { .. } => "apply",
        TagMode::RemovingTag { .. } => "remove",
    };
    let exit = if matches!(tag_mode, TagMode::Normal) {
        "close"
    } else {
        "cancel"
    };
    (primary, exit)
}

fn prompt_lines(
    label: &'static str,
    input: &str,
    options: &CclipOptions,
    hint: Option<&'static str>,
    title: &'static str,
    primary_action: &'static str,
    exit_action: &'static str,
) -> InputLines {
    InputLines {
        classic: prompt_line(label, input, options, hint),
        command: prompt_line(label, input, options, hint),
        title,
        primary_action,
        exit_action,
    }
}

fn prompt_line(
    label: &'static str,
    input: &str,
    options: &CclipOptions,
    hint: Option<&'static str>,
) -> Line<'static> {
    let mut spans = vec![
        Span::styled(label, Style::default().fg(options.highlight_color)),
        Span::styled(
            input.to_string(),
            Style::default().fg(options.input_text_color),
        ),
        Span::styled(
            options.cursor.clone(),
            Style::default().fg(options.highlight_color),
        ),
    ];

    if let Some(hint) = hint {
        spans.push(Span::styled(
            hint,
            Style::default()
                .fg(options.input_text_color)
                .add_modifier(Modifier::DIM),
        ));
    }

    Line::from(spans)
}

fn filter_line(ui: &DmenuUI<'_>, options: &CclipOptions, include_count: bool) -> Line<'static> {
    let mut spans = Vec::new();
    if include_count && options.show_input_count {
        spans.extend([
            Span::styled("(", Style::default().fg(options.input_text_color)),
            Span::styled(
                ui.selected.map_or(0, |selected| selected + 1).to_string(),
                Style::default().fg(options.highlight_color),
            ),
            Span::styled("/", Style::default().fg(options.input_text_color)),
            Span::styled(
                ui.shown.len().to_string(),
                Style::default().fg(options.input_text_color),
            ),
            Span::styled(") ", Style::default().fg(options.input_text_color)),
        ]);
    }
    if options.show_input_prompt {
        spans.extend([
            Span::styled(">", Style::default().fg(options.highlight_color)),
            Span::styled("> ", Style::default().fg(options.input_text_color)),
        ]);
    }
    spans.extend([
        Span::styled(
            ui.query.clone(),
            Style::default().fg(options.input_text_color),
        ),
        Span::styled(
            options.cursor.clone(),
            Style::default().fg(options.highlight_color),
        ),
    ]);
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::command_actions;
    use crate::ui::TagMode;

    #[test]
    fn command_actions_follow_the_active_tag_step() {
        let name = TagMode::PromptingTagName {
            input: String::new(),
            selected_item: None,
            available_tags: Vec::new(),
            selected_tag: None,
        };
        let color = TagMode::PromptingTagColor {
            tag_name: "work".to_string(),
            emoji: None,
            input: String::new(),
            selected_item: None,
        };
        let removal = TagMode::RemovingTag {
            input: String::new(),
            tags: Vec::new(),
            selected: None,
            selected_item: None,
        };

        assert_eq!(command_actions(&TagMode::Normal), ("copy", "close"));
        assert_eq!(command_actions(&name), ("continue", "cancel"));
        assert_eq!(command_actions(&color), ("apply", "cancel"));
        assert_eq!(command_actions(&removal), ("remove", "cancel"));
    }
}
