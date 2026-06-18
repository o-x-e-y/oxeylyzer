use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::widgets::{Block, Gauge};

use crate::ui::App;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::vertical(vec![Constraint::Length(3); app.metrics.len()]).split(area);

    for (m, row) in app.metrics.iter().zip(rows.iter()) {
        let ratio = match (m.elapsed.is_some(), app.target) {
            (true, _) => 1.0,
            (false, Some(target)) if target > 0 => {
                (m.scores.len() as f64 / target as f64).clamp(0.0, 1.0)
            }
            // time mode: progress is unknown until the algorithm finishes
            _ => 0.0,
        };

        let gauge = Gauge::default()
            .block(Block::bordered().title(m.name))
            .ratio(ratio)
            .label(format!("{} layouts", m.scores.len()));
        f.render_widget(gauge, *row);
    }
}
