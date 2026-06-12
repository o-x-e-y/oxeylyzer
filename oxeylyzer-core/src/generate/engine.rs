//! The [`Engine`] trait: a common interface for layout search algorithms.

use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::fast_layout::FastLayout;
use crate::generate::Oxeylyzer;
use crate::layout::PosPair;

/// A layout search algorithm. Implementors only need [`Engine::generate_with_pins`];
/// the other methods have default implementations.
pub trait Engine: Sync {
    /// Generates one optimized layout starting from a random shuffle of `based_on`,
    /// keeping pinned positions fixed.
    fn generate_with_pins(&self, based_on: &FastLayout, pins: &[usize]) -> FastLayout;

    /// Generates one optimized layout starting from a random shuffle of `basis`.
    fn generate(&self, basis: &FastLayout) -> FastLayout {
        self.generate_with_pins(basis, &[])
    }

    /// Generates `amount` optimized layouts in parallel.
    fn generate_n_iter<'a>(
        &'a self,
        amount: usize,
        based_on: &'a FastLayout,
    ) -> impl ParallelIterator<Item = FastLayout> + 'a {
        self.generate_n_with_pins_iter(amount, based_on, &[])
    }

    /// Generates `amount` optimized layouts in parallel with pinned keys.
    fn generate_n_with_pins_iter<'a>(
        &'a self,
        amount: usize,
        based_on: &'a FastLayout,
        pins: &'a [usize],
    ) -> impl ParallelIterator<Item = FastLayout> + 'a {
        (0..amount)
            .into_par_iter()
            .map(move |_| self.generate_with_pins(based_on, pins))
    }
}

impl Engine for Oxeylyzer {
    fn generate_with_pins(&self, based_on: &FastLayout, pins: &[usize]) -> FastLayout {
        Oxeylyzer::generate_with_pins(self, based_on, pins)
    }
}

/// Returns `based_on.possible_swaps` minus any swap touching a pinned position.
pub fn filtered_swaps(based_on: &FastLayout, pins: &[usize]) -> Vec<PosPair> {
    based_on
        .possible_swaps
        .iter()
        .copied()
        .filter(|&PosPair(a, b)| {
            !pins.contains(&(a as usize)) && !pins.contains(&(b as usize))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::test_util;

    fn assert_engine<E: Engine>(_: &E) {}

    #[test]
    fn oxeylyzer_implements_engine() {
        assert_engine(test_util::analyzer());
    }

    #[test]
    fn engine_generate_produces_valid_improved_layout() {
        let analyzer = test_util::analyzer();
        let qwerty = test_util::qwerty();

        let generated = Engine::generate(analyzer, qwerty);

        test_util::assert_valid(qwerty, &generated);
        assert!(test_util::score(&generated) > test_util::score(qwerty));
    }

    #[test]
    fn filtered_swaps_excludes_pins() {
        let qwerty = test_util::qwerty();
        let pins = [0usize, 5, 10];

        let swaps = filtered_swaps(qwerty, &pins);

        assert!(!swaps.is_empty());
        for crate::layout::PosPair(a, b) in &swaps {
            assert!(!pins.contains(&(*a as usize)));
            assert!(!pins.contains(&(*b as usize)));
        }
    }
}
