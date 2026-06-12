pub mod progress;
pub mod results;
pub mod score_chart;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::time::Duration;

use crossterm::event::{self, Event as CtEvent, KeyCode};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::widgets::{Block, Paragraph};

use crate::metrics::AlgoMetrics;
use crate::runner::Message;

pub struct App {
    pub title: String,
    pub metrics: Vec<AlgoMetrics>,
    /// layouts per algorithm in count mode; None in time mode
    pub target: Option<usize>,
    pub done: bool,
}

impl App {
    pub fn new(title: String, names: Vec<&'static str>, target: Option<usize>) -> Self {
        Self {
            title,
            metrics: names.into_iter().map(AlgoMetrics::new).collect(),
            target,
            done: false,
        }
    }

    pub fn apply(&mut self, msg: Message) {
        match msg {
            Message::Generated(e) => self.metrics[e.algo].record(e.score, e.layout),
            Message::AlgoFinished { algo, elapsed } => self.metrics[algo].elapsed = Some(elapsed),
            Message::AllDone => self.done = true,
        }
    }
}

/// Runs the TUI event loop at ~10 Hz until the user presses `q`/`Esc`.
/// Sets `cancel` on exit so the runner stops after its current batch.
pub fn run(rx: Receiver<Message>, mut app: App, cancel: &AtomicBool) -> std::io::Result<()> {
    let mut terminal = ratatui::init();

    loop {
        while let Ok(msg) = rx.try_recv() {
            app.apply(msg);
        }

        terminal.draw(|f| draw(f, &app))?;

        if event::poll(Duration::from_millis(100))?
            && let CtEvent::Key(key) = event::read()?
            && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
        {
            cancel.store(true, Ordering::Relaxed);
            break;
        }
    }

    ratatui::restore();
    Ok(())
}

fn draw(f: &mut Frame, app: &App) {
    let [header, mid, bottom] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(8),
        Constraint::Length(app.metrics.len() as u16 + 4),
    ])
    .areas(f.area());

    let status = if app.done {
        " — done, q to quit"
    } else {
        " — q to quit"
    };
    f.render_widget(
        Paragraph::new(app.title.as_str())
            .block(Block::bordered().title(format!("oxeylyzer bench{status}"))),
        header,
    );

    let [left, right] =
        Layout::horizontal([Constraint::Length(34), Constraint::Min(20)]).areas(mid);
    progress::render(f, left, app);
    score_chart::render(f, right, app);
    results::render(f, bottom, app);
}
