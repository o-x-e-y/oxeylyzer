use std::hint::unreachable_unchecked;

use smallmap::Map;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use anyhow::Result;

use crate::utility::*;
use crate::trigram_patterns::TrigramPattern;
use crate::analyze::{LayoutAnalysis, TrigramStats};
use crate::language_data::TrigramData;
use crate::layout::*;
use crate::weights::{Weights, Config};

pub type CharToFinger<T> = Map<T, usize>;
pub type Matrix<T> = [T; 30];

#[derive(Default, Debug)]
pub struct LayoutCache {
	effort: [f64; 30],
	effort_total: f64,

	scissors: f64,

	usage: [f64; 8],
	usage_total: f64,

	fspeed: [f64; 8],
	fspeed_total: f64,

	trigrams_total: f64
}

impl LayoutCache {
	pub fn total_score(&self) -> f64 {
		self.trigrams_total - self.scissors - self.effort_total - self.usage_total - self.fspeed_total
	}
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
		
		if let Ok(analyzer) = LayoutAnalysis::new(
			language, Some(weights.clone())
		) {
			let available = available_chars(language);
			let possible_chars = analyzer.language_data.characters.iter()
				.map(|(c, _)| *c)
				.collect::<Vec<_>>();
			Ok(
				Self {
					per_char_trigrams: Self::per_char_trigrams(
						&analyzer.language_data.trigrams,
						possible_chars.as_ref(),
						trigram_precision
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
		
		let thingy: Vec<(char, Vec<([char; 3], f64)>)> = possible
			.into_iter()
			.map(|c| {
				let per_char = n_trigrams
					.iter()
					.map(|(t, f)| (t.clone(), f.clone()))
					.filter(|(t, _)| (*t).contains(c))
					.collect::<Vec<([char; 3], f64)>>();
				(*c, per_char)
			})
			.collect();
		
		PerCharTrigrams::from_iter(thingy)
	}

	fn trigram_score_vec(&self, layout: &FastLayout, trigrams: std::slice::Iter<'_, ([char; 3], f64)>) -> f64 {
		let mut freqs = TrigramStats::default();
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
	}

	fn trigram_has_char((t, f): (&[char; 3], f64), c: char) {
		t.contains(&c);
	}

	fn char_trigrams(&self, layout: &FastLayout, pos: [usize; 2]) -> f64 {
		let mut freqs = TrigramStats::default();

		let c1 = layout.c(pos[0]);
		let c2 = layout.c(pos[1]);

		let v1 = self.per_char_trigrams.get(&c1);
		let v2 = self.per_char_trigrams.get(&c2);

		match (v1, v2) {
			(None, None) => 0.0,
			(Some(v), None) | (None, Some(v)) => {
				self.trigram_score_vec(layout, v.into_iter())
			},
			(Some(v1), Some(v2)) => {
				let (big, small, c) =
					if v1.len() >= v2.len() { (v1, v2, c1) } else { (v2, v1, c2) };
				
				for (trigram, freq) in big.into_iter().chain(
					small.into_iter().filter(|(t, _)| t.contains(&c))
				) {
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
			}
		}
	}

	fn data(&self) -> &crate::language_data::LanguageData {
		&self.analysis.language_data
	}

	fn col_usage(&self, layout: &FastLayout, col: usize) -> f64 {
		let mut res = 0.0;
		match col {
			0 | 1 | 2 => {
				for c in [layout.c(col), layout.c(col+10), layout.c(col+20)] {
					res += *self.data().characters.get(&c).unwrap_or_else(|| &0.0);
				}
			},
			3 | 4 => {
				let col = (col - 3) * 2 + 3;
				for c in [layout.c(col), layout.c(col+10), layout.c(col+20),
								layout.c(col+1), layout.c(col+11), layout.c(col+21)] {
					res += *self.data().characters.get(&c).unwrap_or_else(|| &0.0);
				}
			},
			5 | 6 | 7 => {
				let col = col + 2;
				for c in [layout.c(col), layout.c(col+10), layout.c(col+20)] {
					res += *self.data().characters.get(&c).unwrap_or_else(|| &0.0);
				}
			},
			_ => unsafe { unreachable_unchecked() }
		};

		self.weights.max_finger_use.penalty * match col {
			0 | 7 => (res - self.weights.max_finger_use.pinky).max(0.0),
			1 | 6 => (res - self.weights.max_finger_use.ring).max(0.0),
			2 | 5 => (res - self.weights.max_finger_use.middle).max(0.0),
			3 | 4 => (res - self.weights.max_finger_use.index).max(0.0),
			_ => unsafe { unreachable_unchecked() }
		}
	}

	fn col_fspeed(&self, layout: &FastLayout, col: usize) -> f64 {
		let (start, len) = match col {
			0 | 1 | 2 => (col * 3, 3),
			3 | 4 => (18 + ((col - 3) * 15), 15),
			5 | 6 | 7 => ((col - 2) * 3, 3),
			_ => unsafe { unreachable_unchecked() }
		};

		let mut res = 0.0;
		let dsfb_ratio = self.weights.dsfb_ratio;

		for i in start..(start+len) {
			let (PosPair(i1, i2), dist) = self.analysis.fspeed_vals[i];

			let c1 = layout.c(i1);
			let c2 = layout.c(i2);

			res += self.data().bigrams.get(&[c1, c2]).unwrap_or_else(|| &0.0) * dist;
			res += self.data().bigrams.get(&[c2, c1]).unwrap_or_else(|| &0.0) * dist;

			res += self.data().skipgrams.get(&[c1, c2]).unwrap_or_else(|| &0.0) * dist * dsfb_ratio;
			res += self.data().skipgrams.get(&[c2, c1]).unwrap_or_else(|| &0.0) * dist * dsfb_ratio;
		}
		res
	}

	#[inline]
	fn char_effort(&self, layout: &FastLayout, i: usize) -> f64 {
		let c = layout.c(i);
		let mut res = *self.data().characters.get(&c).unwrap_or_else(|| &0.0);
		res *= self.analysis.effort_map[i];
		res
	}

	// #[inline]
	// fn has_key_off_homerow(pair: &PosPair) -> bool {
	// 	unsafe { *TF_TABLE.get_unchecked(pair.0) || *TF_TABLE.get_unchecked(pair.1) }
	// }

	// fn initialize_cache(&self, layout: &FastLayout) -> LayoutCache {
	// 	let mut res = LayoutCache::default();

	// 	res.scissors = self.analysis.scissor_percent(layout);

	// 	for i in 0..30 {
	// 		res.trigrams[i] = self.char_trigrams(layout, i);
	// 		res.effort[i] = self.char_effort(layout, i);
	// 	}
	// 	res.trigrams_total = res.trigrams.iter().sum();
	// 	res.effort_total = res.trigrams.iter().sum();

	// 	for col in 0..8 {
	// 		res.usage[col] = self.col_usage(layout, col);
	// 		res.fspeed[col] = self.col_fspeed(layout, col)
	// 	}
	// 	res.usage_total = res.usage.iter().sum();
	// 	res.fspeed_total = res.fspeed.iter().sum();

	// 	res
	// }

	// fn score_swap(&self, layout: &mut FastLayout, swap: &PosPair, cache: &LayoutCache) -> f64 {
	// 	unsafe { layout.swap_pair_no_bounds(swap) };
	// 	let PosPair(i1, i2) = *swap;

	// 	let col1 = self.analysis.i_to_col[i1];
	// 	let col2 = self.analysis.i_to_col[i2];

	// 	let f1 = self.col_fspeed(layout, col1);
	// 	let f2 = self.col_fspeed(layout, col2);
	// 	let new_fspeed = cache.fspeed_total - cache.fspeed[col1] - cache.fspeed[col2] + f1 + f2;
	// 	let fspeed_score = new_fspeed * self.weights.fspeed;

	// 	let u1 = self.col_usage(layout, col1);
	// 	let u2 = self.col_usage(layout, col2);
	// 	let new_usage = cache.usage_total - cache.usage[col1] - cache.usage[col2] + u1 + u2;
	// 	let usage_score = new_usage * self.weights.max_finger_use.penalty;

	// 	let e_new = self.char_effort(layout, &[i1, i2]);
	// 	let effort_score = cache.effort_total - cache.effort[i1] - cache.effort[i2] + e1 + e2;

	// 	let t1 = self.char_trigrams(layout, i1, false);
	// 	let t2 = self.char_trigrams(layout, i2, );
	// 	let trigrams_score = cache.trigrams_total - cache.trigrams[i1] - cache.trigrams[i2] + t1 + t2;

	// 	let scissors_score = if Self::has_key_off_homerow(swap) {
	// 		self.analysis.scissor_percent(layout) * self.weights.scissors
	// 	} else {
	// 		cache.scissors
	// 	};

	// 	unsafe { layout.swap_pair_no_bounds(swap) };
	// 	fspeed_score + usage_score + effort_score + trigrams_score + scissors_score
	// }

	// fn accept_swap(&self, layout: &mut FastLayout, swap: &PosPair, cache: &mut LayoutCache) {
	// 	unsafe { layout.swap_pair_no_bounds(swap) };
	// 	let PosPair(i1, i2) = *swap;

	// 	let col1 = self.analysis.i_to_col[i1];
	// 	let col2 = self.analysis.i_to_col[i2];

	// 	let f1 = self.col_fspeed(layout, col1);
	// 	let f2 = self.col_fspeed(layout, col2);
	// 	cache.fspeed_total = cache.fspeed_total - cache.fspeed[col1] - cache.fspeed[col2] + f1 + f2;
	// 	cache.fspeed[col1] = f1;
	// 	cache.fspeed[col2] = f2;

	// 	let u1 = self.col_usage(layout, col1);
	// 	let u2 = self.col_usage(layout, col2);
	// 	cache.usage_total = cache.usage_total - cache.usage[col1] - cache.usage[col2] + u1 + u2;
	// 	cache.usage[col1] = u1;
	// 	cache.usage[col2] = u2;

	// 	let e1 = self.char_effort(layout, i1);
	// 	let e2 = self.char_effort(layout, i2);
	// 	cache.effort_total = cache.effort_total - cache.effort[i1] - cache.effort[i2] + e1 + e2;
	// 	cache.effort[i1] = e1;
	// 	cache.effort[i2] = e2;

	// 	let t1 = self.char_trigrams(layout, i1);
	// 	let t2 = self.char_trigrams(layout, i2);
	// 	cache.trigrams_total = cache.trigrams_total - cache.trigrams[i1] - cache.trigrams[i2] + t1 + t2;
	// 	cache.trigrams[i1] = t1;
	// 	cache.trigrams[i2] = t2;

	// 	if Self::has_key_off_homerow(swap) {
	// 		cache.scissors = self.analysis.scissor_percent(layout) * self.weights.scissors;
	// 	}
	// }

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
				unsafe { layout.swap_cols_no_bounds(self.cols[i], self.cols[k - 1]) };
			} else {
				unsafe { layout.swap_cols_no_bounds(self.cols[0], self.cols[k - 1]) };
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
				unsafe { layout.swap_pair_no_bounds(swap) };
				let current = self.analysis.score(&layout, trigram_precision);

				if current > score {
					score = current;
					best_swap = swap;
				}
				unsafe { layout.swap_pair_no_bounds(swap) };
			}
			unsafe { layout.swap_pair_no_bounds(best_swap) };
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
					unsafe { layout.swap_pair_no_bounds(swap) };
					let current = self.analysis.score(&layout, trigram_precision);

					if current > score {
						score = current;
						best_swap = swap;
					}
					unsafe { layout.swap_pair_no_bounds(swap) };
				}
				unsafe { layout.swap_pair_no_bounds(best_swap) };
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