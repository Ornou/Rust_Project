mod base;
mod communication;
mod map;
mod robot;
mod simulation;

use crossterm::event::{self, Event};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use map::{CellType, Position};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{Frame, Terminal};
use robot::RobotType;
use simulation::{RenderSnapshot, Simulation};
use std::collections::HashMap;
use std::io;
use std::time::{Duration, Instant};

fn main() -> io::Result<()> {
    setup_terminal()?;

    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.clear()?;

    let mut sim = Simulation::new(80, 30, 3, 5);
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(100);

    loop {
        let snapshot = sim.get_snapshot();
        terminal.draw(|f| ui(f, &snapshot))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_default();

        if crossterm::event::poll(timeout)? {
            // Any key press exits
            if let Event::Key(_) = event::read()? {
                break;
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

fn ui(f: &mut Frame, snap: &RenderSnapshot) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(20), Constraint::Length(5)])
        .split(f.size());

    draw_map(f, snap, chunks[0]);
    draw_stats(f, snap, chunks[1]);
}

fn draw_map(f: &mut Frame, snap: &RenderSnapshot, area: Rect) {
    // Build a Position → RobotType lookup for O(1) access
    let robot_map: HashMap<Position, RobotType> = snap
        .robot_positions
        .iter()
        .map(|(_, pos, rt)| (*pos, *rt))
        .collect();

    let mut lines: Vec<Line> = Vec::new();

    for y in 0..snap.map.height {
        let mut spans: Vec<Span> = Vec::new();
        for x in 0..snap.map.width {
            let pos = Position::new(x, y);
            let (ch, style) = if pos == snap.map.base_position {
                ('#', Style::default().fg(Color::LightGreen))
            } else if let Some(rt) = robot_map.get(&pos) {
                match rt {
                    RobotType::Scout => ('x', Style::default().fg(Color::Red)),
                    RobotType::Collector => ('o', Style::default().fg(Color::Magenta)),
                }
            } else {
                match snap.map.get_cell(pos) {
                    CellType::Obstacle => ('O', Style::default().fg(Color::LightCyan)),
                    CellType::Energy => ('E', Style::default().fg(Color::Green)),
                    CellType::Crystal => ('C', Style::default().fg(Color::LightMagenta)),
                    CellType::Empty => ('·', Style::default().fg(Color::DarkGray)),
                }
            };
            spans.push(Span::styled(ch.to_string(), style));
        }
        lines.push(Line::from(spans));
    }

    let map_widget = Paragraph::new(lines)
        .block(Block::default().title(" Map ").borders(Borders::ALL));
    f.render_widget(map_widget, area);
}

fn draw_stats(f: &mut Frame, snap: &RenderSnapshot, area: Rect) {
    let num_scouts = snap
        .robot_positions
        .iter()
        .filter(|(_, _, rt)| *rt == RobotType::Scout)
        .count();
    let num_collectors = snap
        .robot_positions
        .iter()
        .filter(|(_, _, rt)| *rt == RobotType::Collector)
        .count();

    let text = vec![
        Line::from(vec![
            Span::raw("Turn: "),
            Span::styled(
                snap.turn.to_string(),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  |  Energy: "),
            Span::styled(
                snap.energy.to_string(),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  |  Crystals: "),
            Span::styled(
                snap.crystals.to_string(),
                Style::default()
                    .fg(Color::LightMagenta)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{}x Scouts", num_scouts),
                Style::default().fg(Color::Red),
            ),
            Span::raw("  |  "),
            Span::styled(
                format!("{}o Collectors", num_collectors),
                Style::default().fg(Color::Magenta),
            ),
        ]),
        Line::from(vec![
            Span::styled("# ", Style::default().fg(Color::LightGreen)),
            Span::raw("Base  "),
            Span::styled("x ", Style::default().fg(Color::Red)),
            Span::raw("Scout  "),
            Span::styled("o ", Style::default().fg(Color::Magenta)),
            Span::raw("Collector  "),
            Span::styled("O ", Style::default().fg(Color::LightCyan)),
            Span::raw("Obstacle  "),
            Span::styled("E ", Style::default().fg(Color::Green)),
            Span::raw("Energy  "),
            Span::styled("C ", Style::default().fg(Color::LightMagenta)),
            Span::raw("Crystal"),
        ]),
        Line::from(Span::raw("Press any key to quit")),
    ];

    let stats = Paragraph::new(text)
        .block(Block::default().title(" Statistics ").borders(Borders::ALL));
    f.render_widget(stats, area);
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
