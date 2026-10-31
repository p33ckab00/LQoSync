//! `lqosync3 tui`: a terminal view of the service. The draw code reads only memory.

pub mod app;
pub mod data;
pub mod devices;
pub mod journal;
pub mod network;
pub mod osc52;
pub mod proc;

use app::{Action, App};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use std::io::stdout;
use std::path::Path;
use std::sync::mpsc::{self, TryRecvError};
use std::time::{Duration, Instant};

/// Give the terminal back: normal mode, normal screen, mouse to the terminal. Errors are ignored.
fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(
        stdout(),
        DisableMouseCapture,
        LeaveAlternateScreen,
        ratatui::crossterm::cursor::Show
    );
}

/// Restores the terminal on each exit path, also an error from `?`.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal();
    }
}

/// A panic on this thread (the main loop) calls `restore` first, so the message is readable.
/// A panic of a TUI worker (`tui-*` thread) does not touch the terminal and prints nothing:
/// the main loop joins the worker and shows its message.
pub fn install_panic_hook(restore: fn()) {
    let owner = std::thread::current().id();
    let old = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let me = std::thread::current();
        if me.id() == owner {
            restore();
        } else if me.name().is_some_and(|n| n.starts_with("tui-")) {
            return;
        }
        old(info);
    }));
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// The message of a worker that stopped.
fn panic_text(r: std::thread::Result<()>) -> String {
    match r {
        Ok(()) => "no error".into(),
        Err(p) => p
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| p.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "a panic".into()),
    }
}

/// `lqosync3 tui`.
pub fn run(config: &Path) -> anyhow::Result<()> {
    let config_text = std::fs::read_to_string(config).unwrap_or_default();
    let (cfg, config_error) = match crate::cli::read_config(config) {
        Ok((c, _)) => (c, None),
        Err(e) => (crate::config::defaults().clone(), Some(format!("{e:#}"))),
    };
    // SAFETY: geteuid has no preconditions and cannot fail.
    let read_only = unsafe { libc::geteuid() } != 0;
    let mut app = App::new(read_only, config_text, config_error);
    app.osc52 = cfg.tui.osc52;
    app.journal = journal::JournalView::new(cfg.tui.journal_lines);
    let (jtx, jrx) = mpsc::channel();
    let mut journal_child = None;

    // ratatui::init is not used: its panic hook restores the terminal on any thread.
    install_panic_hook(restore_terminal);
    enable_raw_mode()?;
    let _guard = TerminalGuard;
    execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    let mut term = Terminal::new(CrosstermBackend::new(stdout()))?;

    let (tx, rx) = mpsc::channel();
    let mut loader = Some(data::spawn_loader(&cfg, config.to_path_buf(), tx)?);

    let tick = Duration::from_secs(1);
    let mut last_tick = Instant::now();
    let mut dirty = true;
    loop {
        loop {
            match rx.try_recv() {
                Ok(d) => {
                    app.set_data(d, unix_now());
                    dirty = true;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if let Some(h) = loader.take() {
                        app.message = Some(format!("the loader stopped: {}", panic_text(h.join())));
                        dirty = true;
                    }
                    break;
                }
            }
        }
        // The journal starts at the first open of its tab, and then keeps its lines.
        if app.tab == app::Tab::Journal && journal_child.is_none() {
            journal_child = Some(journal::spawn_journal(
                "journalctl",
                &cfg.tui.service_unit,
                cfg.tui.journal_lines,
                jtx.clone(),
            ));
        }
        let batch: Vec<_> = jrx.try_iter().collect();
        if !batch.is_empty() {
            app.journal.push_batch(batch);
            dirty = true;
        }
        if dirty {
            term.draw(|f| app.draw(f))?;
            dirty = false;
        }
        if last_tick.elapsed() >= tick {
            // The clocks change each second.
            last_tick = Instant::now();
            dirty = true;
        }
        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        let action = match event::read()? {
            Event::Key(k) if k.kind == KeyEventKind::Press => app.on_key(k),
            Event::Mouse(m) => app.on_mouse(m),
            Event::Resize(..) => Action::None,
            _ => continue,
        };
        dirty = true;
        match action {
            Action::Quit => break,
            Action::SelectMode(on) => {
                if on {
                    execute!(stdout(), DisableMouseCapture)?;
                } else {
                    execute!(stdout(), EnableMouseCapture)?;
                }
            }
            Action::Copy(text) => {
                use std::io::Write;
                let mut out = stdout();
                out.write_all(osc52::osc52(&text).as_bytes())?;
                out.flush()?;
            }
            Action::None | Action::SyncNow | Action::Reload => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static RESTORED: AtomicUsize = AtomicUsize::new(0);
    fn count() {
        RESTORED.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn panic_hook_restores_only_on_its_own_thread() {
        install_panic_hook(count);
        assert!(std::panic::catch_unwind(|| panic!("main loop")).is_err());
        assert_eq!(RESTORED.load(Ordering::SeqCst), 1);
        let worker = std::thread::Builder::new()
            .name("tui-loader".into())
            .spawn(|| panic!("worker"))
            .unwrap();
        assert!(worker.join().is_err());
        assert_eq!(RESTORED.load(Ordering::SeqCst), 1);
        // Give the default hook back to the other tests.
        let _ = std::panic::take_hook();
    }
}
