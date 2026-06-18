use std::sync::mpsc::Receiver;

use crate::metrics::AlgoMetrics;
use crate::runner::Message;

/// Consumes runner messages without a TUI and prints a markdown results table.
pub fn run(rx: Receiver<Message>, names: Vec<&'static str>) {
    let mut metrics: Vec<AlgoMetrics> = names.into_iter().map(AlgoMetrics::new).collect();

    while let Ok(msg) = rx.recv() {
        match msg {
            Message::Generated(e) => metrics[e.algo].record(e.score, e.layout),
            Message::AlgoFinished { algo, elapsed } => {
                metrics[algo].elapsed = Some(elapsed);
                eprintln!("{} finished in {elapsed:.1?}", metrics[algo].name);
            }
            Message::AllDone => break,
        }
    }

    println!("| Algorithm | Layouts | Best score | Top-5 mean | Dupes % | Layouts/sec |");
    println!("|---|---|---|---|---|---|");
    for m in &metrics {
        println!(
            "| {} | {} | {} | {} | {:.1} | {} |",
            m.name,
            m.scores.len(),
            m.best().map_or("-".to_string(), |b| b.to_string()),
            m.top5_mean().map_or("-".to_string(), |t| format!("{t:.0}")),
            m.duplicate_pct(),
            m.layouts_per_sec()
                .map_or("-".to_string(), |r| format!("{r:.1}")),
        );
    }
}
