//! Parameter-tuning harness: sweeps LAHC / SA / ILS configurations under a
//! fixed wall-clock budget each and reports which configuration most reliably
//! finds the optimum.
//!
//! Usage: `cargo run --release -p oxeylyzer-bench --bin tune -- [secs] [stage]`
//! where `secs` is the budget per configuration (default 10) and `stage` is
//! `coarse` or `fine` (default coarse).

use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::anyhow;
use oxeylyzer_core::fast_layout::FastLayout;
use oxeylyzer_core::generate::Oxeylyzer;
use oxeylyzer_core::generate::annealing::SimulatedAnnealing;
use oxeylyzer_core::generate::engine::Engine;
use oxeylyzer_core::generate::ils::IteratedLocalSearch;
use oxeylyzer_core::generate::lahc::LateAcceptanceHillClimbing;
use oxeylyzer_core::rayon::iter::ParallelIterator;
use oxeylyzer_core::{data::Data, weights::Config};
use oxeylyzer_resources::OxeylyzerDirs;

struct RunResult {
    label: String,
    secs: f64,
    scores: Vec<i64>,
    unique: usize,
}

fn run_config<E: Engine>(
    engine: &E,
    analyzer: &Oxeylyzer,
    basis: &FastLayout,
    secs: f64,
    label: String,
) -> RunResult {
    let start = Instant::now();
    let deadline = start + Duration::from_secs_f64(secs);
    let batch = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    let mut scores = Vec::new();
    let mut counts: HashMap<String, usize> = HashMap::new();

    while Instant::now() < deadline {
        let results: Vec<(i64, String)> = engine
            .generate_n_with_pins_iter(batch, basis, &[])
            .map(|l| (analyzer.initialize_cache(&l).total_score(), l.layout_str()))
            .collect();

        for (score, layout) in results {
            scores.push(score);
            *counts.entry(layout).or_default() += 1;
        }
    }

    eprintln!("  {label}: {} layouts", scores.len());

    RunResult {
        label,
        secs: start.elapsed().as_secs_f64(),
        scores,
        unique: counts.len(),
    }
}

fn top5_mean(scores: &[i64]) -> f64 {
    let mut sorted = scores.to_vec();
    sorted.sort_unstable_by(|a, b| b.cmp(a));
    let top = &sorted[..sorted.len().min(5)];
    top.iter().sum::<i64>() as f64 / top.len().max(1) as f64
}

fn main() -> anyhow::Result<()> {
    let secs: f64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(10.0);
    let stage = std::env::args().nth(2).unwrap_or_else(|| "coarse".into());

    let language = "english";
    let dirs = OxeylyzerDirs::resolve().map_err(|e| anyhow!("{e}"))?;
    let config = Config::with_loaded_weights(dirs.config_file()).map_err(|e| anyhow!("{e}"))?;
    let data = Data::load(dirs.language_data_dir().join(format!("{language}.json")))
        .map_err(|e| anyhow!("{e}"))?;
    let analyzer = Oxeylyzer::new(data, config);
    let saved = oxeylyzer_repl::repl::load_layouts(dirs.layouts_dir().join(language))
        .map_err(|e| anyhow!("{e}"))?;
    let basis_layout = saved
        .get("sturdy")
        .or_else(|| saved.values().next())
        .ok_or_else(|| anyhow!("no layouts found"))?;
    let basis = analyzer.fast_layout(basis_layout, &[]);

    let mut results: Vec<RunResult> = Vec::new();

    match stage.as_str() {
        "coarse" => {
            for &history in &[100usize, 1_000, 5_000] {
                for &iters in &[50_000usize, 100_000, 200_000] {
                    let engine = LateAcceptanceHillClimbing {
                        analyzer: &analyzer,
                        history,
                        iters,
                    };
                    let label = format!("lahc h={history} i={iters}");
                    results.push(run_config(&engine, &analyzer, &basis, secs, label));
                }
            }
            for &cooling in &[0.999f64, 0.9995, 0.9999] {
                for &iters in &[50_000usize, 100_000, 200_000] {
                    let engine = SimulatedAnnealing {
                        analyzer: &analyzer,
                        cooling,
                        iters,
                    };
                    let label = format!("sa c={cooling} i={iters}");
                    results.push(run_config(&engine, &analyzer, &basis, secs, label));
                }
            }
            for &perturb_swaps in &[3usize, 5, 8] {
                for &rounds in &[10usize, 25, 50] {
                    let engine = IteratedLocalSearch {
                        analyzer: &analyzer,
                        perturb_swaps,
                        rounds,
                    };
                    let label = format!("ils p={perturb_swaps} r={rounds}");
                    results.push(run_config(&engine, &analyzer, &basis, secs, label));
                }
            }
        }
        "fine" => {
            // refined around the coarse winners: lahc h≈1000 with cheap iters,
            // sa with slow cooling, ils at perturb 4-5 with moderate rounds
            for (history, iters) in [
                (500usize, 50_000usize),
                (1_000, 30_000),
                (1_000, 50_000),
                (1_000, 100_000),
                (2_000, 50_000),
            ] {
                let engine = LateAcceptanceHillClimbing {
                    analyzer: &analyzer,
                    history,
                    iters,
                };
                let label = format!("lahc h={history} i={iters}");
                results.push(run_config(&engine, &analyzer, &basis, secs, label));
            }
            for (cooling, iters) in [(0.9997f64, 50_000usize), (0.9999, 50_000), (0.9999, 30_000)] {
                let engine = SimulatedAnnealing {
                    analyzer: &analyzer,
                    cooling,
                    iters,
                };
                let label = format!("sa c={cooling} i={iters}");
                results.push(run_config(&engine, &analyzer, &basis, secs, label));
            }
            for (perturb_swaps, rounds) in [(4usize, 15usize), (5, 15), (5, 25)] {
                let engine = IteratedLocalSearch {
                    analyzer: &analyzer,
                    perturb_swaps,
                    rounds,
                };
                let label = format!("ils p={perturb_swaps} r={rounds}");
                results.push(run_config(&engine, &analyzer, &basis, secs, label));
            }
        }
        "confirm" => {
            // head-to-head of the fine-stage leaders plus two ils neighbors
            for (perturb_swaps, rounds) in [(5usize, 25usize), (5, 35), (6, 25)] {
                let engine = IteratedLocalSearch {
                    analyzer: &analyzer,
                    perturb_swaps,
                    rounds,
                };
                let label = format!("ils p={perturb_swaps} r={rounds}");
                results.push(run_config(&engine, &analyzer, &basis, secs, label));
            }
            {
                let engine = SimulatedAnnealing {
                    analyzer: &analyzer,
                    cooling: 0.9997,
                    iters: 50_000,
                };
                results.push(run_config(
                    &engine,
                    &analyzer,
                    &basis,
                    secs,
                    "sa c=0.9997 i=50000".into(),
                ));
            }
            {
                let engine = LateAcceptanceHillClimbing {
                    analyzer: &analyzer,
                    history: 1_000,
                    iters: 100_000,
                };
                results.push(run_config(
                    &engine,
                    &analyzer,
                    &basis,
                    secs,
                    "lahc h=1000 i=100000".into(),
                ));
            }
        }
        other => return Err(anyhow!("unknown stage '{other}'")),
    }

    let session_best = results
        .iter()
        .filter_map(|r| r.scores.iter().max())
        .max()
        .copied()
        .ok_or_else(|| anyhow!("no results"))?;

    // "near" = within 0.02% of the session best (the optimum cluster)
    let near_threshold = session_best - (session_best.abs() as f64 * 0.0002) as i64;

    struct Row {
        label: String,
        layouts: usize,
        per_sec: f64,
        best_gap: f64,
        top5_gap: f64,
        unique_pct: f64,
        hits: usize,
        hits_per_min: f64,
        near_per_min: f64,
    }

    let mut rows: Vec<Row> = results
        .iter()
        .map(|r| {
            let n = r.scores.len();
            let best = r.scores.iter().max().copied().unwrap_or(i64::MIN);
            let hits = r.scores.iter().filter(|&&s| s == session_best).count();
            let near = r.scores.iter().filter(|&&s| s >= near_threshold).count();
            Row {
                label: r.label.clone(),
                layouts: n,
                per_sec: n as f64 / r.secs,
                best_gap: (session_best - best) as f64 / session_best.abs() as f64 * 100.0,
                top5_gap: (session_best as f64 - top5_mean(&r.scores)) / session_best.abs() as f64
                    * 100.0,
                unique_pct: r.unique as f64 / n.max(1) as f64 * 100.0,
                hits,
                hits_per_min: hits as f64 / r.secs * 60.0,
                near_per_min: near as f64 / r.secs * 60.0,
            }
        })
        .collect();

    // sort by optimum hits/min desc, then near hits/min desc
    rows.sort_by(|a, b| {
        b.hits_per_min
            .partial_cmp(&a.hits_per_min)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(
                b.near_per_min
                    .partial_cmp(&a.near_per_min)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
    });

    println!("\nsession best: {session_best}  (near = within 0.02%)\n");
    println!(
        "| config | layouts | l/s | bestΔ% | top5Δ% | uniq% | opt hits | hits/min | near/min |"
    );
    println!("|---|---|---|---|---|---|---|---|---|");
    for row in &rows {
        println!(
            "| {} | {} | {:.1} | {:.4} | {:.4} | {:.0} | {} | {:.1} | {:.1} |",
            row.label,
            row.layouts,
            row.per_sec,
            row.best_gap,
            row.top5_gap,
            row.unique_pct,
            row.hits,
            row.hits_per_min,
            row.near_per_min,
        );
    }

    Ok(())
}
