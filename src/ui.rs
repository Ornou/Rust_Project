use crate::map::{CellType, Position};
use crate::robot::RobotType;
use crate::simulation::RenderSnapshot;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
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
    let stats_height = stats_panel_height();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(stats_height),
        ])
        .split(frame.size());

    draw_map(frame, snapshot, chunks[0]);
    draw_stats(frame, snapshot, chunks[1]);
}

/// Content lines plus block borders (title sits on the top border).
fn stats_panel_height() -> u16 {
    const STATS_LINES: u16 = 4;
    const BLOCK_BORDERS: u16 = 2;
    STATS_LINES + BLOCK_BORDERS
}

fn map_title(snap: &RenderSnapshot, visible_w: usize, visible_h: usize) -> String {
    if visible_w < snap.map.width || visible_h < snap.map.height {
        format!(
            " Map [{visible_w}x{visible_h}/{}x{}] ",
            snap.map.width, snap.map.height
        )
    } else {
        " Map ".to_string()
    }
}

fn draw_map(frame: &mut Frame, snap: &RenderSnapshot, area: Rect) {
    let inner = Block::default().borders(Borders::ALL).inner(area);
    let visible_w = inner.width as usize;
    let visible_h = inner.height as usize;
    let map_w = snap.map.width.min(visible_w);
    let map_h = snap.map.height.min(visible_h);

    let block = Block::default()
        .title(map_title(snap, map_w, map_h))
        .borders(Borders::ALL);

    let robot_map: HashMap<Position, RobotType> = snap
        .robot_positions
        .iter()
        .filter_map(|(_, pos, rt)| {
            (pos.x < map_w && pos.y < map_h).then_some((*pos, *rt))
        })
        .collect();

    let mut lines: Vec<Line> = Vec::with_capacity(map_h);

    for y in 0..map_h {
        let mut spans: Vec<Span> = Vec::with_capacity(map_w);
        for x in 0..map_w {
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

    let map_widget = Paragraph::new(lines).block(block);
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
