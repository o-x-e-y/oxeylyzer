//! Iterated local search: climb to a local optimum, perturb with k random
//! swaps, re-climb, and keep the result only if it improved.

use nanorand::{Rng, tls_rng};

use crate::fast_layout::FastLayout;
use crate::generate::Oxeylyzer;
use crate::generate::engine::{Engine, filtered_swaps};

/// Iterated local search over the swap neighborhood.
pub struct IteratedLocalSearch<'a> {
    /// The analyzer providing scoring and swap evaluation.
    pub analyzer: &'a Oxeylyzer,
    /// Number of random swaps applied as perturbation between climbs (k ≈ 3–6).
    pub perturb_swaps: usize,
    /// Number of perturb+climb rounds after the initial climb.
    pub rounds: usize,
}

impl Engine for IteratedLocalSearch<'_> {
    fn generate_with_pins(&self, based_on: &FastLayout, pins: &[usize]) -> FastLayout {
        let analyzer = self.analyzer;
        let swaps = filtered_swaps(based_on, pins);
        let mut best = based_on.random_with_pins(pins);

        if swaps.is_empty() {
            return best;
        }

        let mut rng = tls_rng();
        let mut best_cache = analyzer.initialize_cache(&best);
        analyzer.climb(&mut best, &mut best_cache, &swaps, 200);
        let mut best_score = best_cache.total_score();

        for _ in 0..self.rounds {
            let mut candidate = best.clone();
            let mut cache = best_cache.clone();

            // perturb: k random accepted swaps (bounded retries skip no-op swaps)
            let mut applied = 0;
            for _ in 0..self.perturb_swaps * 10 {
                if applied == self.perturb_swaps {
                    break;
                }
                let swap = swaps[rng.generate_range(0..swaps.len())];
                if analyzer
                    .accept_swap(&mut candidate, &swap, &mut cache)
                    .is_some()
                {
                    applied += 1;
                }
            }

            analyzer.climb(&mut candidate, &mut cache, &swaps, 200);

            if cache.total_score() > best_score {
                best_score = cache.total_score();
                best = candidate;
                best_cache = cache;
            }
        }

        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::test_util;

    #[test]
    fn ils_generates_valid_improved_layout() {
        let analyzer = test_util::analyzer();
        let qwerty = test_util::qwerty();
        let engine = IteratedLocalSearch {
            analyzer,
            perturb_swaps: 4,
            rounds: 3,
        };

        let generated = engine.generate(qwerty);

        test_util::assert_valid(qwerty, &generated);
        assert!(test_util::score(&generated) > test_util::score(qwerty));
    }

    #[test]
    fn ils_zero_rounds_is_a_plain_climb() {
        let analyzer = test_util::analyzer();
        let qwerty = test_util::qwerty();
        let engine = IteratedLocalSearch {
            analyzer,
            perturb_swaps: 4,
            rounds: 0,
        };

        let generated = engine.generate(qwerty);

        test_util::assert_valid(qwerty, &generated);
        assert!(test_util::score(&generated) > test_util::score(qwerty));
    }
}
