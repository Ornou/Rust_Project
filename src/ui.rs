use crate::map::{CellType, Position};
use crate::robot::RobotType;
use crate::simulation::RenderSnapshot;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use std::collections::HashMap;

mod styles {
    use ratatui::style::{Color, Modifier, Style};

    pub fn base() -> Style {
        Style::default().fg(Color::LightGreen)
    }

    pub fn scout() -> Style {
        Style::default().fg(Color::Red)
    }

    pub fn collector() -> Style {
        Style::default().fg(Color::Magenta)
    }

    pub fn obstacle() -> Style {
        Style::default().fg(Color::LightCyan)
    }

    pub fn energy() -> Style {
        Style::default().fg(Color::Green)
    }

    pub fn crystal() -> Style {
        Style::default().fg(Color::LightMagenta)
    }

    pub fn empty() -> Style {
        Style::default().fg(Color::DarkGray)
    }

    pub fn turn() -> Style {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    }

    pub fn energy_stat() -> Style {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    }

    pub fn crystal_stat() -> Style {
        Style::default()
            .fg(Color::LightMagenta)
            .add_modifier(Modifier::BOLD)
    }
}

pub fn render(snapshot: &RenderSnapshot, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(0), Constraint::Length(5)])
        .split(frame.size());

    draw_map(frame, snapshot, chunks[0]);
    draw_stats(frame, snapshot, chunks[1]);
}

fn draw_map(frame: &mut Frame, snap: &RenderSnapshot, area: Rect) {
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
                ('#', styles::base())
            } else if let Some(rt) = robot_map.get(&pos) {
                match rt {
                    RobotType::Scout => ('x', styles::scout()),
                    RobotType::Collector => ('o', styles::collector()),
                }
            } else {
                match snap.map.get_cell(pos) {
                    CellType::Obstacle => ('O', styles::obstacle()),
                    CellType::Energy => ('E', styles::energy()),
                    CellType::Crystal => ('C', styles::crystal()),
                    CellType::Empty => ('·', styles::empty()),
                }
            };
            spans.push(Span::styled(ch.to_string(), style));
        }
        lines.push(Line::from(spans));
    }

    let map_widget = Paragraph::new(lines)
        .block(Block::default().title(" Map ").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    frame.render_widget(map_widget, area);
}

fn draw_stats(frame: &mut Frame, snap: &RenderSnapshot, area: Rect) {
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
            Span::styled(snap.turn.to_string(), styles::turn()),
            Span::raw("  |  Energy: "),
            Span::styled(snap.energy.to_string(), styles::energy_stat()),
            Span::raw("  |  Crystals: "),
            Span::styled(snap.crystals.to_string(), styles::crystal_stat()),
        ]),
        Line::from(vec![
            Span::styled(format!("{}x Scouts", num_scouts), styles::scout()),
            Span::raw("  |  "),
            Span::styled(
                format!("{}o Collectors", num_collectors),
                styles::collector(),
            ),
        ]),
        legend_line(),
        Line::from(Span::raw("Press q or Esc to quit")),
    ];

    let stats =
        Paragraph::new(text).block(Block::default().title(" Statistics ").borders(Borders::ALL));
    frame.render_widget(stats, area);
}

fn legend_line() -> Line<'static> {
    Line::from(vec![
        Span::styled("# ", styles::base()),
        Span::raw("Base  "),
        Span::styled("x ", styles::scout()),
        Span::raw("Scout  "),
        Span::styled("o ", styles::collector()),
        Span::raw("Collector  "),
        Span::styled("O ", styles::obstacle()),
        Span::raw("Obstacle  "),
        Span::styled("E ", styles::energy()),
        Span::raw("Energy  "),
        Span::styled("C ", styles::crystal()),
        Span::raw("Crystal"),
    ])
}
