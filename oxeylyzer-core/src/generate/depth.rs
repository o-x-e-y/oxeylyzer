//! Depth-2 swap search: evaluating pairs of swaps instead of single swaps.
//!
//! [`DepthTwoHillClimber`] runs the full quadratic search every move (expected
//! to lose on throughput-adjusted quality — it exists to be measured).
//! [`VariableNeighborhoodDescent`] only pays the quadratic cost at depth-1
//! local optima, as an escape mechanism.

use crate::fast_layout::FastLayout;
use crate::generate::engine::{Engine, filtered_swaps};
use crate::generate::{LayoutCache, Oxeylyzer};
use crate::layout::PosPair;

/// Finds the best move of either one swap or two consecutive swaps.
///
/// Returns `(first, Some(second), score)` for a double swap or
/// `(first, None, score)` for a single swap. The layout is left unmodified.
pub fn best_double_swap(
    analyzer: &Oxeylyzer,
    layout: &mut FastLayout,
    cache: &LayoutCache,
    swaps: &[PosPair],
) -> Option<(PosPair, Option<PosPair>, i64)> {
    let mut best: Option<(PosPair, Option<PosPair>, i64)> = None;

    for &a in swaps {
        if let Some(single) = analyzer.score_swap_cached(layout, &a, cache)
            && best.is_none_or(|(_, _, s)| single > s)
        {
            best = Some((a, None, single));
        }

        // temporarily apply `a` on a scratch cache to search the second level
        let mut inner_cache = cache.clone();
        if analyzer.accept_swap(layout, &a, &mut inner_cache).is_none() {
            continue;
        }

        let (second, double) = analyzer.best_swap_cached(layout, &inner_cache, swaps, None);
        if let Some(b) = second
            && best.is_none_or(|(_, _, s)| double > s)
        {
            best = Some((a, Some(b), double));
        }

        // undo `a` (swapping the same pair again restores the layout; the
        // scratch cache is discarded)
        analyzer.accept_swap(layout, &a, &mut inner_cache);
    }

    best
}

/// Greedy search over the depth-2 neighborhood: every move evaluates all
/// single swaps and all ordered swap pairs, and applies the best.
pub struct DepthTwoHillClimber<'a> {
    /// The analyzer providing scoring and swap evaluation.
    pub analyzer: &'a Oxeylyzer,
    /// Maximum number of (double) moves before stopping.
    pub max_moves: usize,
}

impl Engine for DepthTwoHillClimber<'_> {
    fn generate_with_pins(&self, based_on: &FastLayout, pins: &[usize]) -> FastLayout {
        let analyzer = self.analyzer;
        let swaps = filtered_swaps(based_on, pins);
        let mut layout = based_on.random_with_pins(pins);

        if swaps.is_empty() {
            return layout;
        }

        let mut cache = analyzer.initialize_cache(&layout);

        for _ in 0..self.max_moves {
            let current = cache.total_score();

            match best_double_swap(analyzer, &mut layout, &cache, &swaps) {
                Some((a, b, score)) if score > current => {
                    analyzer.accept_swap(&mut layout, &a, &mut cache);
                    if let Some(b) = b {
                        analyzer.accept_swap(&mut layout, &b, &mut cache);
                    }
                }
                _ => break,
            }
        }

        layout
    }
}

/// Variable neighborhood descent: climb with single swaps until stalled, then
/// search the depth-2 neighborhood once to escape; repeat until even depth-2
/// finds nothing better.
pub struct VariableNeighborhoodDescent<'a> {
    /// The analyzer providing scoring and swap evaluation.
    pub analyzer: &'a Oxeylyzer,
    /// Maximum number of depth-2 escapes before giving up.
    pub max_escapes: usize,
}

impl Engine for VariableNeighborhoodDescent<'_> {
    fn generate_with_pins(&self, based_on: &FastLayout, pins: &[usize]) -> FastLayout {
        let analyzer = self.analyzer;
        let swaps = filtered_swaps(based_on, pins);
        let mut layout = based_on.random_with_pins(pins);

        if swaps.is_empty() {
            return layout;
        }

        let mut cache = analyzer.initialize_cache(&layout);
        let mut escapes = 0;

        loop {
            analyzer.climb(&mut layout, &mut cache, &swaps, 200);
            let current = cache.total_score();

            // after a full climb no single swap improves, so only a true
            // double swap can escape
            if escapes < self.max_escapes {
                if let Some((a, Some(b), score)) =
                    best_double_swap(analyzer, &mut layout, &cache, &swaps)
                {
                    if score > current {
                        analyzer.accept_swap(&mut layout, &a, &mut cache);
                        analyzer.accept_swap(&mut layout, &b, &mut cache);
                        escapes += 1;
                        continue;
                    }
                }
            }

            break;
        }

        layout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::test_util;

    #[test]
    fn best_double_swap_improves_and_score_is_exact() {
        let analyzer = test_util::analyzer();
        let mut layout = test_util::qwerty().clone();
        let mut cache = analyzer.initialize_cache(&layout);
        let swaps = layout.possible_swaps.clone();

        let before = layout.clone();
        let current = cache.total_score();

        let (a, b, score) = best_double_swap(analyzer, &mut layout, &cache, &swaps).unwrap();

        // the search itself must not leave the layout modified
        assert_eq!(before, layout);
        assert!(
            score > current,
            "qwerty must have an improving (double) swap"
        );

        // applying the reported swaps must reproduce the reported score exactly
        analyzer.accept_swap(&mut layout, &a, &mut cache);
        if let Some(b) = b {
            analyzer.accept_swap(&mut layout, &b, &mut cache);
        }
        assert_eq!(cache.total_score(), score);
    }

    #[test]
    fn depth_two_generates_valid_layout() {
        let analyzer = test_util::analyzer();
        let qwerty = test_util::qwerty();
        let engine = DepthTwoHillClimber {
            analyzer,
            max_moves: 5,
        };

        let generated = engine.generate(qwerty);

        test_util::assert_valid(qwerty, &generated);
    }

    #[test]
    fn vnd_ends_at_depth_two_local_optimum() {
        let analyzer = test_util::analyzer();
        let qwerty = test_util::qwerty();
        let engine = VariableNeighborhoodDescent {
            analyzer,
            max_escapes: 3,
        };

        let mut generated = engine.generate(qwerty);

        test_util::assert_valid(qwerty, &generated);
        assert!(test_util::score(&generated) > test_util::score(qwerty));

        // no single swap may improve the result (it ends with a climb)
        let cache = analyzer.initialize_cache(&generated);
        let swaps = generated.possible_swaps.clone();
        let (swap, _) =
            analyzer.best_swap_cached(&mut generated, &cache, &swaps, Some(cache.total_score()));
        assert!(swap.is_none());
    }
}
