mod base;
mod communication;
mod map;
mod robot;
mod simulation;
mod ui;

use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use simulation::Simulation;
use std::io;
use std::time::{Duration, Instant};
use tracing::info;
use tracing_subscriber::FmtSubscriber;

fn main() -> io::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .with_writer(std::io::stderr)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default tracing subscriber failed");

    let seed = std::env::var("SIM_SEED")
        .ok()
        .and_then(|value| value.parse::<u64>().ok());
    info!(seed = seed.unwrap_or(0), "Starting simulation");

    setup_terminal()?;

    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.clear()?;

    let mut sim = Simulation::new(80, 30, 3, 5, seed);
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(100);

    loop {
        let snapshot = sim.get_snapshot();
        terminal.draw(|frame| ui::render(&snapshot, frame))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_default();

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
                    break;
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            sim.tick();
            last_tick = Instant::now();
        }
    }

    sim.stop();
    restore_terminal()?;
    Ok(())
}

fn setup_terminal() -> io::Result<()> {
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    Ok(())
}

fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
