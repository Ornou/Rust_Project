mod base;
mod communication;
mod map;
mod robot;
mod simulation;

use map::{CellType, Position};
use ratatui::backend::CrosstermBackend;
use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{Frame, Terminal};
use robot::RobotType;
use simulation::Simulation;
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
        terminal.draw(|f| ui(f, &sim))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                    break;
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            sim.tick();
            last_tick = Instant::now();
        }
    }

    restore_terminal()?;
    Ok(())
}

fn ui(f: &mut Frame, sim: &Simulation) {
    let main_chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .margin(1)
        .constraints([
            ratatui::layout::Constraint::Min(25),
            ratatui::layout::Constraint::Length(4),
        ])
        .split(f.size());

    let map_area = main_chunks[0];
    let stats_area = main_chunks[1];

    // Draw map
    draw_map(f, sim, map_area);

    // Draw stats
    draw_stats(f, sim, stats_area);
}

fn draw_map(f: &mut Frame, sim: &Simulation, area: ratatui::layout::Rect) {
    let mut map_content = String::new();
    let robot_positions: std::collections::HashMap<_, _> = sim
        .get_robot_positions()
        .iter()
        .map(|(id, pos, rtype)| (pos.clone(), (*id, *rtype)))
        .collect();

    for y in 0..sim.map.height {
        for x in 0..sim.map.width {
            let pos = Position::new(x, y);

            if pos == sim.map.base_position {
                map_content.push('#');
            } else if let Some((_, rtype)) = robot_positions.get(&pos) {
                let c = match rtype {
                    RobotType::Scout => 'x',
                    RobotType::Collector => 'o',
                };
                map_content.push(c);
            } else {
                let cell = sim.map.get_cell(pos);
                match cell {
                    CellType::Obstacle => map_content.push('█'),
                    CellType::Energy => map_content.push('E'),
                    CellType::Crystal => map_content.push('C'),
                    CellType::Empty => map_content.push('·'),
                }
            }
        }
        if y < sim.map.height - 1 {
            map_content.push('\n');
        }
    }

    let map_paragraph = Paragraph::new(map_content)
        .block(Block::default().title("Map").borders(Borders::ALL))
        .style(
            Style::default()
                .fg(Color::White)
                .bg(Color::Black),
        );

    f.render_widget(map_paragraph, area);
}

fn draw_stats(f: &mut Frame, sim: &Simulation, area: ratatui::layout::Rect) {
    let energy = sim.base.get_total_energy();
    let crystals = sim.base.get_total_crystals();

    let stats_text = vec![
        Line::from(vec![
            Span::raw("Turn: "),
            Span::styled(
                sim.turn.to_string(),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  |  Energy: "),
            Span::styled(
                energy.to_string(),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  |  Crystals: "),
            Span::styled(
                crystals.to_string(),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Robots: "),
            Span::styled(
                format!("{}S", sim.robots.iter().filter(|r| r.lock().robot_type == RobotType::Scout).count()),
                Style::default().fg(Color::Red),
            ),
            Span::raw(" / "),
            Span::styled(
                format!("{}C", sim.robots.iter().filter(|r| r.lock().robot_type == RobotType::Collector).count()),
                Style::default().fg(Color::Magenta),
            ),
        ]),
        Line::from(Span::raw("Press 'q' or ESC to quit")),
    ];

    let stats_paragraph = Paragraph::new(stats_text)
        .block(Block::default().title("Statistics").borders(Borders::ALL));

    f.render_widget(stats_paragraph, area);
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
