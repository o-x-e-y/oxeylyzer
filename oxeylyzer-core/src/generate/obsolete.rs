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
        // let effort = (0..layout.matrix.len())
        //     .map(|i| self.char_effort(layout, i))
        //     .sum::<f64>();

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

        trigram_score - stretch_score - fspeed_usage - scissors - lsbs - pinky_ring
    }

    #[allow(dead_code)]
    fn col_fspeed_before(&self, layout: &FastLayout, finger: Finger) -> i64 {
        let (start, len) = Self::col_to_start_len(finger);

        let mut res = 0.0;
        let dsfb_ratio = self.weights.dsfb_ratio;
        let dsfb_ratio2 = self.weights.dsfb_ratio2;
        let dsfb_ratio3 = self.weights.dsfb_ratio3;

        for i in start..(start + len) {
            let (PosPair(i1, i2), dist) = self.fspeed_vals[i];

            let c1 = layout.char(i1).unwrap() as usize;
            let c2 = layout.char(i2).unwrap() as usize;

            let len = self.data.len();
            let (idx, rev) = (c1 * len + c2, c2 * len + c1);

            let dist = dist * 10.0;

            let bp = self.data.bigrams().get(idx).copied().unwrap_or_default();
            let br = self.data.bigrams().get(rev).copied().unwrap_or_default();

            let sp = self.data.skipgrams().get(idx).copied().unwrap_or_default();
            let sr = self.data.skipgrams().get(rev).copied().unwrap_or_default();

            let s2p = self.data.skipgrams2().get(idx).copied().unwrap_or_default();
            let s2r = self.data.skipgrams2().get(rev).copied().unwrap_or_default();

            let s3p = self.data.skipgrams3().get(idx).copied().unwrap_or_default();
            let s3r = self.data.skipgrams3().get(rev).copied().unwrap_or_default();

            res += (bp as f64) * dist;
            res += (br as f64) * dist;

            res += (sp as f64 * dsfb_ratio) * dist;
            res += (sr as f64 * dsfb_ratio) * dist;

            res += (s2p as f64 * dsfb_ratio2) * dist;
            res += (s2r as f64 * dsfb_ratio2) * dist;

            res += (s3p as f64 * dsfb_ratio3) * dist;
            res += (s3r as f64 * dsfb_ratio3) * dist;
        }

        (res * self.weights.fspeed) as i64
    }

    #[allow(dead_code)]
    pub(crate) fn score_swap(&self, layout: &mut FastLayout, swap: &PosPair) -> i64 {
        layout.swap_pair(swap);
        let score = self.score_with_precision(layout, self.trigram_precision);
        layout.swap_pair(swap);
        score
    }

    #[allow(dead_code)]
    pub(crate) fn best_swap(
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

    // #[allow(dead_code)]
    // pub(crate) fn effort_score(&self, layout: &FastLayout) -> f64 {
    //     (0..layout.matrix.len())
    //         .map(|i| self.char_effort(layout, i))
    //         .sum()
    // }

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
