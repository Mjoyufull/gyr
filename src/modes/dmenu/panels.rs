//! Named command previews with a fixed bound on processes, decodes, and retained output.
//! Each panel reuses the existing preview cancellation and password-isolation boundary.

use super::preview::{PreviewResult, PreviewRuntime};
use crate::ui::panels::PanelSide;
use crate::ui::{DmenuUI, GraphicsAdapter};
use serde::Deserialize;

/// One named dmenu command panel. At most three supplement the primary preview.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DmenuPanel {
    /// Stable panel name used as its title and movement identifier.
    pub name: String,
    /// Trusted shell command, with the same placeholders as `--preview`.
    pub command: String,
    /// Docking edge relative to the remaining result area.
    pub position: PanelSide,
    /// Percentage of the available width or height; zero hides the panel.
    pub size: u16,
}

impl DmenuPanel {
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        let mut fields = value.splitn(4, ':');
        let name = fields.next().unwrap_or_default().to_owned();
        let position = fields
            .next()
            .ok_or("panel needs NAME:SIDE:PERCENT:COMMAND")?
            .parse()?;
        let size = fields
            .next()
            .ok_or("panel needs a size")?
            .parse()
            .map_err(|_| "panel size must be an integer")?;
        let command = fields.next().ok_or("panel needs a command")?.to_owned();
        let panel = Self {
            name,
            command,
            position,
            size,
        };
        panel.validate()?;
        Ok(panel)
    }

    fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() || self.name.len() > 64 || self.name.chars().any(char::is_control) {
            return Err("panel name must contain 1-64 bytes without control characters".to_owned());
        }
        if self.name == "preview" || self.name == "input" || self.name == "items" {
            return Err("preview, input, and items are reserved panel names".to_owned());
        }
        if self.size > 90 {
            return Err("panel size must be between 0 and 90 percent".to_owned());
        }
        if self.command.trim().is_empty() {
            return Err("panel command must not be empty".to_owned());
        }
        Ok(())
    }
}

pub(crate) fn validate(panels: &[DmenuPanel]) -> Result<(), String> {
    if panels.len() > 3 {
        return Err("at most three custom dmenu panels are supported".to_owned());
    }
    for (index, panel) in panels.iter().enumerate() {
        panel.validate()?;
        if panels[..index]
            .iter()
            .any(|previous| previous.name == panel.name)
        {
            return Err(format!("duplicate panel name: {}", panel.name));
        }
    }
    Ok(())
}

pub(super) struct PreviewPanels {
    pub(super) primary: PreviewRuntime,
    pub(super) custom: Vec<PreviewRuntime>,
}

impl PreviewPanels {
    pub(super) fn new(
        command: Option<String>,
        panels: &[DmenuPanel],
        adapter: GraphicsAdapter,
        expose_query: bool,
    ) -> Self {
        Self {
            primary: PreviewRuntime::new(command, adapter, expose_query),
            custom: panels
                .iter()
                .map(|panel| {
                    PreviewRuntime::new(Some(panel.command.clone()), adapter, expose_query)
                })
                .collect(),
        }
    }

    pub(super) fn request(&mut self, ui: &DmenuUI<'_>, options: &super::options::DmenuOptions) {
        if options
            .panels
            .info_size
            .unwrap_or(options.content_panel_height_percent)
            > 0
            && (!options.hide_before_typing || !ui.query.is_empty())
        {
            self.primary.request_if_changed(ui);
        } else {
            self.primary.clear_request();
        }
        for (runtime, panel) in self.custom.iter_mut().zip(&options.custom_panels) {
            if panel.size > 0 && (!options.hide_before_typing || !ui.query.is_empty()) {
                runtime.request_if_changed(ui);
            } else {
                runtime.clear_request();
            }
        }
    }

    pub(super) async fn next_result(&mut self) -> (usize, Option<PreviewResult>) {
        let receivers = std::iter::once(&mut self.primary)
            .chain(self.custom.iter_mut())
            .map(|runtime| Box::pin(runtime.next_result()));
        let (result, index, _) = futures::future::select_all(receivers).await;
        (index, result)
    }

    pub(super) fn apply_result(&mut self, index: usize, result: PreviewResult) {
        if index == 0 {
            self.primary.apply_result(result);
        } else if let Some(runtime) = self.custom.get_mut(index - 1) {
            runtime.apply_result(result);
        }
    }

    pub(super) async fn shutdown(&mut self) {
        futures::future::join_all(
            std::iter::once(&mut self.primary)
                .chain(self.custom.iter_mut())
                .map(PreviewRuntime::shutdown),
        )
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_command_preserves_colons_and_rejects_duplicate_names() {
        let panel = DmenuPanel::parse("details:right:30:printf '%s:%s' {} {n}").unwrap();
        assert_eq!(panel.command, "printf '%s:%s' {} {n}");
        assert!(validate(&[panel.clone(), panel]).is_err());
        assert!(DmenuPanel::parse("preview:left:30:cat {}").is_err());
        assert!(DmenuPanel::parse("x:left:91:cat {}").is_err());
    }
}
