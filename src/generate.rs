use fxhash::FxHashMap;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use anyhow::Result;

use crate::analysis::*;
use crate::trigram_patterns::{TRIGRAM_COMBINATIONS, TrigramPattern};
use crate::analyze::LayoutAnalysis;

pub type CharToFinger = FxHashMap<char, u8>;
pub type Matrix = [char; 30];

#[inline]
pub fn shuffle_pins<T>(slice: &mut [T], pins: &[usize]) {
    let mapping: Vec<_> = (0..slice.len()).filter(|x| !pins.contains(x)).collect();

	for (m, &swap1) in mapping.iter().enumerate() {
        let swap2 = fastrand::usize(m..mapping.len());
        slice.swap(swap1, mapping[swap2]);
    }
}

pub trait Layout {
	fn layout_str(&self) -> String;

	fn char(&self, x: usize, y: usize) -> char;

	fn char_by_index(&self, i: usize) -> char;

	fn swap(&mut self, i1: usize, i2: usize) -> bool;

	fn swap_no_bounds(&mut self, i1: usize, i2: usize);

	fn swap_pair(&mut self, pair: &PosPair) -> bool;

	fn swap_pair_no_bounds(&mut self, pair: &PosPair);

	fn swap_cols_no_bounds(&mut self, col1: usize, col2: usize);

	fn swap_indexes(&mut self);

	fn get_index(&self, index: usize) -> [char; 6];

	fn get_trigram_pattern(&self, trigram: &[char; 3]) -> TrigramPattern;

	fn random(available_chars: [char; 30]) -> Self;

	fn random_pins(layout_chars: [char; 30], pins: &[usize]) -> Self;
}

#[derive(Default, Clone)]
pub struct BasicLayout {
	pub matrix: Matrix,
	pub char_to_finger: CharToFinger,
	pub score: f64
}

impl BasicLayout {
	pub fn new() -> BasicLayout {
		BasicLayout {
			matrix: ['.'; 30],
			char_to_finger: CharToFinger::default(),
			score: 0.0
		}
	}
}

impl TryFrom<&str> for BasicLayout {
    type Error = anyhow::Error;

    fn try_from(layout: &str) -> Result<Self, Self::Error> {
		// if !BasicLayout::is_valid_layout(layout) {
		// 	panic!("brother {} is not a valid layout nooooooo", layout)
		// }
		let mut new_layout = BasicLayout::new();

		for (i, c) in layout.chars().enumerate() {
			new_layout.matrix[i] = c;
			new_layout.char_to_finger.insert(c, COL_TO_FINGER[i%10]);
		}
		Ok(new_layout)
    }
}

impl From<[char; 30]> for BasicLayout {
    fn from(layout: [char; 30]) -> Self {
        let mut new_layout = BasicLayout::new();

		for (i, c) in layout.into_iter().enumerate() {
			new_layout.matrix[i] = c;
			new_layout.char_to_finger.insert(c, COL_TO_FINGER[i%10]);
		}
		new_layout
    }
}

impl Layout for BasicLayout {
	fn layout_str(&self) -> String {
		self.matrix.iter().collect::<String>()
	}

	fn char(&self, x: usize, y: usize) -> char {
		assert!(x < 10 && y < 3);
		self.matrix[x + 10*y]
	}

	fn char_by_index(&self, i: usize) -> char {
		self.matrix[i]
	}

	fn swap(&mut self, i1: usize, i2: usize) -> bool {
		if i1 < 30 && i2 < 30 {

			let char1 = self.matrix[i1];
			let char2 = self.matrix[i2];

			self.matrix[i1] = char2;
			self.matrix[i2] = char1;
			self.char_to_finger.insert(char1, COL_TO_FINGER[i2 % 10]);
			self.char_to_finger.insert(char2, COL_TO_FINGER[i1 % 10]);

			return true
		}
		println!("Invalid coordinate, swap was cancelled");
		false
	}

	fn swap_no_bounds(&mut self, i1: usize, i2: usize) {
		let char1 = self.matrix[i1];
		let char2 = self.matrix[i2];

		self.matrix[i1] = char2;
		self.matrix[i2] = char1;
		self.char_to_finger.insert(char1, COL_TO_FINGER[i2 % 10]);
		self.char_to_finger.insert(char2, COL_TO_FINGER[i1 % 10]);
	}

	fn swap_pair(&mut self, pair: &PosPair) -> bool {
		self.swap(pair.0, pair.1)
	}

	fn swap_pair_no_bounds(&mut self, pair: &PosPair) {
		self.swap_no_bounds(pair.0, pair.1);
	}

	fn swap_cols_no_bounds(&mut self, col1: usize, col2: usize) {
		self.swap_no_bounds(col1, col2);
		self.swap_no_bounds(col1 + 10, col2 + 10);
		self.swap_no_bounds(col1 + 20, col2 + 20);
	}

	fn swap_indexes(&mut self) {
		self.swap_cols_no_bounds(3, 6);
		self.swap_cols_no_bounds(4, 5);
	}

	fn get_index(&self, index: usize) -> [char; 6] {
		let mut new_index = [' '; 6];
		let start_pos = index*2 + 3;
		for i in 0..2 {
			for j in 0..3 {
				new_index[2*j + i] = self.matrix[start_pos + i + 10*j];
			}
		}
		new_index
	}

	fn get_trigram_pattern(&self, trigram: &[char; 3]) -> TrigramPattern {
		let a = *self.char_to_finger.get(&trigram[0]).unwrap_or(&u8::MAX);
		let b = *self.char_to_finger.get(&trigram[1]).unwrap_or(&u8::MAX);
		let c = *self.char_to_finger.get(&trigram[2]).unwrap_or(&u8::MAX);
		if (a | b | c) == u8::MAX {
			return TrigramPattern::Invalid
		}
		// a, b and c are numbers between 0 and 7. This means they fit in exactly 3 bits (7 = 0b111)
		let combination = ((a as usize) << 6) | ((b as usize) << 3) | c as usize;
		TRIGRAM_COMBINATIONS[combination]
	}

	// fn is_valid_layout(layout: &str) -> bool {
	// 	let chars: FxHashSet<char> = FxHashSet::from_iter(layout.chars());
	// 	layout.chars().count() == 30 && chars.len() == 30
	// }

	fn random(mut available_chars: [char; 30]) -> BasicLayout {
		fastrand::shuffle(&mut available_chars);
		BasicLayout::from(available_chars)
	}

	fn random_pins(mut layout_chars: [char; 30], pins: &[usize]) -> BasicLayout {
		shuffle_pins(&mut layout_chars, pins);
		BasicLayout::from(layout_chars)
	}
}

impl std::fmt::Display for BasicLayout {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut res = String::with_capacity(63);
		for (i, c) in self.matrix.iter().enumerate() {
			if i % 10 == 0 && i > 0 {
				res.push('\n');
			}
			if (i + 5) % 10 == 0 {
				res.push(' ');
			}
			res.push(*c);
			res.push(' ');
		}
		write![f, "{}", res]
	}
}

pub struct LayoutGeneration {
	pub available_chars: [char; 30],
	pub analysis: LayoutAnalysis,
	pub improved_layout: BasicLayout,
	pub temp_generated: Option<Vec<String>>,
	cols: [usize; 6],
}

impl LayoutGeneration {
	pub fn new(language: &str, weights: crate::analyze::Weights) -> Result<Self> {
		if let Ok(analyzer) = LayoutAnalysis::new(language, weights) {
			Ok(
				Self {
					analysis: analyzer,
					improved_layout: BasicLayout::new(),
					available_chars: crate::analysis::available_chars(language),
					temp_generated: None,
					cols: [0, 1, 2, 7, 8, 9],
				}
			)
		} else {
			anyhow::bail!("Could not initalize analyzer.")
		}
	}

	pub fn optimize_cols(&self, layout: &mut BasicLayout, trigram_precision: usize, score: Option<f64>) -> f64 {
		let mut best_score = score.unwrap_or(self.analysis.score(layout, trigram_precision));

		let mut best = layout.clone();
		self.col_perms(layout, &mut best, &mut best_score, 6);
		layout.swap_indexes();

		self.col_perms(layout, &mut best, &mut best_score, 6);
		*layout = best;
		best_score
	}

	fn col_perms(&self, layout: &mut BasicLayout, best: &mut BasicLayout, best_score: &mut f64, k: usize) {
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

	pub fn generate(&self) -> BasicLayout {
		let layout = BasicLayout::random(self.available_chars);
		self.optimize_with_cols(layout, 1000, &POSSIBLE_SWAPS)
	}

	pub fn optimize(&self, mut layout: BasicLayout, trigram_precision: usize, possible_swaps: &[PosPair]) -> BasicLayout {
		let mut best_score = f64::MIN / 2.0;
		let mut best_swap = PosPair::default();
		let mut score = f64::MIN;
		// while best_score != score {
			while best_score != score {
				best_score = score;
				for swap in possible_swaps.iter() {
					layout.swap_pair_no_bounds(swap);
					let current = self.analysis.score(&layout, trigram_precision);
					if current > score {
						score = current;
						best_swap = *swap;
					}
					layout.swap_pair_no_bounds(swap);
				}
				layout.swap_pair_no_bounds(&best_swap);
			}
		// 	score = self.optimize_cols(&mut layout, trigram_precision, Some(score));
		// }
		layout
	}

	pub fn optimize_with_cols(&self, mut layout: BasicLayout, trigram_precision: usize, possible_swaps: &[PosPair]) -> BasicLayout {
		let mut best_score = f64::MIN / 2.0;
		let mut best_swap = PosPair::default();
		let mut score = f64::MIN;
		while best_score != score {
			while best_score != score {
				best_score = score;
				for swap in possible_swaps.iter() {
					layout.swap_pair_no_bounds(swap);
					let current = self.analysis.score(&layout, trigram_precision);
					if current > score {
						score = current;
						best_swap = *swap;
					}
					layout.swap_pair_no_bounds(swap);
				}
				layout.swap_pair_no_bounds(&best_swap);
			}
			score = self.optimize_cols(&mut layout, trigram_precision, Some(score));
		}
		layout
	}

	pub fn generate_n(&mut self, amount: usize) {
		if amount == 0 {
			return;
		}
		let mut layouts: Vec<(BasicLayout, f64)> = Vec::with_capacity(amount);
		let start = std::time::Instant::now();
		
		let pb = ProgressBar::new(amount as u64);
		pb.set_style(ProgressStyle::default_bar()
			.template("[{elapsed_precise}] [{bar:40.white/white}] [eta: {eta}] - {per_sec:>4} {pos:>6}/{len}")
			.progress_chars("=>-"));

		(0..amount)
			.into_par_iter()
			.progress_with(pb)
			.map(|_| -> (BasicLayout, f64) {
				let layout = self.generate();
				let score = self.analysis.score(&layout, usize::MAX);
				(layout, score)
			}).collect_into_vec(&mut layouts);

		println!("generating {} layouts took: {} seconds", amount, start.elapsed().as_secs());
		layouts.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());
		for (layout, score) in layouts.iter().take(10) {
			println!("{}\nscore: {:.5}", layout, score);
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

	pub fn generate_pinned(&self, based_on: &BasicLayout, pins: &[usize], possible_swaps: Option<&[PosPair]>) -> BasicLayout {
		let layout = BasicLayout::random_pins(based_on.matrix, pins);
		if let Some(ps) = possible_swaps {
			self.optimize(layout, 1000, ps)
		} else {
			self.optimize(layout, 1000, &Self::pinned_swaps(pins))
		}
	}

	pub fn generate_n_pins(&mut self, amount: usize, based_on: BasicLayout, pins: &[usize]) {
		if amount == 0 {
			return;
		}
		let possible_swaps = Self::pinned_swaps(pins);
		let mut layouts: Vec<(BasicLayout, f64)> = Vec::with_capacity(amount);
		let start = std::time::Instant::now();
		
		let pb = ProgressBar::new(amount as u64);
		pb.set_style(ProgressStyle::default_bar()
			.template("[{elapsed_precise}] [{bar:40.white/white}] [eta: {eta}] - {per_sec:>4} {pos:>6}/{len}")
			.progress_chars("=>-"));

		(0..amount)
			.into_par_iter()
			.progress_with(pb)
			.map(|_| -> (BasicLayout, f64) {
				let layout = self.generate_pinned(&based_on, pins, Some(&possible_swaps));
				let score = self.analysis.score(&layout, usize::MAX);
				(layout, score)
			}).collect_into_vec(&mut layouts);

		println!("optmizing {} variants took: {} seconds", amount, start.elapsed().as_secs());
		layouts.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());
		for (layout, score) in layouts.iter().take(10) {
			println!("{}\nscore: {:.5}", layout, score);
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
	fn layout_str() {
		let qwerty = BasicLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
		assert_eq!(
			qwerty.matrix,
			[
				'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p',
				'a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l', ';',
				'z', 'x', 'c', 'v', 'b', 'n', 'm', ',', '.', '/'
			]
		);
		assert_eq!(qwerty.layout_str(), "qwertyuiopasdfghjkl;zxcvbnm,./".to_string());
	}

	#[test]
	fn swap() {
		let mut qwerty = BasicLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
		qwerty.swap(10, 11);
		assert_eq!(qwerty.layout_str(), "qwertyuiopsadfghjkl;zxcvbnm,./".to_owned());
	}

	#[test]
	fn swap_no_bounds() {
		let mut qwerty = BasicLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
		qwerty.swap_no_bounds(9, 12);
		assert_eq!(qwerty.layout_str(), "qwertyuiodaspfghjkl;zxcvbnm,./".to_string());
	}

	#[test]
	fn swap_cols_no_bounds() {
		let mut qwerty = BasicLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
		qwerty.swap_cols_no_bounds(1, 9);
		assert_eq!(
			qwerty.layout_str(), "qpertyuiowa;dfghjklsz/cvbnm,.x".to_string()
		);
	}

	#[test]
	fn swap_pair() {
		let mut qwerty = BasicLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
		let new_swap = PosPair::new(0, 29);
		qwerty.swap_pair(&new_swap);
		assert_eq!(qwerty.layout_str(), "/wertyuiopasdfghjkl;zxcvbnm,.q".to_string());
	}

	#[test]
	fn swap_pair_no_bounds() {
		let mut qwerty = BasicLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
		let new_swap = PosPair::new(0, 29);
		qwerty.swap_pair_no_bounds(&new_swap);
		assert_eq!(qwerty.layout_str(), "/wertyuiopasdfghjkl;zxcvbnm,.q".to_string());
	}

	#[test]
	fn char_to_finger() {
		let qwerty = BasicLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
		assert_eq!(qwerty.char_to_finger.get(&'a'), Some(&0u8));
		assert_eq!(qwerty.char_to_finger.get(&'w'), Some(&1u8));
		assert_eq!(qwerty.char_to_finger.get(&'c'), Some(&2u8));

		assert_eq!(qwerty.char_to_finger.get(&'r'), Some(&3u8));
		assert_eq!(qwerty.char_to_finger.get(&'b'), Some(&3u8));

		assert_eq!(qwerty.char_to_finger.get(&'h'), Some(&4u8));
		assert_eq!(qwerty.char_to_finger.get(&'u'), Some(&4u8));

		assert_eq!(qwerty.char_to_finger.get(&'i'), Some(&5u8));
		assert_eq!(qwerty.char_to_finger.get(&'.'), Some(&6u8));
		assert_eq!(qwerty.char_to_finger.get(&';'), Some(&7u8));
	}

	#[test]
	fn char() {
		let qwerty = BasicLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
		assert_eq!(qwerty.char(4, 1), 'g');
		assert_eq!(qwerty.char(9, 2), '/');
		assert_eq!(qwerty.char(8, 1), 'l');
	}

	#[test]
	fn char_by_index() {
		let qwerty = BasicLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
		assert_eq!(qwerty.char_by_index(10), 'a');
		assert_eq!(qwerty.char_by_index(24), 'b');
		assert_eq!(qwerty.char_by_index(22), 'c');
	}

	#[test]
	fn get_trigram_pattern() {
		let qwerty = BasicLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
		assert_eq!(TrigramPattern::Alternate, qwerty.get_trigram_pattern(&['r', 'o', 'd']));
		assert_eq!(TrigramPattern::AlternateSfs, qwerty.get_trigram_pattern(&['j', 'a', 'y']));

		assert_eq!(TrigramPattern::Inroll, qwerty.get_trigram_pattern(&['w', 'o', 'u']));
		assert_eq!(TrigramPattern::Outroll, qwerty.get_trigram_pattern(&['m', 'o', 't']));
		assert_eq!(TrigramPattern::Onehand, qwerty.get_trigram_pattern(&['s', 'e', 'r']));

		assert_eq!(TrigramPattern::Redirect, qwerty.get_trigram_pattern(&['y', 'o', 'u']));
		assert_eq!(TrigramPattern::BadRedirect, qwerty.get_trigram_pattern(&['s', 'a', 'd']));

		assert_eq!(TrigramPattern::Other, qwerty.get_trigram_pattern(&['s', 's', 'h']));
		assert_eq!(TrigramPattern::Other, qwerty.get_trigram_pattern(&['s', 's', 's']));

		assert_eq!(TrigramPattern::Invalid, qwerty.get_trigram_pattern(&['d', '\'', 'n']));
		assert_eq!(TrigramPattern::Invalid, qwerty.get_trigram_pattern(&['\'', 'l', 'l']));
		assert_eq!(TrigramPattern::Invalid, qwerty.get_trigram_pattern(&['l', 'l', ']']));
	}

	#[test]
	fn thing() {
		let qwerty = BasicLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
		assert_eq!(qwerty.score, 0.0);
	}
}