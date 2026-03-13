use libdof::prelude::Finger;

use crate::{
    cached_layout::*,
    generate::{LayoutGeneration, SMALLEST_SCORE},
    utility::*,
};

impl LayoutGeneration {
    #[allow(dead_code)]
    pub(crate) fn score_with_precision(
        &self,
        layout: &FastLayout,
        trigram_precision: usize,
    ) -> i64 {
        let fspeed_usage = Finger::FINGERS
            .into_iter()
            .map(|f| self.finger_usage(layout, f) + self.finger_fspeed(layout, f))
            .sum::<i64>();

        let lsbs = self.lsb_score(layout);
        let pinky_ring = self.pinky_ring_score(layout);

        let trigram_iter = self.data.gen_trigrams().iter().take(trigram_precision);
        let trigram_score = self.trigram_score_iter(layout, trigram_iter);
        let stretch_score = self.stretch_score(layout);

        trigram_score + stretch_score + fspeed_usage + lsbs + pinky_ring
    }

    #[allow(dead_code)]
    pub(crate) fn score_swap(&self, layout: &mut FastLayout, swap: &PosPair) -> i64 {
        layout.swap_pair(swap);
        let score = self.score_with_precision(layout, self.trigram_precision);
        layout.swap_pair(swap);
        score
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub(crate) fn usage_score(&self, layout: &FastLayout) -> i64 {
        Finger::FINGERS
            .into_iter()
            .map(|f| self.finger_usage(layout, f))
            .sum()
    }

    #[allow(dead_code)]
    pub(crate) fn fspeed_score(&self, layout: &FastLayout) -> i64 {
        Finger::FINGERS
            .into_iter()
            .map(|f| self.finger_fspeed(layout, f))
            .sum()
    }
}
