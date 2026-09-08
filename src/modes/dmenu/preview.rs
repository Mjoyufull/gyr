//! Latest-selection preview commands and image replacement.

mod command;
mod expand;
#[cfg(test)]
mod tests;

use command::{CommandOutput, run_preview_command};
use expand::expand_preview_command;

use crate::ui::{DmenuUI, GraphicsAdapter, ImageManager};
use eyre::Result;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui_image::protocol::StatefulProtocol;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

const MAX_PREVIEW_BYTES: u64 = 32 * 1024 * 1024;

pub(super) struct PreviewRuntime {
    command_template: Option<String>,
    expose_query: bool,
    content: PreviewContent,
    image_manager: ImageManager,
    active_request: Option<JoinHandle<()>>,
    decode_tx: Option<mpsc::Sender<()>>,
    decode_request: Arc<Mutex<Option<DecodeRequest>>>,
    decode_worker: Option<std::thread::JoinHandle<()>>,
    current_signature: Option<PreviewSignature>,
    generation: u64,
    result_tx: mpsc::Sender<PreviewResult>,
    result_rx: mpsc::Receiver<PreviewResult>,
}

#[derive(Debug, PartialEq, Eq)]
struct PreviewSignature {
    selected: usize,
    input_ordinal: usize,
    item: String,
    query: String,
}

enum PreviewContent {
    Empty,
    Loading,
    Text(String),
    Image(String),
}

struct DecodeRequest {
    generation: u64,
    key: String,
    bytes: Vec<u8>,
}

pub(super) enum PreviewResult {
    Command {
        generation: u64,
        output: Result<CommandOutput, String>,
    },
    Image {
        generation: u64,
        key: String,
        protocol: Result<Box<StatefulProtocol>, String>,
    },
}

impl PreviewRuntime {
    pub(super) fn new(
        command_template: Option<String>,
        adapter: GraphicsAdapter,
        expose_query: bool,
    ) -> Self {
        let (result_tx, result_rx) = mpsc::channel(4);
        let (decode_tx, mut decode_rx) = mpsc::channel::<()>(1);
        let decode_request = Arc::new(Mutex::new(None::<DecodeRequest>));
        let worker_request = Arc::clone(&decode_request);
        let picker = adapter.picker();
        let decode_result_tx = result_tx.clone();
        let decode_worker = std::thread::spawn(move || {
            while decode_rx.blocking_recv().is_some() {
                while decode_rx.try_recv().is_ok() {}
                let request = worker_request
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .take();
                let Some(request) = request else {
                    continue;
                };
                let protocol =
                    ImageManager::prepare_image_bytes_blocking(picker.clone(), request.bytes)
                        .map(Box::new)
                        .map_err(|error| format!("Failed to decode preview image: {error}"));
                if decode_result_tx
                    .blocking_send(PreviewResult::Image {
                        generation: request.generation,
                        key: request.key,
                        protocol,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            command_template,
            expose_query,
            content: PreviewContent::Empty,
            image_manager: ImageManager::new(adapter.picker()),
            active_request: None,
            decode_tx: Some(decode_tx),
            decode_request,
            decode_worker: Some(decode_worker),
            current_signature: None,
            generation: 0,
            result_tx,
            result_rx,
        }
    }

    pub(super) fn is_enabled(&self) -> bool {
        self.command_template.is_some()
    }

    pub(super) fn title(&self) -> &'static str {
        if self.is_enabled() {
            " Preview "
        } else {
            " Content "
        }
    }

    pub(super) fn text_lines(&self) -> Option<Vec<Line<'static>>> {
        match &self.content {
            PreviewContent::Empty => Some(Vec::new()),
            PreviewContent::Loading => Some(vec![Line::from("Loading preview…")]),
            PreviewContent::Text(text) => Some(
                text.lines()
                    .map(|line| Line::from(line.to_string()))
                    .collect(),
            ),
            PreviewContent::Image(_) => None,
        }
    }

    pub(super) fn request_if_changed(&mut self, ui: &DmenuUI<'_>) {
        let Some(command_template) = self.command_template.as_deref() else {
            return;
        };
        let Some(selected) = ui.selected else {
            self.clear_request();
            return;
        };
        let Some(item) = ui.shown.get(selected) else {
            self.clear_request();
            return;
        };

        let signature = PreviewSignature {
            selected,
            input_ordinal: item.line_number.saturating_sub(1),
            item: item.original_line.clone(),
            query: signature_query(self.expose_query, &ui.query),
        };
        if self.current_signature.as_ref() == Some(&signature) {
            return;
        }

        if let Some(task) = self.active_request.take() {
            task.abort();
        }
        self.generation = self.generation.wrapping_add(1);
        if !matches!(self.content, PreviewContent::Image(_)) {
            self.content = PreviewContent::Loading;
        }

        let generation = self.generation;
        let command = match expand_preview_command(command_template) {
            Ok(command) => command,
            Err(error) => {
                self.content = PreviewContent::Text(error);
                self.current_signature = Some(signature);
                return;
            }
        };
        let item = signature.item.clone();
        let query = self.expose_query.then(|| signature.query.clone());
        let input_ordinal = signature.input_ordinal;
        let result_tx = self.result_tx.clone();
        self.active_request = Some(tokio::spawn(async move {
            let output =
                run_preview_command(&command, &item, query.as_deref(), input_ordinal).await;
            let _ = result_tx
                .send(PreviewResult::Command { generation, output })
                .await;
        }));
        self.current_signature = Some(signature);
    }

    pub(super) async fn next_result(&mut self) -> Option<PreviewResult> {
        self.result_rx.recv().await
    }

    pub(super) fn apply_result(&mut self, result: PreviewResult) {
        let PreviewResult::Command { generation, output } = result else {
            return self.apply_image_result(result);
        };
        if generation != self.generation {
            return;
        }
        self.active_request = None;

        let output = match output {
            Ok(output) => output,
            Err(error) => {
                self.content = PreviewContent::Text(error);
                return;
            }
        };

        if should_report_command_failure(&output) {
            let stderr = output_text(&output.stderr);
            let mut text = if stderr.trim().is_empty() {
                format!("Preview command exited with {}", output.status)
            } else {
                stderr
            };
            append_truncation_notice(&mut text, output.truncated());
            self.content = PreviewContent::Text(text);
            return;
        }

        let image_key = format!("dmenu-preview-{generation}");
        if let Some(message) = truncated_image_message(&output) {
            self.content = PreviewContent::Text(message);
            return;
        }
        if ImageManager::recognizes_image_bytes(&output.stdout) {
            let request = DecodeRequest {
                generation,
                key: image_key,
                bytes: output.stdout,
            };
            *self
                .decode_request
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(request);
            if let Some(sender) = &self.decode_tx {
                let _ = sender.try_send(());
            }
            return;
        }

        let mut text = output_text(&output.stdout);
        append_truncation_notice(&mut text, output.truncated());
        self.content = PreviewContent::Text(text);
    }

    fn apply_image_result(&mut self, result: PreviewResult) {
        let PreviewResult::Image {
            generation,
            key,
            protocol,
        } = result
        else {
            return;
        };
        if generation != self.generation {
            return;
        }
        self.active_request = None;
        match protocol {
            Ok(protocol) => {
                self.image_manager.clear();
                self.image_manager.insert_protocol(key.clone(), *protocol);
                self.content = PreviewContent::Image(key);
            }
            Err(error) => self.content = PreviewContent::Text(error),
        }
    }

    pub(super) fn render_image(&mut self, frame: &mut Frame, area: Rect) -> Result<bool> {
        let PreviewContent::Image(key) = &self.content else {
            return Ok(false);
        };
        let key = key.clone();
        if self.image_manager.render_cached(frame, &key, area)? {
            Ok(true)
        } else {
            self.content = PreviewContent::Text("Failed to render preview image".to_string());
            Ok(false)
        }
    }

    pub(super) fn clear_request(&mut self) {
        if let Some(task) = self.active_request.take() {
            task.abort();
        }
        self.generation = self.generation.wrapping_add(1);
        self.current_signature = None;
        self.content = PreviewContent::Empty;
        self.decode_request
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
    }

    pub(super) async fn shutdown(&mut self) {
        if let Some(task) = self.active_request.take() {
            task.abort();
            let _ = task.await;
        }
        self.clear_request();
        self.decode_tx.take();
        self.result_rx.close();
        if let Some(worker) = self.decode_worker.take() {
            // Decoding is not interruptible; at most one bounded image remains per panel.
            let join = tokio::task::spawn_blocking(move || worker.join());
            let _ = tokio::time::timeout(std::time::Duration::from_secs(1), join).await;
        }
    }
}

impl Drop for PreviewRuntime {
    fn drop(&mut self) {
        if let Some(task) = self.active_request.take() {
            task.abort();
        }
    }
}

fn output_text(bytes: &[u8]) -> String {
    let stripped = strip_ansi_escapes::strip(bytes);
    String::from_utf8_lossy(&stripped).trim_end().to_string()
}

fn append_truncation_notice(text: &mut String, truncated: bool) {
    if truncated {
        text.push_str("\n\n[preview output truncated]");
    }
}

fn truncated_image_message(output: &CommandOutput) -> Option<String> {
    (output.stdout_truncated && ImageManager::recognizes_image_bytes(&output.stdout)).then(|| {
        format!(
            "Preview image exceeds the {} MiB output limit",
            MAX_PREVIEW_BYTES / (1024 * 1024)
        )
    })
}

fn should_report_command_failure(output: &CommandOutput) -> bool {
    !output.success && output.stdout.is_empty()
}

fn signature_query(expose_query: bool, query: &str) -> String {
    if expose_query {
        query.to_string()
    } else {
        String::new()
    }
}
