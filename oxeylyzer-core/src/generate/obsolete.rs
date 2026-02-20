use libdof::prelude::Finger;

use crate::{
    generate::{LayoutGeneration, SMALLEST_SCORE},
    layout::*,
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

        let scissors = self.scissor_score(layout);
        let lsbs = self.lsb_score(layout);
        let pinky_ring = self.pinky_ring_score(layout);

        let trigram_iter = self.data.gen_trigrams().iter().take(trigram_precision);
        let trigram_score = self.trigram_score_iter(layout, trigram_iter);
        let stretch_score = self.stretch_score(layout);

        trigram_score + stretch_score + fspeed_usage + scissors + lsbs + pinky_ring
    }

    #[allow(dead_code)]
    fn col_fspeed_before(&self, layout: &FastLayout, finger: Finger) -> i64 {
        let dsfb_ratio = self.weights.dsfb_ratio;
        let dsfb_ratio2 = self.weights.dsfb_ratio2;
        let dsfb_ratio3 = self.weights.dsfb_ratio3;

        let fspeed = if let Some(indices) = layout.fspeed_indices.fingers.get(finger as usize) {
            indices
                .iter()
                .map(
                    |BigramPair {
                         pair: PosPair(p1, p2),
                         dist,
                     }| {
                        if let Some(c1) = layout.char(*p1)
                            && let Some(c2) = layout.char(*p2)
                        {
                            let len = self.data.len();
                            let (c1, c2) = (c1 as usize, c2 as usize);
                            let (idx, rev) = (c1 * len + c2, c2 * len + c1);

                            let bp = self.data.bigrams().get(idx).copied().unwrap_or_default();
                            let br = self.data.bigrams().get(rev).copied().unwrap_or_default();

                            let sp = self.data.skipgrams().get(idx).copied().unwrap_or_default();
                            let sr = self.data.skipgrams().get(rev).copied().unwrap_or_default();

                            let s2p = self.data.skipgrams2().get(idx).copied().unwrap_or_default();
                            let s2r = self.data.skipgrams2().get(rev).copied().unwrap_or_default();

                            let s3p = self.data.skipgrams3().get(idx).copied().unwrap_or_default();
                            let s3r = self.data.skipgrams3().get(rev).copied().unwrap_or_default();

                            let mut res = 0;

                            res += bp * dist;
                            res += br * dist;

                            res += sp * dsfb_ratio * dist;
                            res += sr * dsfb_ratio * dist;

                            res += s2p * dsfb_ratio2 * dist;
                            res += s2r * dsfb_ratio2 * dist;

                            res += s3p * dsfb_ratio3 * dist;
                            res += s3r * dsfb_ratio3 * dist;

                            res
                        } else {
                            0
                        }
                    },
                )
                .sum()
        } else {
            0
        };

        fspeed * self.weights.fspeed
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
    pub(crate) fn optimize_normal_no_cols(
        &self,
        mut layout: FastLayout,
        possible_swaps: &[PosPair],
    ) -> FastLayout {
        let mut current_best_score = SMALLEST_SCORE;

        while let (Some(best_swap), new_score) =
            self.best_swap(&mut layout, Some(current_best_score), possible_swaps)
        {
            current_best_score = new_score;
            layout.swap_pair(&best_swap);
        }

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
