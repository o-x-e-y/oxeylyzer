//! Late acceptance hill climbing: accept a swap if it beats the score from
//! `history` iterations ago. One parameter, no cooling schedule.

use nanorand::{Rng, tls_rng};

use crate::fast_layout::FastLayout;
use crate::generate::Oxeylyzer;
use crate::generate::engine::{Engine, filtered_swaps};

/// Late acceptance hill climbing over the swap neighborhood.
pub struct LateAcceptanceHillClimbing<'a> {
    /// The analyzer providing scoring and swap evaluation.
    pub analyzer: &'a Oxeylyzer,
    /// History length L: a candidate is accepted if it beats the score from L iterations ago.
    pub history: usize,
    /// Total number of LAHC iterations before the final greedy polish.
    pub iters: usize,
}

impl Engine for LateAcceptanceHillClimbing<'_> {
    fn generate_with_pins(&self, based_on: &FastLayout, pins: &[usize]) -> FastLayout {
        let analyzer = self.analyzer;
        let swaps = filtered_swaps(based_on, pins);
        let mut layout = based_on.random_with_pins(pins);

        if swaps.is_empty() || self.history == 0 {
            return layout;
        }

        let mut cache = analyzer.initialize_cache(&layout);
        let mut rng = tls_rng();

        let mut current = cache.total_score();
        let mut history = vec![current; self.history];

        for i in 0..self.iters {
            let swap = swaps[rng.generate_range(0..swaps.len())];
            let Some(candidate) = analyzer.score_swap_cached(&mut layout, &swap, &cache) else {
                continue;
            };

            let v = i % self.history;
            if candidate >= history[v] || candidate >= current {
                analyzer.accept_swap(&mut layout, &swap, &mut cache);
                current = candidate;
            }
            if current > history[v] {
                history[v] = current;
            }
        }

        analyzer.climb(&mut layout, &mut cache, &swaps, 200);

        layout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::test_util;

    #[test]
    fn lahc_generates_valid_improved_layout() {
        let analyzer = test_util::analyzer();
        let qwerty = test_util::qwerty();
        let engine = LateAcceptanceHillClimbing {
            analyzer,
            history: 100,
            iters: 10_000,
        };

        let generated = engine.generate(qwerty);

        test_util::assert_valid(qwerty, &generated);
        assert!(test_util::score(&generated) > test_util::score(qwerty));
    }
}
