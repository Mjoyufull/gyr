//! Bounded command output and process-group cancellation.

use super::MAX_PREVIEW_BYTES;
use eyre::Result;
use std::process::Stdio;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::sync::mpsc;

pub(in crate::modes::dmenu) struct CommandOutput {
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
    pub(super) status: String,
    pub(super) success: bool,
    pub(super) stdout_truncated: bool,
    pub(super) stderr_truncated: bool,
}

impl CommandOutput {
    pub(super) fn truncated(&self) -> bool {
        self.stdout_truncated || self.stderr_truncated
    }
}

pub(super) async fn run_preview_command(
    command: &str,
    item: &str,
    query: Option<&str>,
    input_ordinal: usize,
) -> Result<CommandOutput, String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let mut process = tokio::process::Command::new(shell);
    process
        .args(["-c", command])
        .env("FSEL_PREVIEW_ITEM", item)
        .env("FSEL_PREVIEW_ORDINAL", input_ordinal.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if let Some(query) = query {
        process.env("FSEL_PREVIEW_QUERY", query);
    } else {
        process.env_remove("FSEL_PREVIEW_QUERY");
    }
    #[cfg(unix)]
    process.process_group(0);

    let mut child = process
        .spawn()
        .map_err(|error| format!("Failed to start preview command: {error}"))?;
    #[cfg(unix)]
    let mut process_group = ProcessGroupGuard::new(child.id());

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture preview stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to capture preview stderr".to_string())?;

    let (limit_tx, mut limit_rx) = mpsc::channel(2);
    let stdout_task = tokio::spawn(read_limited(stdout, limit_tx.clone()));
    let stderr_task = tokio::spawn(read_limited(stderr, limit_tx.clone()));
    drop(limit_tx);
    let status = tokio::select! {
        status = child.wait() => status,
        Some(()) = limit_rx.recv() => {
            #[cfg(unix)]
            process_group.terminate();
            let _ = child.start_kill();
            child.wait().await
        }
    };
    #[cfg(unix)]
    process_group.terminate();
    let status = status.map_err(|error| format!("Preview command failed: {error}"))?;
    let (stdout_result, stderr_result) = tokio::join!(stdout_task, stderr_task);
    let (stdout, stdout_truncated) = stdout_result
        .map_err(|error| format!("Preview output reader failed: {error}"))?
        .map_err(|error| format!("Failed to read preview output: {error}"))?;
    let (stderr, stderr_truncated) = stderr_result
        .map_err(|error| format!("Preview error reader failed: {error}"))?
        .map_err(|error| format!("Failed to read preview error output: {error}"))?;

    Ok(CommandOutput {
        stdout,
        stderr,
        status: status.to_string(),
        success: status.success(),
        stdout_truncated,
        stderr_truncated,
    })
}

#[cfg(unix)]
struct ProcessGroupGuard {
    pgid: Option<rustix::process::Pid>,
}

#[cfg(unix)]
impl ProcessGroupGuard {
    fn new(pid: Option<u32>) -> Self {
        Self {
            pgid: pid
                .and_then(|pid| i32::try_from(pid).ok())
                .and_then(rustix::process::Pid::from_raw),
        }
    }

    fn terminate(&mut self) {
        if let Some(pgid) = self.pgid.take() {
            let _ = rustix::process::kill_process_group(pgid, rustix::process::Signal::KILL);
        }
    }
}

#[cfg(unix)]
impl Drop for ProcessGroupGuard {
    fn drop(&mut self) {
        self.terminate();
    }
}

async fn read_limited(
    reader: impl AsyncRead + Unpin,
    limit_tx: mpsc::Sender<()>,
) -> std::io::Result<(Vec<u8>, bool)> {
    read_limited_to(reader, MAX_PREVIEW_BYTES, limit_tx).await
}

pub(super) async fn read_limited_to(
    mut reader: impl AsyncRead + Unpin,
    limit: u64,
    limit_tx: mpsc::Sender<()>,
) -> std::io::Result<(Vec<u8>, bool)> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 8192];
    let mut truncated = false;
    loop {
        let read = reader.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        let remaining = limit.saturating_sub(bytes.len() as u64) as usize;
        bytes.extend_from_slice(&buffer[..read.min(remaining)]);
        if read > remaining && !truncated {
            truncated = true;
            let _ = limit_tx.try_send(());
        }
    }
    Ok((bytes, truncated))
}
