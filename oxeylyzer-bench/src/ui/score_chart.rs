use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::widgets::{Block, Sparkline};

use crate::ui::App;

/// One sparkline per algorithm showing recent scores (shifted to be positive,
/// since scores are i64 and Sparkline takes u64).
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::vertical(vec![Constraint::Min(3); app.metrics.len()]).split(area);

    for (m, row) in app.metrics.iter().zip(rows.iter()) {
        let width = row.width.saturating_sub(2) as usize;
        let min = m.scores.iter().min().copied().unwrap_or(0);
        let data: Vec<u64> = m
            .scores
            .iter()
            .rev()
            .take(width)
            .rev()
            .map(|&s| (s - min) as u64 + 1)
            .collect();

        let spark = Sparkline::default()
            .block(Block::bordered().title(format!("{} scores", m.name)))
            .data(&data);
        f.render_widget(spark, *row);
    }
}
