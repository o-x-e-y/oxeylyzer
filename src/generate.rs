use std::hint::unreachable_unchecked;
use core::option::Option;

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

	fn per_char_trigrams(trigrams: &TrigramData, possible: &[char], trigram_precision: usize) -> PerCharTrigrams {
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

	#[inline]
	fn trigram_score_iter<'a, T>(&self, layout: &FastLayout, trigrams: T) -> f64
	where T: IntoIterator<Item=&'a ([char; 3], f64)> {
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

	fn trigram_char_score(&self, layout: &FastLayout, pos: &PosPair) -> f64 {
		let c1 = layout.c(pos.0);
		let c2 = layout.c(pos.1);

		let v1 = self.per_char_trigrams.get(&c1);
		let v2 = self.per_char_trigrams.get(&c2);

		match (v1, v2) {
			(None, None) => 0.0,
			(Some(v), None) | (None, Some(v)) => {
				self.trigram_score_iter(layout, v)
			},
			(Some(v1), Some(v2)) => {
				let (big, small, c) =
					if v1.len() >= v2.len() { (v1, v2, &c1) } else { (v2, v1, &c2) };
				
				let iter = big.into_iter().chain(
					small.into_iter().filter(|(t, _)| !t.contains(c))
				);
				self.trigram_score_iter(layout, iter)
			}
		}
	}

	const fn data(&self) -> &crate::language_data::LanguageData {
		&self.analysis.language_data
	}

	fn scissor_score(&self, layout: &FastLayout) -> f64 {
		self.analysis.scissor_percent(layout) * self.weights.scissors
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

	pub(self) fn col_to_start_len(col: usize) -> (usize, usize) {
		match col {
			0 | 1 | 2 => (col * 3, 3),
			3 | 4 => (18 + ((col - 3) * 15), 15),
			5 | 6 | 7 => ((col - 2) * 3, 3),
			_ => unsafe { unreachable_unchecked() }
		}
	}

	fn col_fspeed(&self, layout: &FastLayout, col: usize) -> f64 {
		let (start, len) = Self::col_to_start_len(col);

		let mut res = 0.0;
		let dsfb_ratio = self.weights.dsfb_ratio;
		let dsfb_ratio2 = self.weights.dsfb_ratio2;
		let dsfb_ratio3 = self.weights.dsfb_ratio3;

		for i in start..(start+len) {
			let (PosPair(i1, i2), dist) = self.analysis.fspeed_vals[i];

			let c1 = layout.c(i1);
			let c2 = layout.c(i2);

			let (pair, rev) = ([c1, c2], [c2, c1]);

			res += self.data().bigrams.get(&pair).unwrap_or_else(|| &0.0) * dist;
			res += self.data().bigrams.get(&rev).unwrap_or_else(|| &0.0) * dist;

			res += self.data().skipgrams.get(&pair).unwrap_or_else(|| &0.0) * dist * dsfb_ratio;
			res += self.data().skipgrams.get(&rev).unwrap_or_else(|| &0.0) * dist * dsfb_ratio;

			res += self.data().skipgrams2.get(&pair).unwrap_or_else(|| &0.0) * dist * dsfb_ratio2;
			res += self.data().skipgrams2.get(&rev).unwrap_or_else(|| &0.0) * dist * dsfb_ratio2;

			res += self.data().skipgrams3.get(&pair).unwrap_or_else(|| &0.0) * dist * dsfb_ratio3;
			res += self.data().skipgrams3.get(&rev).unwrap_or_else(|| &0.0) * dist * dsfb_ratio3;
		}

		res * self.weights.fspeed
	}

	#[inline]
	fn char_effort(&self, layout: &FastLayout, i: usize) -> f64 {
		let c = layout.c(i);
		let mut res = *self.data().characters.get(&c).unwrap_or_else(|| &0.0);
		res *= self.analysis.effort_map[i];
		res
	}

	fn initialize_cache(&self, layout: &FastLayout) -> LayoutCache {
		let mut res = LayoutCache::default();

		for i in 0..layout.matrix.len() {
			res.effort[i] = self.char_effort(layout, i);
		}
		res.effort_total = res.effort.iter().sum();

		for col in 0..8 {
			res.usage[col] = self.col_usage(layout, col);
			res.fspeed[col] = self.col_fspeed(layout, col)
		}
		res.usage_total = res.usage.iter().sum();
		res.fspeed_total = res.fspeed.iter().sum();

		res.scissors = self.scissor_score(layout);

		res.trigrams_total = self.analysis.trigram_score(layout, 1000);

		res
	}

	fn score_swap_cached(&self, layout: &mut FastLayout, swap: &PosPair, cache: &LayoutCache) -> f64 {
		let trigrams_start = self.trigram_char_score(layout, swap);

		unsafe { layout.swap_pair_no_bounds(swap) };

		let PosPair(i1, i2) = *swap;

		let col1 = self.analysis.i_to_col[i1];
		let col2 = self.analysis.i_to_col[i2];

		let fspeed_score = if col1 == col2 {
			let fspeed = self.col_fspeed(layout, col1);
			let new = cache.fspeed_total - cache.fspeed[col1] + fspeed;

			new
		} else {
			let fspeed1 = self.col_fspeed(layout, col1);
			let fspeed2 = self.col_fspeed(layout, col2);
			let new = cache.fspeed_total - cache.fspeed[col1] - cache.fspeed[col2] + fspeed1 + fspeed2;
			
			new
		};

		let usage_score = if col1 == col2 {
			let usage = self.col_usage(layout, col1);
			cache.usage_total - cache.usage[col1] + usage
		} else {
			let usage1 = self.col_usage(layout, col1);
			let usage2 = self.col_usage(layout, col2);
			cache.usage_total - cache.usage[col1] - cache.usage[col2] + usage1 + usage2
		};

		let effort1 = self.char_effort(layout, i1);
		let effort2 = self.char_effort(layout, i2);
		let effort_score = cache.effort_total - cache.effort[i1] - cache.effort[i2] + effort1 + effort2;

		let trigrams_end = self.trigram_char_score(layout, &swap);
		let trigrams_score = cache.trigrams_total - trigrams_start + trigrams_end;

		let scissors_score = if swap.affects_scissor() {
			self.scissor_score(layout)
		} else {
			cache.scissors
		};

		unsafe { layout.swap_pair_no_bounds(swap) };

		trigrams_score - scissors_score - effort_score - usage_score - fspeed_score

	}

	fn accept_swap(&self, layout: &mut FastLayout, swap: &PosPair, cache: &mut LayoutCache) {
		let trigrams_start = self.trigram_char_score(layout, swap);

		unsafe { layout.swap_pair_no_bounds(swap) };

		let PosPair(i1, i2) = *swap;

		let col1 = self.analysis.i_to_col[i1];
		let col2 = self.analysis.i_to_col[i2];

		cache.fspeed_total = if col1 == col2 {
			let fspeed = self.col_fspeed(layout, col1);
			let total = cache.fspeed_total - cache.fspeed[col1] + fspeed;

			cache.fspeed[col1] = fspeed;

			total
		} else {
			let fspeed1 = self.col_fspeed(layout, col1);
			let fspeed2 = self.col_fspeed(layout, col2);
			let total = cache.fspeed_total - cache.fspeed[col1] - cache.fspeed[col2]
				+ fspeed1 + fspeed2;

			cache.fspeed[col1] = fspeed1;
			cache.fspeed[col2] = fspeed2;

			total
		};

		cache.usage_total = if col1 == col2 {
			let usage = self.col_usage(layout, col1);
			let total = cache.usage_total - cache.usage[col1] + usage;

			cache.usage[col1] = usage;
			
			total
		} else {
			let usage1 = self.col_usage(layout, col1);
			let usage2 = self.col_usage(layout, col2);
			let total = cache.usage_total - cache.usage[col1] - cache.usage[col2] + usage1 + usage2;

			cache.usage[col1] = usage1;
			cache.usage[col2] = usage2;

			total
		};

		let effort1 = self.char_effort(layout, i1);
		let effort2 = self.char_effort(layout, i2);
		cache.effort_total = cache.effort_total - cache.effort[i1] - cache.effort[i2] + effort1 + effort2;
		cache.effort[i1] = effort1;
		cache.effort[i2] = effort2;

		let trigrams_end = self.trigram_char_score(layout, &swap);
		cache.trigrams_total = cache.trigrams_total - trigrams_start + trigrams_end;

		if swap.affects_scissor() {
			cache.scissors = self.scissor_score(layout);
		}
	}

	pub fn best_swap_cached(
		&self, layout: &mut FastLayout, cache: &LayoutCache, current_best_score: Option<f64>, possible_swaps: &[PosPair]
	) -> (Option<PosPair>, f64) {
		let mut best_score = current_best_score.unwrap_or_else(|| f64::MIN / 2.0);
		let mut best_swap: Option<PosPair> = None;

		for swap in possible_swaps {
			let score = self.score_swap_cached(layout, swap, cache);
			
			if score > best_score {
				best_score = score;
				best_swap = Some(*swap);
			}
		}

		(best_swap, best_score)
	}

	pub fn optimize_cached(&self, mut layout: FastLayout, cache: &mut LayoutCache, possible_swaps: &[PosPair]) -> FastLayout {
		let mut current_best_score = f64::MIN / 2.0;
		
		while let (Some(best_swap), new_score) =
			self.best_swap_cached(&mut layout, &cache, Some(current_best_score), possible_swaps) {
			current_best_score = new_score;
			self.accept_swap(&mut layout, &best_swap, cache);
		}
		layout
	}

	pub fn score_swap(&self, layout: &mut FastLayout, swap: &PosPair) -> f64 {
		unsafe { layout.swap_pair_no_bounds(swap) };
		let score = self.analysis.score(&layout, 1000);
		unsafe { layout.swap_pair_no_bounds(swap) };
		score
	}

	pub fn best_swap(
		&self, layout: &mut FastLayout, current_best_score: Option<f64>, possible_swaps: &[PosPair]
	) -> (Option<PosPair>, f64) {
		let mut best_score = current_best_score.unwrap_or_else(|| f64::MIN / 2.0);
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

	pub fn optimize(&self, mut layout: FastLayout, possible_swaps: &[PosPair]) -> FastLayout {
		let mut current_best_score = f64::MIN / 2.0;

		while let (Some(best_swap), new_score) =
			self.best_swap(&mut layout, Some(current_best_score), possible_swaps) {
			current_best_score = new_score;
			unsafe { layout.swap_pair_no_bounds(&best_swap) };
		}

		layout
	}

	pub fn optimize_with_cols(&self, mut layout: FastLayout, trigram_precision: usize, possible_swaps: &[PosPair]) -> FastLayout {
		let mut best_score = f64::MIN / 2.0;
		let mut score = f64::MIN;
		let mut best_swap = &PosPair::default();

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
			self.optimize(layout, ps)
		} else {
			self.optimize(layout, &Self::pinned_swaps(pins))
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
	use lazy_static::lazy_static;
	use crate::utility::ApproxEq;

	lazy_static!{
		pub static ref GEN: LayoutGeneration = LayoutGeneration::new("english", 1000, None).unwrap();
	}

	#[test]
	fn cached_scissors() {
		let mut qwerty = FastLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
		let mut cache = GEN.initialize_cache(&qwerty);

		for swap in POSSIBLE_SWAPS.iter() {
			GEN.accept_swap(&mut qwerty, swap, &mut cache);

			assert!(
				cache.scissors.approx_equal_dbg(
					GEN.analysis.scissor_percent(&qwerty) * GEN.weights.scissors, 7
				)
			);
			assert!(cache.scissors.approx_equal_dbg(GEN.scissor_score(&qwerty), 7));
		}
	}

	#[test]
	fn can_a_cache_swap() {
		let mut qwerty = FastLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
		let cache = GEN.initialize_cache(&qwerty);
		
		if let (Some(best_swap_normal), best_score_normal) =
			GEN.best_swap(&mut qwerty, None, &POSSIBLE_SWAPS) &&
			let (Some(best_swap_cached), best_score_cached) =
			GEN.best_swap_cached(&mut qwerty, &cache, None, &POSSIBLE_SWAPS) {
				
			if best_score_normal.approx_equal_dbg(best_score_cached, 7) {
				assert_eq!(best_swap_normal, best_swap_cached);
			} else {
				println!("scores not the same")
			}
		}
	}

	#[test]
	fn score_arbitrary_swaps() {
		let mut qwerty = FastLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
		let mut cache = GEN.initialize_cache(&qwerty);

		for swap in POSSIBLE_SWAPS.iter() {
			let score_normal = GEN.score_swap(&mut qwerty, swap);
			let score_cached = GEN.score_swap_cached(&mut qwerty, swap, &mut cache);
		
			assert!(score_normal.approx_equal_dbg(score_cached, 7));
		}
	}

	#[test]
	fn accept_swaps() {
		let mut qwerty = FastLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
		let mut cache = GEN.initialize_cache(&qwerty);

		assert!(cache.fspeed.iter().sum::<f64>().approx_equal(cache.fspeed_total, 7));
		assert!(cache.total_score().approx_equal(GEN.analysis.score(&qwerty, 1000), 7));

		for swap in POSSIBLE_SWAPS.iter() {
			GEN.accept_swap(&mut qwerty, swap, &mut cache);
			println!("swap: {swap}");

			assert!(cache.fspeed.iter().sum::<f64>().approx_equal(cache.fspeed_total, 7));
			assert!(cache.total_score().approx_equal(GEN.analysis.score(&qwerty, 1000), 7));
		}
	}

	#[test]
	fn test_col_fspeed() {
		let reference = [(0, 3), (3, 3), (6, 3), (18, 15), (33, 15), (9, 3), (12, 3), (15, 3)];
		for i in 0..8 {
			let test = LayoutGeneration::col_to_start_len(i);
			assert_eq!(test, reference[i]);
		}
	}

	#[test]
	fn optimize_qwerty() {
		let qwerty = FastLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();

		let optimized_normal = 
			GEN.optimize(qwerty.clone(), &POSSIBLE_SWAPS);

		println!("optimized normally:\n{}", GEN.analysis.print_heatmap(&optimized_normal));

		let mut cache = GEN.initialize_cache(&qwerty);
		let optimized_cached =
			GEN.optimize_cached(qwerty, &mut cache, &POSSIBLE_SWAPS);

		println!("optimized with cache:\n{}", GEN.analysis.print_heatmap(&optimized_cached));
		
		let final_score = GEN.analysis.score(&optimized_cached, 1000);
		let final_score_cached = cache.total_score();
		assert!(final_score.approx_equal_dbg(final_score_cached, 7));
	}
}