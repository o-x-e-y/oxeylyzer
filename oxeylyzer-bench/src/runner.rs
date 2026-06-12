use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use oxeylyzer_core::fast_layout::FastLayout;
use oxeylyzer_core::generate::{
    Oxeylyzer,
    annealing::SimulatedAnnealing,
    depth::{DepthTwoHillClimber, VariableNeighborhoodDescent},
    engine::Engine,
    hill_climber::CachedHillClimber,
    ils::IteratedLocalSearch,
    lahc::LateAcceptanceHillClimbing,
};
use oxeylyzer_core::rayon::iter::ParallelIterator;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AlgorithmKind {
    HillClimb,
    Annealing,
    Ils,
    Lahc,
    DepthTwo,
    Vnd,
}

impl AlgorithmKind {
    pub const ALL: [AlgorithmKind; 6] = [
        AlgorithmKind::HillClimb,
        AlgorithmKind::Annealing,
        AlgorithmKind::Ils,
        AlgorithmKind::Lahc,
        AlgorithmKind::DepthTwo,
        AlgorithmKind::Vnd,
    ];

    pub fn name(self) -> &'static str {
        match self {
            AlgorithmKind::HillClimb => "hill",
            AlgorithmKind::Annealing => "sa",
            AlgorithmKind::Ils => "ils",
            AlgorithmKind::Lahc => "lahc",
            AlgorithmKind::DepthTwo => "d2",
            AlgorithmKind::Vnd => "vnd",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "hill" | "hc" | "hill-climb" => Some(Self::HillClimb),
            "sa" | "annealing" => Some(Self::Annealing),
            "ils" => Some(Self::Ils),
            "lahc" => Some(Self::Lahc),
            "d2" | "depth2" => Some(Self::DepthTwo),
            "vnd" => Some(Self::Vnd),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
pub enum Budget {
    /// Generate exactly this many layouts per algorithm.
    Count(usize),
    /// Generate as many layouts as possible within this wall-clock budget.
    Time(Duration),
}

pub struct Event {
    pub algo: usize,
    pub score: i64,
    pub layout: String,
}

pub enum Message {
    Generated(Event),
    AlgoFinished { algo: usize, elapsed: Duration },
    AllDone,
}

/// Runs each algorithm sequentially (each fully parallel inside), streaming
/// results over `tx`. Checks `cancel` between batches.
pub fn run_all(
    kinds: &[AlgorithmKind],
    analyzer: &Oxeylyzer,
    basis: &FastLayout,
    pins: &[usize],
    budget: Budget,
    tx: Sender<Message>,
    cancel: &AtomicBool,
) {
    for (idx, &kind) in kinds.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let start = Instant::now();
        run_one(kind, idx, analyzer, basis, pins, budget, &tx, cancel);
        tx.send(Message::AlgoFinished {
            algo: idx,
            elapsed: start.elapsed(),
        })
        .ok();
    }
    tx.send(Message::AllDone).ok();
}

#[allow(clippy::too_many_arguments)]
fn run_one(
    kind: AlgorithmKind,
    idx: usize,
    analyzer: &Oxeylyzer,
    basis: &FastLayout,
    pins: &[usize],
    budget: Budget,
    tx: &Sender<Message>,
    cancel: &AtomicBool,
) {
    match kind {
        AlgorithmKind::HillClimb => drive(
            &CachedHillClimber { analyzer },
            idx,
            analyzer,
            basis,
            pins,
            budget,
            tx,
            cancel,
        ),
        AlgorithmKind::Annealing => drive(
            &SimulatedAnnealing {
                analyzer,
                cooling: 0.9995,
                iters: 50_000,
            },
            idx,
            analyzer,
            basis,
            pins,
            budget,
            tx,
            cancel,
        ),
        AlgorithmKind::Ils => drive(
            &IteratedLocalSearch {
                analyzer,
                perturb_swaps: 4,
                rounds: 10,
            },
            idx,
            analyzer,
            basis,
            pins,
            budget,
            tx,
            cancel,
        ),
        AlgorithmKind::Lahc => drive(
            &LateAcceptanceHillClimbing {
                analyzer,
                history: 1_000,
                iters: 100_000,
            },
            idx,
            analyzer,
            basis,
            pins,
            budget,
            tx,
            cancel,
        ),
        AlgorithmKind::DepthTwo => drive(
            &DepthTwoHillClimber {
                analyzer,
                max_moves: 100,
            },
            idx,
            analyzer,
            basis,
            pins,
            budget,
            tx,
            cancel,
        ),
        AlgorithmKind::Vnd => drive(
            &VariableNeighborhoodDescent {
                analyzer,
                max_escapes: 10,
            },
            idx,
            analyzer,
            basis,
            pins,
            budget,
            tx,
            cancel,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn drive<E: Engine>(
    engine: &E,
    idx: usize,
    analyzer: &Oxeylyzer,
    basis: &FastLayout,
    pins: &[usize],
    budget: Budget,
    tx: &Sender<Message>,
    cancel: &AtomicBool,
) {
    let batch = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let deadline = match budget {
        Budget::Time(d) => Some(Instant::now() + d),
        Budget::Count(_) => None,
    };
    let target = match budget {
        Budget::Count(n) => Some(n),
        Budget::Time(_) => None,
    };
    let mut produced = 0usize;

    loop {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        if let Some(t) = target
            && produced >= t
        {
            return;
        }
        if let Some(d) = deadline
            && Instant::now() >= d
        {
            return;
        }

        let n = match target {
            Some(t) => batch.min(t - produced),
            None => batch,
        };

        engine
            .generate_n_with_pins_iter(n, basis, pins)
            .for_each_with(tx.clone(), |tx, layout| {
                let score = analyzer.initialize_cache(&layout).total_score();
                tx.send(Message::Generated(Event {
                    algo: idx,
                    score,
                    layout: layout.layout_str(),
                }))
                .ok();
            });

        produced += n;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_all_aliases() {
        for (input, expected) in [
            ("hill", AlgorithmKind::HillClimb),
            ("hc", AlgorithmKind::HillClimb),
            ("SA", AlgorithmKind::Annealing),
            ("annealing", AlgorithmKind::Annealing),
            ("ils", AlgorithmKind::Ils),
            ("lahc", AlgorithmKind::Lahc),
            ("d2", AlgorithmKind::DepthTwo),
            ("depth2", AlgorithmKind::DepthTwo),
            ("vnd", AlgorithmKind::Vnd),
        ] {
            assert_eq!(AlgorithmKind::parse(input), Some(expected), "{input}");
        }
        assert_eq!(AlgorithmKind::parse("nonsense"), None);
    }

    #[test]
    fn every_kind_in_all_has_a_roundtripping_name() {
        for kind in AlgorithmKind::ALL {
            assert_eq!(AlgorithmKind::parse(kind.name()), Some(kind));
        }
    }
}
