//! Application launcher mode

use crate::cli::Opts;
use crate::core::ranking::{current_unix_seconds, sort_by_ranking};
use crate::core::state::State;
use crate::ui::{InputConfig, InputEvent as Event, UI};
use eyre::{Result, WrapErr};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use scopeguard::defer;
use std::cell::Cell;
use std::io;
use std::sync::Arc;
use std::time::Duration;

/// Run application launcher mode.
pub async fn run(cli: Opts) -> Result<()> {
    use crossterm::event::KeyCode;

    let data_dir = crate::app::paths::runtime_data_dir()?;
    let application_dirs = crate::desktop::application_dirs();
    let history_db_path = crate::app::paths::history_db_path()?;
    let lock_path = crate::app::paths::launcher_lock_path()?;
    let session =
        super::session::LauncherSession::start(&history_db_path, &lock_path, cli.replace)?;
    let db = std::sync::Arc::clone(session.db());

    if let Some(ref program_name) = cli.program
        && program_name.len() >= 2
    {
        return super::direct::launch_program_directly(&cli, program_name, &session);
    }

    if super::admin::handle_maintenance_command(&cli, &db, data_dir.as_path())? {
        return Ok(());
    }

    let hidden_store = crate::core::hidden_entries::HiddenEntryStore::new(Arc::clone(&db))?;

    super::admin::initialize_test_mode(&cli);

    let apps_rx = crate::desktop::read_with_options(
        application_dirs.clone(),
        &db,
        crate::desktop::DiscoverOptions {
            filter_desktop: cli.filter_desktop,
            filter_actions: cli.filter_actions,
            list_executables: cli.list_executables_in_path,
            auto_hide_duplicates: cli.auto_hide_duplicates,
        },
    );

    let mut all_apps = Vec::with_capacity(500);
    while let Ok(app) = apps_rx.recv() {
        all_apps.push(app);
    }

    let frecency_data = crate::core::database::load_frecency(&db);
    let frecency_count = frecency_data.len();
    let mut pin_timestamps = crate::core::database::load_pin_timestamps(&db);
    sort_by_ranking(
        &mut all_apps,
        &frecency_data,
        cli.ranking_mode,
        cli.pinned_order_mode,
        &pin_timestamps,
        current_unix_seconds(),
    );

    let discovered_count = all_apps.len();

    let mut state = State::new(
        all_apps,
        cli.match_mode,
        frecency_data,
        cli.prefix_depth,
        cli.ranking_mode,
        cli.pinned_order_mode,
        std::mem::take(&mut pin_timestamps),
    );
    state.set_visibility_options(crate::core::hidden_entries::VisibilityOptions {
        auto_hide_duplicates: cli.auto_hide_duplicates,
        application_dirs,
    });
    state.set_hidden_entry_keys(hidden_store.entry_keys()?);
    super::admin::log_startup_if_enabled(
        &cli,
        discovered_count,
        frecency_count,
        state.hidden_summary(),
    );

    if let Some(ref search) = cli.search_string {
        state.query = search.clone();
    }

    state.filter();
    state.update_info(
        cli.highlight_color,
        cli.fancy_mode,
        cli.verbose.unwrap_or(0),
    );

    if cli.verbose.unwrap_or(0) > 2 {
        let hidden_summary = state.hidden_summary();
        eprintln!(
            "Hidden entries: {} manual, {} automatic, {} unavailable",
            hidden_summary.manual, hidden_summary.automatic, hidden_summary.unavailable,
        );
    }

    if cli.stdout {
        println!(
            "{}",
            serde_json::to_string(&state.shown)
                .expect("Desktop entries should be serializable to json")
        );
        return Ok(());
    }

    crate::ui::terminal::setup_terminal(cli.disable_mouse)?;
    let terminal_active = Cell::new(true);
    defer! {
        if terminal_active.get() {
            let _ = crate::ui::terminal::shutdown_terminal(cli.disable_mouse);
        }
    }

    let backend = CrosstermBackend::new(io::stderr());
    let mut terminal = Terminal::new(backend).wrap_err("Failed to start crossterm terminal")?;
    terminal.hide_cursor().wrap_err("Failed to hide cursor")?;
    crate::ui::terminal::clear_fullscreen(&mut terminal).wrap_err("Failed to clear terminal")?;

    // The graphics capability probe must run before the input reader, but it can
    // wait for an unanswered terminal response. Show a usable launcher first.
    let mut initial_render_result = Ok((false, false));
    terminal.draw(|frame| {
        initial_render_result = UI::new().render(frame, &state, &cli, None);
    })?;
    initial_render_result?;

    let mut icons = super::icons::IconRuntime::new(&cli, Arc::clone(&db));
    icons.request_if_changed(&state, terminal.size()?.into(), &cli);

    let mut input = InputConfig {
        disable_mouse: cli.disable_mouse,
        tick_rate: Duration::from_millis(250),
        render_rate: None,
        exit_key: KeyCode::Null,
        ..InputConfig::default()
    }
    .init_async();
    let mut needs_redraw = true;

    loop {
        if needs_redraw {
            let mut render_result = Ok((false, false));
            terminal.draw(|frame| {
                render_result = UI::new().render(frame, &state, &cli, icons.render_state());
            })?;
            let (preview_failed, list_failed) = render_result?;
            if preview_failed || list_failed {
                icons.handle_render_failure(preview_failed);
                needs_redraw = true;
                continue;
            }
        }

        tokio::select! {
            Some(result) = icons.next_result() => {
                icons.apply_result(result);
                needs_redraw = true;
            }
            maybe_event = input.next() => {
                let Some(event) = maybe_event else {
                    break;
                };
                let should_handle = matches!(&event, Event::Input(_) | Event::Mouse(_));
                needs_redraw =
                    matches!(&event, Event::Input(_) | Event::Mouse(_) | Event::Render);
                let terminal_area = terminal.size()?;
                let total_height = terminal_area.height;
                if should_handle {
                    super::events::handle_event(
                        &mut state,
                        event,
                        &cli,
                        &db,
                        &hidden_store,
                        total_height,
                    );
                }
                icons.request_if_changed(&state, terminal_area.into(), &cli);
            }
        }

        if state.should_exit {
            if crate::cli::DEBUG_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                crate::core::debug_logger::log_session_end();
            }
            break;
        }

        if state.should_launch {
            if let Some(selected_idx) = state.selected
                && let Some(app) = state.shown.get(selected_idx)
            {
                if let Err(error) = crate::core::database::record_access(&db, &app.name) {
                    eprintln!("Failed to record access: {}", error);
                }

                crate::ui::terminal::shutdown_terminal(cli.disable_mouse)?;
                terminal_active.set(false);

                if cli.no_exec {
                    println!("{}", app.command);
                    return Ok(());
                }

                super::launch::launch_app(app, &cli, &db)?;
            }
            break;
        }
    }

    if !state.should_launch {
        crate::ui::terminal::shutdown_terminal(cli.disable_mouse)?;
        terminal_active.set(false);
    }

    if crate::cli::DEBUG_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        crate::core::debug_logger::log_session_end();
    }

    Ok(())
}
