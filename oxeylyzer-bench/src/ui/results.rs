use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::{Block, Row, Table};

use crate::ui::App;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let rows = app.metrics.iter().map(|m| {
        Row::new(vec![
            m.name.to_string(),
            m.scores.len().to_string(),
            m.best().map_or("-".to_string(), |b| b.to_string()),
            m.top5_mean().map_or("-".to_string(), |t| format!("{t:.0}")),
            format!("{:.1}%", m.duplicate_pct()),
            m.layouts_per_sec()
                .map_or("-".to_string(), |r| format!("{r:.1}/s")),
        ])
    });

    let table = Table::new(
        rows,
        [
            Constraint::Min(10),
            Constraint::Length(8),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(8),
            Constraint::Length(10),
        ],
    )
    .header(Row::new([
        "Algorithm",
        "Layouts",
        "Best",
        "Top-5 mean",
        "Dupes",
        "Rate",
    ]))
    .block(Block::bordered().title("Results"));

    f.render_widget(table, area);
}
