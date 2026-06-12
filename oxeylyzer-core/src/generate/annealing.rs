//! Simulated annealing: random swaps, accepting worse moves with probability
//! `e^(Δ/T)` under a geometric cooling schedule, followed by a greedy polish.

use nanorand::{Rng, tls_rng};

use crate::fast_layout::FastLayout;
use crate::generate::Oxeylyzer;
use crate::generate::engine::{Engine, filtered_swaps};

/// Simulated annealing over the swap neighborhood.
pub struct SimulatedAnnealing<'a> {
    /// The analyzer providing scoring and swap evaluation.
    pub analyzer: &'a Oxeylyzer,
    /// Geometric cooling factor per iteration, e.g. `0.9995`.
    pub cooling: f64,
    /// Total number of annealing iterations before the final greedy polish.
    pub iters: usize,
}

impl Engine for SimulatedAnnealing<'_> {
    fn generate_with_pins(&self, based_on: &FastLayout, pins: &[usize]) -> FastLayout {
        let analyzer = self.analyzer;
        let swaps = filtered_swaps(based_on, pins);
        let mut layout = based_on.random_with_pins(pins);

        if swaps.is_empty() {
            return layout;
        }

        let mut cache = analyzer.initialize_cache(&layout);
        let mut rng = tls_rng();

        // Initial temperature: mean |Δ| over a sample of random swaps, so the
        // schedule self-scales to the corpus/weight magnitudes.
        let mut t = {
            let current = cache.total_score();
            let (sum, count) = (0..128)
                .filter_map(|_| {
                    let swap = swaps[rng.generate_range(0..swaps.len())];
                    analyzer.score_swap_cached(&mut layout, &swap, &cache)
                })
                .fold((0.0f64, 0u32), |(s, c), cand| {
                    (s + (cand - current).abs() as f64, c + 1)
                });

            if count == 0 { 1.0 } else { sum / count as f64 }
        };
        let t_min = t * 1e-4;

        for _ in 0..self.iters {
            let swap = swaps[rng.generate_range(0..swaps.len())];

            if let Some(candidate) = analyzer.score_swap_cached(&mut layout, &swap, &cache) {
                let delta = candidate - cache.total_score();
                let accept = delta >= 0 || {
                    let unit = rng.generate::<u64>() as f64 / u64::MAX as f64;
                    unit < (delta as f64 / t).exp()
                };

                if accept {
                    analyzer.accept_swap(&mut layout, &swap, &mut cache);
                }
            }

            t = (t * self.cooling).max(t_min);
        }

        // polish: annealing rarely ends exactly on a local optimum
        analyzer.climb(&mut layout, &mut cache, &swaps, 200);

        layout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::test_util;

    #[test]
    fn annealing_generates_valid_improved_layout() {
        let analyzer = test_util::analyzer();
        let qwerty = test_util::qwerty();
        let engine = SimulatedAnnealing {
            analyzer,
            cooling: 0.999,
            iters: 5_000,
        };

        let generated = engine.generate(qwerty);

        test_util::assert_valid(qwerty, &generated);
        assert!(test_util::score(&generated) > test_util::score(qwerty));
    }

    #[test]
    fn annealing_respects_pins() {
        let analyzer = test_util::analyzer();
        let qwerty = test_util::qwerty();
        let engine = SimulatedAnnealing {
            analyzer,
            cooling: 0.999,
            iters: 2_000,
        };
        let pins = [3usize, 17, 25];

        let generated = engine.generate_with_pins(qwerty, &pins);

        test_util::assert_valid(qwerty, &generated);
        for &p in &pins {
            assert_eq!(generated.keys[p], qwerty.keys[p], "pinned key moved at {p}");
        }
    }
}
