use crate::{
    fast_layout::*,
    generate::{Oxeylyzer, SMALLEST_SCORE},
    layout::PosPair,
};

#[cfg(test)]
use libdof::prelude::Finger;

impl Oxeylyzer {
    /// Finds the best possible swap without using the cache. This uses the `.possible_swaps` field
    /// on the [`FastLayout`] that is put in. It returns the swap and the new score if it did find a
    /// swap that was better.
    pub fn best_swap(
        &self,
        layout: &mut FastLayout,
        current_best_score: Option<i64>,
        possible_swaps: &[PosPair],
    ) -> (Option<PosPair>, i64) {
        let mut best_score = current_best_score.unwrap_or(SMALLEST_SCORE);
        let mut best_swap = None;

        for swap in possible_swaps.iter() {
            let current = self.score_swap(layout, swap);

            if current > best_score {
                best_score = current;
                best_swap = Some(*swap);
            }
        }

        (best_swap, best_score)
    }

    pub(crate) fn score_swap(&self, layout: &mut FastLayout, swap: &PosPair) -> i64 {
        layout.swap_pair(swap);
        let score = self.score_with_precision(layout, self.trigram_precision);
        layout.swap_pair(swap);
        score
    }
}

#[cfg(test)]
impl Oxeylyzer {
    pub(crate) fn optimize_normal_no_cols(&self, mut layout: FastLayout) -> FastLayout {
        let mut current_best_score = SMALLEST_SCORE;
        let possible_swaps = std::mem::take(&mut layout.possible_swaps);

        while let (Some(best_swap), new_score) =
            self.best_swap(&mut layout, Some(current_best_score), &possible_swaps)
        {
            current_best_score = new_score;
            layout.swap_pair(&best_swap);
        }

        layout.possible_swaps = possible_swaps;

        layout
    }

    pub(crate) fn usage_score(&self, layout: &FastLayout) -> i64 {
        Finger::FINGERS
            .into_iter()
            .map(|f| self.finger_usage(layout, f))
            .sum()
    }

    pub(crate) fn fspeed_score(&self, layout: &FastLayout) -> i64 {
        Finger::FINGERS
            .into_iter()
            .map(|f| self.finger_fspeed(layout, f))
            .sum()
    }
}
