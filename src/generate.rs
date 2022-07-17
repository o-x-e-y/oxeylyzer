use smallmap::Map;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use anyhow::Result;

use crate::utility::*;
use crate::trigram_patterns::TrigramPattern;
use crate::analyze::LayoutAnalysis;
use crate::language_data::TrigramData;
use crate::layout::*;
use crate::weights::{Weights, Config};

pub type CharToFinger<T> = Map<T, usize>;
pub type Matrix<T> = [T; 30];

#[derive(Default)]
struct LayoutCache {
	fspeed_total: f64,
	trigrams_total: f64,
	fspeed: [f64; 8],
	trigrams: [f64; 30]
}

type PerCharTrigrams = fxhash::FxHashMap<char, TrigramData>;

pub struct LayoutGeneration {
	pub available_chars: [char; 30],
	pub per_char_trigrams: PerCharTrigrams,
	pub weights: Weights,
	pub analysis: LayoutAnalysis,
	pub improved_layout: FastLayout,
	pub temp_generated: Option<Vec<String>>,
	cols: [usize; 6]
}

impl LayoutGeneration {
	pub fn new(
		language: &str, trigram_precision: usize, weights_opt: Option<Weights>
	) -> Result<Self> {
		let weights = if weights_opt.is_none() {
			Config::new().weights
		} else {
			weights_opt.unwrap()
		};
		
		if let Ok(mut analyzer) = LayoutAnalysis::new(
			language, Some(weights.clone())
		) {
			let mut available = available_chars(language);
			Ok(
				Self {
					per_char_trigrams: Self::per_char_trigrams(
						&analyzer.language_data.trigrams, &available, trigram_precision
					),
					weights: weights,
					available_chars: available,
					analysis: analyzer,
					improved_layout: FastLayout::new(),
					temp_generated: None,
					cols: [0, 1, 2, 7, 8, 9],
				}
			)
		} else {
			anyhow::bail!("Could not initalize analyzer.")
		}
	}

	fn per_char_trigrams(
		trigrams: &TrigramData, available_chars: &[char; 30], trigram_precision: usize
	) -> PerCharTrigrams {
		let mut n_trigrams = trigrams.clone();
		n_trigrams.truncate(trigram_precision);
		
		let thingy: Vec<(char, Vec<([char; 3], f64)>)> = available_chars
			.iter()
			.map(|c| {
				let per_char = n_trigrams
					.iter()
					.map(|(t, f)| (t.clone(), f.clone()))
					.filter(|(t, _)| t.contains(c))
					.collect::<Vec<([char; 3], f64)>>();
				(*c, per_char)
			})
			.collect();
		
		PerCharTrigrams::from_iter(thingy)
	}

	pub fn trigrams_char_score(&self, layout: &FastLayout, pos: usize) -> f64 {
		let mut freqs = crate::analyze::TrigramStats::default();
		let c = layout.matrix[pos];
		if let Some(trigrams) = self.per_char_trigrams.get(&c)
		&& trigrams.len() > 0 {
			for (trigram, freq) in trigrams {
				match layout.get_trigram_pattern(trigram) {
					TrigramPattern::Alternate => freqs.alternates += freq,
					TrigramPattern::AlternateSfs => freqs.alternates_sfs += freq,
					TrigramPattern::Inroll => freqs.inrolls += freq,
					TrigramPattern::Outroll => freqs.outrolls += freq,
					TrigramPattern::Onehand => freqs.onehands += freq,
					TrigramPattern::Redirect => freqs.redirects += freq,
					TrigramPattern::BadRedirect => freqs.bad_redirects += freq,
					_ => {}
				}
			}
			let mut score = 0.0;
			score += self.weights.inrolls * freqs.inrolls;
			score += self.weights.outrolls * freqs.outrolls;
			score += self.weights.onehands * freqs.onehands;
			score += self.weights.alternates * freqs.alternates;
			score += self.weights.alternates_sfs * freqs.alternates_sfs;
			score -= self.weights.redirects * freqs.redirects;
			score -= self.weights.bad_redirects * freqs.bad_redirects;
			score
		} else {
			0.0
		}
	}

	pub fn score_whole_matrix(&self, layout: &FastLayout) -> [f64; 30] {
		let mut res = [0.0; 30];
		for i in 0..30 {
			res[i] = self.trigrams_char_score(layout, i);
		}
		res
	}

	fn score_swap(&self, layout: &mut FastLayout, swap: &PosPair, cache: &LayoutCache) {
		let (c1, c2) = layout.swap_pair_no_bounds(swap);

	}

	pub fn optimize_cols(&self, layout: &mut FastLayout, trigram_precision: usize, score: Option<f64>) -> f64 {
		let mut best_score = score.unwrap_or(self.analysis.score(layout, trigram_precision));

		let mut best = layout.clone();
		self.col_perms(layout, &mut best, &mut best_score, 6);
		layout.swap_indexes();

		self.col_perms(layout, &mut best, &mut best_score, 6);
		*layout = best;
		best_score
	}

	fn col_perms(&self, layout: &mut FastLayout, best: &mut FastLayout, best_score: &mut f64, k: usize) {
		if k == 1 {
			let new_score = self.analysis.score(layout, 1000);
			if new_score > *best_score {
				*best_score = new_score;
				*best = layout.clone();
			}
			return;
		}
		for i in 0..k {
			LayoutGeneration::col_perms(self, layout, best, best_score, k - 1);
			if k % 2 == 0 {
				layout.swap_cols_no_bounds(self.cols[i], self.cols[k - 1]);
			} else {
				layout.swap_cols_no_bounds(self.cols[0], self.cols[k - 1]);
			}
		}
	}

	pub fn generate(&self) -> FastLayout {
		let layout = FastLayout::random(self.available_chars);
		self.optimize_with_cols(layout, 1000, &POSSIBLE_SWAPS)
	}

	pub fn optimize(&self, mut layout: FastLayout, trigram_precision: usize, possible_swaps: &[PosPair]) -> FastLayout {
		let mut best_score = f64::MIN / 2.0;
		let mut score = f64::MIN;
		let mut best_swap = &PosPair::default();

		while best_score != score {
			best_score = score;
			for swap in possible_swaps.iter() {
				layout.swap_pair_no_bounds(swap);
				let current = self.analysis.score(&layout, trigram_precision);

				if current > score {
					score = current;
					best_swap = swap;
				}
				layout.swap_pair_no_bounds(swap);
			}
			layout.swap_pair_no_bounds(best_swap);
		}
		layout
	}

	pub fn optimize_with_cols(&self, mut layout: FastLayout, trigram_precision: usize, possible_swaps: &[PosPair]) -> FastLayout {
		let mut best_score = f64::MIN / 2.0;
		let mut score = f64::MIN;
		let mut best_swap = &PosPair::default();
		// let mut sfb_best = 0.0;
		// let mut dsfb_best = 0.0;
		// let mut matrix_scores = self.score_whole_matrix(&layout);

		while best_score != score {
			while best_score != score {
				best_score = score;
				for swap in possible_swaps.iter() {
					layout.swap_pair_no_bounds(swap);
					let current = self.analysis.score(&layout, trigram_precision);

					if current > score {
						score = current;
						best_swap = swap;
					}
					layout.swap_pair_no_bounds(swap);
				}
				layout.swap_pair_no_bounds(best_swap);
			}
			score = self.optimize_cols(&mut layout, trigram_precision, Some(score));
		}
		layout
	}

	pub fn generate_n(&mut self, amount: usize) {
		if amount == 0 {
			return;
		}
		let mut layouts: Vec<(FastLayout, f64)> = Vec::with_capacity(amount);
		let start = std::time::Instant::now();
		
		let pb = ProgressBar::new(amount as u64);
		pb.set_style(ProgressStyle::default_bar()
			.template("[{elapsed_precise}] [{bar:40.white/white}] [eta: {eta}] - {per_sec:>4} {pos:>6}/{len}")
			.progress_chars("=>-"));

		(0..amount)
			.into_par_iter()
			.progress_with(pb)
			.map(|_| -> (FastLayout, f64) {
				let layout = self.generate();
				let score = self.analysis.score(&layout, usize::MAX);
				(layout, score)
			}).collect_into_vec(&mut layouts);

		println!("generating {} layouts took: {} seconds", amount, start.elapsed().as_secs());
		layouts.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());
		for (layout, score) in layouts.iter().take(10) {
			let printable = self.analysis.print_heatmap(layout);
			println!("{}\nscore: {:.5}", printable, score);
		}
		
		let temp_generated = layouts
			.into_iter()
			.map(|(x, _)| x.layout_str())
			.collect::<Vec<String>>();
		self.temp_generated = Some(temp_generated);
	}

	fn pinned_swaps(pins: &[usize]) -> Vec<PosPair> {
		let mut map = [false; 30];
		for i in 0..30 {
			if pins.contains(&i) {
				map[i] = true;
			}
		}
		let mut res = Vec::new();
		for ps in POSSIBLE_SWAPS {
			if !map[ps.0] && !map[ps.1] {
				res.push(ps);
			}
		}
		res
	}

	pub fn generate_pinned(&self, based_on: &FastLayout, pins: &[usize], possible_swaps: Option<&[PosPair]>) -> FastLayout {
		let layout = FastLayout::random_pins(based_on.matrix, pins);
		if let Some(ps) = possible_swaps {
			self.optimize(layout, 1000, ps)
		} else {
			self.optimize(layout, 1000, &Self::pinned_swaps(pins))
		}
	}

	pub fn generate_n_pins(&mut self, amount: usize, based_on: FastLayout, pins: &[usize]) {
		if amount == 0 {
			return;
		}
		let possible_swaps = Self::pinned_swaps(pins);
		let mut layouts: Vec<(FastLayout, f64)> = Vec::with_capacity(amount);
		let start = std::time::Instant::now();
		
		let pb = ProgressBar::new(amount as u64);
		pb.set_style(ProgressStyle::default_bar()
			.template("[{elapsed_precise}] [{bar:40.white/white}] [eta: {eta}] - {per_sec:>4} {pos:>6}/{len}")
			.progress_chars("=>-"));

		(0..amount)
			.into_par_iter()
			.progress_with(pb)
			.map(|_| -> (FastLayout, f64) {
				let layout = self.generate_pinned(&based_on, pins, Some(&possible_swaps));
				let score = self.analysis.score(&layout, usize::MAX);
				(layout, score)
			}).collect_into_vec(&mut layouts);

		println!("optmizing {} variants took: {} seconds", amount, start.elapsed().as_secs());
		layouts.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());
		
		for (layout, score) in layouts.iter().take(10) {
			let printable = self.analysis.print_heatmap(layout);
			println!("{}\nscore: {:.5}", printable, score);
		}

		let temp_generated = layouts
			.into_iter()
			.map(|(x, _)| x.layout_str())
			.collect::<Vec<String>>();
		
		self.temp_generated = Some(temp_generated);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn tr() {
		let a = LayoutAnalysis::new("tr", None);
		assert!(!a.is_err());
	}
}