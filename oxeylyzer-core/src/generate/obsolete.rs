use crate::{generate::LayoutGeneration, layout::*, utility::*};

impl LayoutGeneration {
    #[allow(dead_code)]
    pub(crate) fn score_with_precision(
        &self,
        layout: &FastLayout,
        trigram_precision: usize,
    ) -> f64 {
        let effort = (0..layout.matrix.len())
            .into_iter()
            .map(|i| self.char_effort(layout, i))
            .sum::<f64>();

        let fspeed_usage = (0..8)
            .into_iter()
            .map(|col| self.col_usage(layout, col) + self.col_fspeed(layout, col))
            .sum::<f64>();

        let scissors = self.scissor_score(layout);
        let lsbs = self.lsb_score(layout);

        let trigram_iter = self.data.trigrams.iter().take(trigram_precision);
        let trigram_score = self.trigram_score_iter(layout, trigram_iter);

        trigram_score - effort - fspeed_usage - scissors - lsbs
    }

    #[allow(dead_code)]
    fn col_fspeed_before(&self, layout: &FastLayout, col: usize) -> f64 {
        let (start, len) = unsafe { Self::col_to_start_len(col) };

        let mut res = 0.0;
        let dsfb_ratio = self.weights.dsfb_ratio;
        let dsfb_ratio2 = self.weights.dsfb_ratio2;
        let dsfb_ratio3 = self.weights.dsfb_ratio3;

        for i in start..(start + len) {
            let (PosPair(i1, i2), dist) = self.fspeed_vals[i];

            let c1 = unsafe { layout.cu(i1) } as usize;
            let c2 = unsafe { layout.cu(i2) } as usize;

            let (pair, rev) = (c1 * len + c2, c2 * len + c1);

            res += self.data.bigrams.get(pair).unwrap_or(&0.0) * dist;
            res += self.data.bigrams.get(rev).unwrap_or(&0.0) * dist;

            res += self.data.skipgrams.get(pair).unwrap_or(&0.0) * dist * dsfb_ratio;
            res += self.data.skipgrams.get(rev).unwrap_or(&0.0) * dist * dsfb_ratio;

            res += self.data.skipgrams2.get(pair).unwrap_or(&0.0) * dist * dsfb_ratio2;
            res += self.data.skipgrams2.get(rev).unwrap_or(&0.0) * dist * dsfb_ratio2;

            res += self.data.skipgrams3.get(pair).unwrap_or(&0.0) * dist * dsfb_ratio3;
            res += self.data.skipgrams3.get(rev).unwrap_or(&0.0) * dist * dsfb_ratio3;
        }

        res * self.weights.fspeed
    }

    #[allow(dead_code)]
    pub(crate) fn score_swap(&self, layout: &mut FastLayout, swap: &PosPair) -> f64 {
        unsafe { layout.swap_no_bounds(swap) };
        let score = self.score_with_precision(&layout, 1000);
        unsafe { layout.swap_no_bounds(swap) };
        score
    }

    #[allow(dead_code)]
    pub(crate) fn best_swap(
        &self,
        layout: &mut FastLayout,
        current_best_score: Option<f64>,
        possible_swaps: &[PosPair],
    ) -> (Option<PosPair>, f64) {
        let mut best_score = current_best_score.unwrap_or(f64::MIN / 2.0);
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
        let mut current_best_score = f64::MIN / 2.0;

        while let (Some(best_swap), new_score) =
            self.best_swap(&mut layout, Some(current_best_score), possible_swaps)
        {
            current_best_score = new_score;
            unsafe { layout.swap_no_bounds(&best_swap) };
        }

        layout
    }

    #[allow(dead_code)]
    pub(crate) fn effort_score(&self, layout: &FastLayout) -> f64 {
        (0..layout.matrix.len())
            .map(|i| self.char_effort(layout, i))
            .sum()
    }

    #[allow(dead_code)]
    pub(crate) fn usage_score(&self, layout: &FastLayout) -> f64 {
        (0..8).map(|i| self.col_usage(layout, i)).sum()
    }

    #[allow(dead_code)]
    pub(crate) fn fspeed_score(&self, layout: &FastLayout) -> f64 {
        (0..8).map(|i| self.col_fspeed(layout, i)).sum()
    }
}
