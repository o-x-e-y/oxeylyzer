//! The baseline search algorithm: random restart + greedy cached hill climbing.

use crate::fast_layout::FastLayout;
use crate::generate::Oxeylyzer;
use crate::generate::engine::Engine;

/// The baseline algorithm: a thin named wrapper around [`Oxeylyzer`]'s built-in
/// random-restart greedy hill climb, so it can be benched under a stable name.
pub struct CachedHillClimber<'a> {
    /// The analyzer providing scoring and swap evaluation.
    pub analyzer: &'a Oxeylyzer,
}

impl Engine for CachedHillClimber<'_> {
    fn generate_with_pins(&self, based_on: &FastLayout, pins: &[usize]) -> FastLayout {
        Oxeylyzer::generate_with_pins(self.analyzer, based_on, pins)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::test_util;

    #[test]
    fn hill_climber_generates_valid_improved_layout() {
        let analyzer = test_util::analyzer();
        let qwerty = test_util::qwerty();
        let engine = CachedHillClimber { analyzer };

        let generated = engine.generate(qwerty);

        test_util::assert_valid(qwerty, &generated);
        assert!(test_util::score(&generated) > test_util::score(qwerty));
    }

    #[test]
    fn hill_climber_respects_pins() {
        let analyzer = test_util::analyzer();
        let qwerty = test_util::qwerty();
        let engine = CachedHillClimber { analyzer };
        let pins = [0usize, 14, 29];

        let generated = engine.generate_with_pins(qwerty, &pins);

        test_util::assert_valid(qwerty, &generated);
        for &p in &pins {
            assert_eq!(generated.keys[p], qwerty.keys[p], "pinned key moved at {p}");
        }
    }
}
