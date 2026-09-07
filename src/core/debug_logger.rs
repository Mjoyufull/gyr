//! Optional diagnostic log output and session timestamps.

use crate::desktop::App;
use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Instant;

static LOG_FILE: OnceLock<PathBuf> = OnceLock::new();
static SESSION_START: OnceLock<Instant> = OnceLock::new();

pub fn init_test_log() -> std::io::Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let log_dir = PathBuf::from(home).join(".config/fsel/logs");

    // Create log directory, log error if it fails
    if let Err(e) = create_dir_all(&log_dir) {
        eprintln!(
            "Warning: Failed to create log directory {:?}: {}",
            log_dir, e
        );
        return Err(e);
    }

    // Generate timestamped filename with PID
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let timestamp = now
        .format(
            &time::format_description::parse_borrowed::<1>(
                "[year][month][day]-[hour][minute][second]",
            )
            .expect("literal timestamp format is valid"),
        )
        .unwrap();
    let pid = std::process::id();
    let path = log_dir.join(format!("fsel-debug-{}-pid{}.log", timestamp, pid));

    // Clear the log on start
    let mut file = match OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Warning: Failed to create log file {:?}: {}", path, e);
            return Err(e);
        }
    };

    let session_start = Instant::now();
    SESSION_START.set(session_start).ok();

    writeln!(file, "=== FSEL DEBUG SESSION STARTED ===")?;
    writeln!(
        file,
        "Timestamp: {}",
        time::OffsetDateTime::now_local()
            .unwrap_or_else(|_| time::OffsetDateTime::now_utc())
            .format(
                &time::format_description::parse_borrowed::<1>(
                    "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"
                )
                .unwrap()
            )
            .unwrap()
    )?;
    writeln!(file, "PID: {}", pid)?;
    writeln!(file, "Version: {}", env!("CARGO_PKG_VERSION"))?;
    writeln!(file, "Log file: {:?}", path)?;
    writeln!(file)?;

    LOG_FILE.set(path).ok();
    Ok(())
}

pub fn log_startup_info(
    cli: &crate::cli::Opts,
    app_count: usize,
    frecency_count: usize,
    hidden_summary: &crate::core::hidden_entries::HiddenSummary,
) {
    if let Some(path) = LOG_FILE.get()
        && let Ok(mut file) = OpenOptions::new().append(true).open(path)
    {
        let _ = writeln!(file, "[STARTUP] Configuration:");
        let _ = writeln!(file, "  Prefix depth: {}", cli.prefix_depth);
        let _ = writeln!(file, "  Match mode: {:?}", cli.match_mode);
        let _ = writeln!(file, "  Ranking mode: {}", cli.ranking_mode.as_str());
        let _ = writeln!(file, "  Pinned order: {}", cli.pinned_order_mode.as_str());
        let _ = writeln!(file, "  Filter desktop: {}", cli.filter_desktop);
        let _ = writeln!(file, "  Filter actions: {}", cli.filter_actions);
        let _ = writeln!(file, "  Auto-hide duplicates: {}", cli.auto_hide_duplicates);
        let _ = writeln!(
            file,
            "  List executables in PATH: {}",
            cli.list_executables_in_path
        );
        let _ = writeln!(file, "  Hide before typing: {}", cli.hide_before_typing);
        let _ = writeln!(file);
        let _ = writeln!(file, "[STARTUP] Loaded data:");
        let _ = writeln!(file, "  Total apps: {}", app_count);
        let _ = writeln!(file, "  Frecency entries: {}", frecency_count);
        let _ = writeln!(file, "  Manually hidden: {}", hidden_summary.manual);
        let _ = writeln!(file, "  Automatically hidden: {}", hidden_summary.automatic);
        let _ = writeln!(file, "  Unavailable hides: {}", hidden_summary.unavailable);
        let _ = writeln!(file);
    }
}

pub fn log_event(event: &str) {
    if let Some(path) = LOG_FILE.get() {
        match OpenOptions::new().append(true).open(path) {
            Ok(mut file) => {
                let elapsed = SESSION_START
                    .get()
                    .map(|start| start.elapsed().as_millis())
                    .unwrap_or(0);
                let _ = writeln!(file, "[{:>6}ms] EVENT: {}", elapsed, event);
            }
            _ => {
                eprintln!("Warning: Failed to write to log file: {:?}", path);
            }
        }
    }
}

pub fn log_query_change(old_query: &str, new_query: &str, trigger: &str) {
    if let Some(path) = LOG_FILE.get()
        && let Ok(mut file) = OpenOptions::new().append(true).open(path)
    {
        let _ = writeln!(
            file,
            "[QUERY] {}: \"{}\" -> \"{}\"",
            trigger, old_query, new_query
        );
    }
}

pub fn log_selection_change(
    selected_idx: Option<usize>,
    app_name: Option<&str>,
    scroll_offset: usize,
) {
    if let Some(path) = LOG_FILE.get()
        && let Ok(mut file) = OpenOptions::new().append(true).open(path)
    {
        if let Some(idx) = selected_idx {
            let _ = writeln!(
                file,
                "[SELECTION] Index: {}, Scroll: {}, App: {}",
                idx,
                scroll_offset,
                app_name.unwrap_or("Unknown")
            );
        } else {
            let _ = writeln!(file, "[SELECTION] None (no selection)");
        }
    }
}

pub fn log_search_snapshot(query: &str, matches: &[App], prefix_depth: usize, filter_time_ms: u64) {
    if let Some(path) = LOG_FILE.get()
        && let Ok(mut file) = OpenOptions::new().append(true).open(path)
    {
        let _ = writeln!(file);
        let _ = writeln!(
            file,
            "[SEARCH] Query: \"{}\" (len: {}, prefix_depth: {})",
            query,
            query.len(),
            prefix_depth
        );
        let _ = writeln!(file, "[SEARCH] Filter time: {}ms", filter_time_ms);
        let _ = writeln!(
            file,
            "[SEARCH] Total matches: {} (showing top 50)",
            matches.len()
        );
        let _ = writeln!(file);

        for (idx, app) in matches.iter().take(50).enumerate() {
            let _ = writeln!(
                file,
                "  [{:>3}] {} (Score: {})",
                idx + 1,
                app.name,
                app.score
            );
            if let Some(ref b) = app.breakdown {
                let _ = writeln!(file, "       ├── Tier: {}", b.tier);
                let _ = writeln!(file, "       ├── Bucket Score: {}", b.bucket_score);
                let base_score = if b.matcher_score > 0 {
                    b.matcher_score / 100
                } else {
                    0
                };
                let _ = writeln!(
                    file,
                    "       ├── Matcher Score: {} (base: {}, 100x multiplier)",
                    b.matcher_score, base_score
                );
                let _ = writeln!(
                    file,
                    "       ├── {}: {:.3} (raw: {:.3}, boost: +{})",
                    b.ranking_mode,
                    b.raw_frecency_milli as f64 / 1000.0,
                    b.raw_frecency_milli as f64 / 1000.0,
                    b.frecency_boost
                );
                let _ = writeln!(
                    file,
                    "       └── Final Score: {}",
                    b.bucket_score + b.matcher_score + b.frecency_boost
                );
            } else {
                let _ = writeln!(file, "       └── (No breakdown available)");
            }

            // Show additional app metadata for top 10
            if idx < 10 {
                let exec_name = crate::strings::extract_exec_name(&app.command);
                let _ = writeln!(
                    file,
                    "       └── Exec: {}, Pinned: {}, History: {}",
                    exec_name, app.pinned, app.history
                );
            }
        }

        if matches.len() > 50 {
            let _ = writeln!(file, "       ... and {} more matches", matches.len() - 50);
        }
        let _ = writeln!(file);
    }
}

pub fn log_launch(app: &App, command: &str) {
    if let Some(path) = LOG_FILE.get()
        && let Ok(mut file) = OpenOptions::new().append(true).open(path)
    {
        let _ = writeln!(file, "[LAUNCH] App: \"{}\"", app.name);
        let _ = writeln!(file, "         Command: {}", command);
        let _ = writeln!(
            file,
            "         Pinned: {}, History: {}, Score: {}",
            app.pinned, app.history, app.score
        );
        if let Some(ref b) = app.breakdown {
            let _ = writeln!(file, "         Tier: {}", b.tier);
        }
    }
}

pub fn log_session_end() {
    if let Some(path) = LOG_FILE.get()
        && let Ok(mut file) = OpenOptions::new().append(true).open(path)
    {
        let elapsed = SESSION_START
            .get()
            .map(|start| start.elapsed())
            .unwrap_or(std::time::Duration::ZERO);
        let _ = writeln!(file);
        let _ = writeln!(file, "=== FSEL DEBUG SESSION ENDED ===");
        let _ = writeln!(file, "Duration: {:.3}s", elapsed.as_secs_f64());
        let _ = writeln!(
            file,
            "Timestamp: {}",
            time::OffsetDateTime::now_local()
                .unwrap_or_else(|_| time::OffsetDateTime::now_utc())
                .format(
                    &time::format_description::parse_borrowed::<1>(
                        "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"
                    )
                    .unwrap()
                )
                .unwrap()
        );
        let _ = writeln!(file);
    }
}
