//! Clipboard selection, provider lifecycle, deletion, and tagging.

use super::CclipItem;
use eyre::{Result, eyre};
use std::io::{self, Cursor, Read};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const CLIPBOARD_PROVIDER_STARTUP_TIMEOUT: Duration = Duration::from_millis(500);

#[derive(Debug, Eq, PartialEq)]
enum ClipboardProviderState {
    Exited,
    StillRunning,
}

impl CclipItem {
    /// Copy this item back to the clipboard (Wayland)
    fn copy_to_clipboard_wayland(&self) -> Result<()> {
        if command_is_available("wl-copy")
            && let Ok(()) = self.copy_to_clipboard_wayland_with_wl_copy()
        {
            return Ok(());
        }

        let mut child = Command::new("cclip")
            .args(["copy", &self.rowid])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        wait_for_clipboard_provider_start(
            &mut child,
            "cclip copy",
            CLIPBOARD_PROVIDER_STARTUP_TIMEOUT,
        )?;

        Ok(())
    }

    fn copy_to_clipboard_wayland_with_wl_copy(&self) -> Result<()> {
        let mut cclip_child = Command::new("cclip")
            .args(["get", &self.rowid])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let cclip_stdout = cclip_child
            .stdout
            .take()
            .ok_or_else(|| eyre!("failed to capture cclip stdout"))?;

        let copy_result = copy_reader_with_wl_copy(
            cclip_stdout,
            &self.mime_type,
            CLIPBOARD_PROVIDER_STARTUP_TIMEOUT,
        );
        let cclip_output = cclip_child.wait_with_output()?;
        if !cclip_output.status.success() {
            return Err(eyre!(
                "cclip get failed: {}",
                String::from_utf8_lossy(&cclip_output.stderr)
            ));
        }
        require_original_copy_data(copy_result?)
    }

    /// Copy this item back to the clipboard.
    pub fn copy_to_clipboard(&self) -> Result<()> {
        if std::env::var("WAYLAND_DISPLAY").is_err() {
            return Err(eyre!("cclip mode requires a Wayland session"));
        }

        self.copy_to_clipboard_wayland()
    }

    /// Copy rendered HTML as plain text, preserving the established path for every other MIME type.
    pub fn copy_rendered_to_clipboard(&self) -> Result<()> {
        if !super::html::is_html_mime(&self.mime_type) {
            return self.copy_to_clipboard();
        }
        let rendered_content =
            rendered_clipboard_content(&self.mime_type, self.get_content_for_preview()?)?
                .ok_or_else(|| eyre!("failed to render HTML clipboard content"))?;

        if std::env::var("WAYLAND_DISPLAY").is_err() {
            return Err(eyre!("cclip mode requires a Wayland session"));
        }
        if command_is_available("wl-copy")
            && copy_reader_with_wl_copy(
                Cursor::new(rendered_content.clone()),
                "text/plain;charset=utf-8",
                CLIPBOARD_PROVIDER_STARTUP_TIMEOUT,
            )
            .is_ok()
        {
            return Ok(());
        }
        copy_bytes_with_cclip(rendered_content, CLIPBOARD_PROVIDER_STARTUP_TIMEOUT)
    }
}

fn rendered_clipboard_content(mime_type: &str, bytes: Vec<u8>) -> Result<Option<Vec<u8>>> {
    if !super::html::is_html_mime(mime_type) {
        return Ok(None);
    }

    let html =
        super::html::decode_text_bytes(mime_type, &bytes).map_err(|message| eyre!(message))?;
    Ok(Some(
        super::html::text_for_display(mime_type, &html).into_bytes(),
    ))
}

fn copy_reader_with_wl_copy(
    source: impl Read + Send + 'static,
    mime_type: &str,
    timeout: Duration,
) -> Result<u64> {
    let child = Command::new("wl-copy")
        .args(["--type", mime_type])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    pipe_to_clipboard_provider(source, child, "wl-copy", timeout)
}

fn copy_bytes_with_cclip(bytes: Vec<u8>, timeout: Duration) -> Result<()> {
    let child = Command::new("cclip")
        .args(["copy", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    pipe_to_clipboard_provider(Cursor::new(bytes), child, "cclip copy -", timeout).map(|_| ())
}

fn pipe_to_clipboard_provider(
    mut source: impl Read + Send + 'static,
    mut child: Child,
    command: &str,
    timeout: Duration,
) -> Result<u64> {
    let Some(child_stdin) = child.stdin.take() else {
        terminate_and_reap(&mut child);
        return Err(eyre!("failed to open {command} stdin"));
    };

    let pipe_handle = std::thread::spawn(move || {
        let mut sink = child_stdin;
        io::copy(&mut source, &mut sink)
    });
    let copy_result = match pipe_handle.join() {
        Ok(result) => result,
        Err(_) => {
            terminate_and_reap(&mut child);
            return Err(eyre!("clipboard pipe thread panicked"));
        }
    };
    let copied_bytes = match copy_result {
        Ok(copied_bytes) => copied_bytes,
        Err(error) => {
            terminate_and_reap(&mut child);
            return Err(error.into());
        }
    };
    wait_for_clipboard_provider_start(&mut child, command, timeout)?;
    Ok(copied_bytes)
}

fn terminate_and_reap(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn require_original_copy_data(copied_bytes: u64) -> Result<()> {
    if copied_bytes == 0 {
        return Err(eyre!("cclip get returned no data"));
    }
    Ok(())
}

fn wait_for_clipboard_provider_start(
    child: &mut Child,
    command: &str,
    timeout: Duration,
) -> Result<ClipboardProviderState> {
    let deadline = Instant::now() + timeout;

    loop {
        if let Some(status) = child.try_wait()? {
            if status.success() {
                return Ok(ClipboardProviderState::Exited);
            }
            return Err(eyre!("{} failed", command));
        }

        if Instant::now() >= deadline {
            return Ok(ClipboardProviderState::StillRunning);
        }

        std::thread::sleep(Duration::from_millis(10));
    }
}

fn command_is_available(command: &str) -> bool {
    Command::new(command)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Tag a cclip item using cclip's tag command
pub fn tag_item(rowid: &str, tag: &str) -> Result<()> {
    let output = Command::new("cclip").args(["tag", rowid, tag]).output()?;

    if !output.status.success() {
        return Err(eyre!(
            "Failed to tag item: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

/// Remove tag from a cclip item. If `tag` is `None`, all tags are removed.
pub fn untag_item(rowid: &str, tag: Option<&str>) -> Result<()> {
    let mut args = vec!["tag", "-d", rowid];
    if let Some(tag) = tag {
        args.push(tag);
    }

    let output = Command::new("cclip").args(&args).output()?;

    if !output.status.success() {
        return Err(eyre!(
            "Failed to remove tag: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

/// Delete a specific cclip item by rowid
pub fn delete_item(rowid: &str) -> Result<()> {
    let output = Command::new("cclip").args(["delete", rowid]).output()?;

    if !output.status.success() {
        return Err(eyre!(
            "Failed to delete item: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

/// Wipe all tags from all cclip entries (cclip 3.2.0+)
pub fn wipe_all_tags() -> Result<()> {
    let output = Command::new("cclip").args(["tags", "wipe"]).output()?;

    if !output.status.success() {
        return Err(eyre!(
            "Failed to wipe tags: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

/// Delete a specific tag from cclip (cclip 3.2.0+)
#[allow(dead_code)]
pub fn delete_tag(tag: &str) -> Result<()> {
    let output = Command::new("cclip")
        .args(["tags", "delete", tag])
        .output()?;

    if !output.status.success() {
        return Err(eyre!(
            "Failed to delete tag '{}': {}",
            tag,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        ClipboardProviderState, pipe_to_clipboard_provider, rendered_clipboard_content,
        require_original_copy_data, wait_for_clipboard_provider_start,
    };
    use std::io::{self, Read};
    use std::process::{Command, Stdio};
    use std::time::Duration;

    #[test]
    fn provider_start_wait_returns_while_clipboard_owner_stays_running() {
        let mut child = Command::new("sh")
            .args(["-c", "sleep 1"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("test process should spawn");

        let state = wait_for_clipboard_provider_start(
            &mut child,
            "test-provider",
            Duration::from_millis(20),
        )
        .expect("running provider should be accepted");

        assert_eq!(state, ClipboardProviderState::StillRunning);
        child.kill().expect("test process should be killable");
        let _ = child.wait();
    }

    #[test]
    fn provider_start_wait_rejects_fast_failures() {
        let mut child = Command::new("sh")
            .args(["-c", "exit 7"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("test process should spawn");

        let result =
            wait_for_clipboard_provider_start(&mut child, "test-provider", Duration::from_secs(1));

        assert!(result.is_err());
    }

    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("source failed"))
        }
    }

    #[test]
    fn provider_is_terminated_when_the_pipe_fails() {
        let child = Command::new("sh")
            .args(["-c", "cat >/dev/null; sleep 30"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("test provider should spawn");

        let result = pipe_to_clipboard_provider(
            FailingReader,
            child,
            "test-provider",
            Duration::from_secs(1),
        );

        assert!(result.is_err());
    }

    #[test]
    fn rendered_copy_converts_html_to_plain_text() {
        let rendered =
            rendered_clipboard_content("text/html", b"<p>Hello &amp; goodbye</p>".to_vec())
                .expect("valid HTML should render");

        assert_eq!(rendered.as_deref(), Some(b"Hello & goodbye".as_slice()));
    }

    #[test]
    fn rendered_copy_accepts_empty_visible_html() {
        let rendered = rendered_clipboard_content("text/html", b"<style>x {}</style>".to_vec())
            .expect("valid empty HTML should render");

        assert_eq!(rendered, Some(Vec::new()));
    }

    #[test]
    fn original_copy_still_rejects_empty_content() {
        assert!(require_original_copy_data(0).is_err());
        assert!(require_original_copy_data(1).is_ok());
    }

    #[test]
    fn rendered_copy_honors_the_declared_charset() {
        let rendered =
            rendered_clipboard_content("text/html;charset=iso-8859-1", b"<p>caf\xe9</p>".to_vec())
                .expect("declared HTML charset should render");

        assert_eq!(rendered.as_deref(), Some("café".as_bytes()));
    }

    #[test]
    fn rendered_copy_leaves_non_html_on_the_original_copy_path() {
        let rendered = rendered_clipboard_content("text/plain", b"plain".to_vec())
            .expect("plain text should be accepted");

        assert_eq!(rendered, None);
    }
}
