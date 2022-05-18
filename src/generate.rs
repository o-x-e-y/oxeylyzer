use std::collections::{HashMap, HashSet};
use std::fmt::Formatter;
use std::iter::FromIterator;
use std::sync::atomic::{AtomicUsize, Ordering};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use crate::analysis::*;
use crate::trigram_patterns::{TRIGRAM_COMBINATIONS, TrigramPattern};
use crate::{LayoutAnalysis, TrigramFreq};

#[derive(Default, Clone)]
pub struct Layout {
	pub matrix: LMatrix,
	pub char_to_finger: LCharToFinger,
}

impl Layout {
	pub fn new() -> Layout {
		Layout{ matrix: [['.'; 3]; 10], char_to_finger: LCharToFinger::new() }
	}

	pub fn from_str(layout: &str) -> Layout {
		if !Layout::is_valid_layout(layout) {
			panic!("brother {} is not a valid layout nooooooo", layout)
		}
		let mut new_layout = Layout::new();

		for (i, c) in layout.chars().enumerate() {
			new_layout.matrix[i%10][i/10] = c;
			new_layout.char_to_finger.insert(c, COL_TO_FINGER[i%10]);
		}
		new_layout
	}

	pub fn char(&self, x: usize, y: usize) -> char {
		assert![x < 10 && y < 3];
		self.matrix[x][y]
	}

	pub fn char_by_index(&self, i: usize) -> char {
		self.matrix[i%10][i/10]
	}

	pub fn swap(&mut self, x1: usize, y1: usize, x2: usize, y2: usize) -> bool {
		if x1 < 10 && x2 < 10 && y1 < 3 && y2 < 3 {

			let char1 = self.char(x1, y1);
			let char2 = self.char(x2, y2);

			self.matrix[x1][y1] = char2;
			self.matrix[x2][y2] = char1;
			self.char_to_finger.insert(char1, COL_TO_FINGER[x2]);
			self.char_to_finger.insert(char2, COL_TO_FINGER[x1]);

			return true
		}
		println!("Invalid coordinate, swap was cancelled");
		false
	}

	pub fn swap_no_bounds(&mut self, x1: usize, y1: usize, x2: usize, y2: usize) {
		let char1 = self.char(x1, y1);
		let char2 = self.char(x2, y2);

		self.matrix[x1][y1] = char2;
		self.matrix[x2][y2] = char1;
		self.char_to_finger.insert(char1, COL_TO_FINGER[x2]);
		self.char_to_finger.insert(char2, COL_TO_FINGER[x1]);
	}

	pub fn swap_pair(&mut self, pair: &PosPair) -> bool {
		self.swap(pair.0.x, pair.0.y, pair.1.x, pair.1.y)
	}

	pub fn swap_pair_no_bounds(&mut self, pair: &PosPair) {
		self.swap_no_bounds(pair.0.x, pair.0.y, pair.1.x, pair.1.y);
	}

	pub fn swap_cols_no_bounds(&mut self, col1: usize, col2: usize) {
		self.swap_no_bounds(col1, 0, col2, 0);
		self.swap_no_bounds(col1, 1, col2, 1);
		self.swap_no_bounds(col1, 2, col2, 2);
	}

	pub fn swap_indexes(&mut self) {
		self.swap_cols_no_bounds(3, 6);
		self.swap_cols_no_bounds(4, 5);
	}

	pub fn get_index(&self, index: usize) -> [char; 6] {
		let mut new_index = [' '; 6];
		let start_pos = index*2 + 3;
		for i in 0..2 {
			for j in 0..3 {
				new_index[2*j + i] = self.matrix[start_pos + i][j];
			}
		}
		new_index
	}

	pub fn get_trigram_pattern(&self, trigram: &[char; 3]) -> TrigramPattern {
		let a = *self.char_to_finger.get(&trigram[0]).unwrap_or(&u8::MAX);
		let b = *self.char_to_finger.get(&trigram[1]).unwrap_or(&u8::MAX);
		let c = *self.char_to_finger.get(&trigram[2]).unwrap_or(&u8::MAX);
		if (a | b | c) == u8::MAX {
			return TrigramPattern::Invalid
		}
		// a, b and c are numbers between 0 and 7. This means they fit in exactly 3 bits (7 = 0b111)
		let combination = ((a as usize) << 6) | ((b as usize) << 3) | c as usize;
		let thing = TRIGRAM_COMBINATIONS[combination];
		if thing == TrigramPattern::Other && (a != b && b != c) {
			println!("inroll: {}{}{}: {} {} {}", trigram[0], trigram[1], trigram[2], a, b, c);
		}
		thing
	}

	pub fn is_valid_layout(layout: &str) -> bool {
		let chars: HashSet<char> = HashSet::from_iter(layout.chars());
		layout.chars().count() == 30 && chars.len() == 30
	}

	pub fn random() -> Layout {
		let mut available_chars =
			// ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n','o', 'p',
			// 	'q', 'r', 's', 't', 'u', 'v', 'x', 'y', 'z', 'ë', 'ç', '.', ',', '\''];
			// ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n','o', 'p',
			// 	'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'ü', 'ä', 'ö', '.', ','];
			// ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n','o', 'p',
			// 	'æ', 'r', 's', 't', 'u', 'v', 'w', 'ø', 'y', 'å', '\'', ',', '.', ';'];
			['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n','o', 'p',
				'*', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '\'', ',', '.', ';'];
			// ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n','o', 'p',
			// 	'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '\'', ',', '.', ';'];
			fastrand::shuffle(&mut available_chars);
		let layout_str = available_chars.iter().collect::<String>();
		Layout::from_str(layout_str.as_str())
	}

	pub fn random_pinned() {

	}
}

impl std::fmt::Display for Layout {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let mut res = String::with_capacity(63);
		for y in 0..3 {
			res.push('\n');
			for x in 0..10 {
				res.push(self.char(x, y));
				res.push(' ');
			}
		}
		write![f, "{}", res]
	}
}

pub struct LayoutGeneration {
	pub analysis: LayoutAnalysis,
	pub improved_layout: Layout,
	cols: [usize; 6]
}

impl LayoutGeneration {

	pub fn new(language: &str) -> Self {
		Self {
			analysis: LayoutAnalysis::new(language),
			improved_layout: Layout::new(),
			cols: [0, 1, 2, 7, 8, 9],
		}
	}

	pub fn optimize_cols(&self, layout: &mut Layout, trigram_precision: usize, score: Option<f64>) -> f64 {
		let mut best_score = score.unwrap_or(self.analysis.score(layout, trigram_precision));

		let mut best = layout.clone();
		self.col_perms(layout, &mut best, &mut best_score, 6);
		layout.swap_indexes();

		self.col_perms(layout, &mut best, &mut best_score, 6);
		*layout = best;
		best_score
	}

	fn col_perms(&self, layout: &mut Layout, best: &mut Layout, best_score: &mut f64, k: usize) {
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

	pub fn exclude_chars(&self, excluded: &str, layout_name: &str) -> Vec<PosPair> {
		let layout = self.analysis.layout_by_name(layout_name)
			.unwrap_or_else(|| panic!("layout {} does not exist", layout_name));

		let i_to_char = |index: usize, layout: &Layout| -> char {
			layout.char(index % 10, index / 10)
		};
		let i_to_pos = |index: usize| -> Pos {
			Pos{x: index % 10, y: index / 10}
		};

		let mut res: Vec<PosPair> = Vec::new();
		for pos1 in 0..30 {
			for pos2 in (pos1 + 1)..30 {
				if !excluded.contains(i_to_char(pos1, &layout)) {
					res.push(PosPair(i_to_pos(pos1),i_to_pos(pos2)))
				}
			}
		}
		res
	}

	pub fn generate(&self) -> Layout {
		let layout = Layout::random();
		self.optimize(layout, 1000, &POSSIBLE_SWAPS)
	}

	pub fn generate_n_pinned(&self, amount: usize, pinned_chars: &str, base_name: &str) {
		if amount == 0 {
			return;
		}
		let mut layouts: Vec<LayoutScore> = Vec::with_capacity(amount);
		let possible_swaps = self.exclude_chars(pinned_chars, base_name).as_slice();
		let start = std::time::Instant::now();
		let i: AtomicUsize = AtomicUsize::new(0);
		(0..amount).into_par_iter()
			.map(|_| -> LayoutScore {
				let layout = self.generate();
				let score = self.analysis.score(&layout, usize::MAX);
				i.fetch_add(1, Ordering::SeqCst);
				let i_current = i.load(Ordering::SeqCst);
				if i_current % (if amount < 20 {amount} else {amount / 20}) == 0 {
					println!("{i_current}/{amount} done");
				}
				LayoutScore{layout, score}
			}).collect_into_vec(&mut layouts);

		// for i in 0..amount {
		// 	let layout = self.generate();
		// 	let score = self.analysis.score(&layout, 10000);
		// 	layouts.push(LayoutScore{layout, score});
		// 	// if i % (if amount < 20 {amount} else {amount / 20}) == 0 {
		// 	// 	println!("i: {}", i);
		// 	// }
		// 	println!("i: {}", i);
		// }
		println!("generating {} layouts took: {} seconds", amount, start.elapsed().as_secs());
		layouts.sort_unstable();
		for i in 0..(if amount < 10 {amount} else {10}) {
			println!("{}\nscore: {:.5}", layouts[i].layout, layouts[i].score);
		}
		println!("worst layout:\n{}\n{}", layouts[layouts.len()-1].layout, layouts[layouts.len()-1].score)
	}

	pub fn optimize(&self, mut layout: Layout, trigram_precision: usize, possible_swaps: &[PosPair]) -> Layout {
		let mut best_score = f64::MIN / 2.0;
		let mut best_swap = &PosPair::new();
		let mut score = f64::MIN;
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

	// pub fn generate_annealing(&self) -> Layout {
	// 	let layout = Layout::random();
	// 	self.optimize_annealing(layout)
	// }

	// pub fn optimize_annealing(&self, layout: Layout) -> Layout {
	// 	let mut temp = 1.0;
	// 	let temp_diff = 0.9999;
	// 	let alpha = |t: f64| -> f64 {
	// 		t / (1.0 + 0.5*t)
	// 	};

	// 	for swaps in 2..=3 {
	// 		for _ in 0..(3u32*(435u32).pow(swaps)) {
				
	// 		}
	// 	}
	// 	layout
	// }

	pub fn generate_n(&self, amount: usize) {
		if amount == 0 {
			return;
		}
		let mut layouts: Vec<LayoutScore> = Vec::with_capacity(amount);
		let start = std::time::Instant::now();
		let i: AtomicUsize = AtomicUsize::new(0);
		(0..amount).into_par_iter()
			.map(|_| -> LayoutScore {
				let layout = self.generate();
				let score = self.analysis.score(&layout, usize::MAX);
				i.fetch_add(1, Ordering::SeqCst);
				let i_current = i.load(Ordering::SeqCst);
				if i_current % (if amount < 20 {amount} else {amount / 20}) == 0 {
					println!("{i_current}/{amount} done");
				}
				std::thread::sleep(std::time::Duration::from_secs(1));
				LayoutScore{layout, score}
			}).collect_into_vec(&mut layouts);

		// for i in 0..amount {
		// 	let layout = self.generate();
		// 	let score = self.analysis.score(&layout, 10000);
		// 	layouts.push(LayoutScore{layout, score});
		// 	// if i % (if amount < 20 {amount} else {amount / 20}) == 0 {
		// 	// 	println!("i: {}", i);
		// 	// }
		// 	println!("i: {}", i);
		// }
		println!("generating {} layouts took: {} seconds", amount, start.elapsed().as_secs());
		layouts.sort_unstable();
		for i in 0..(if amount < 10 {amount} else {10}) {
			println!("{}\nscore: {:.5}", layouts[i].layout, layouts[i].score);
		}
		println!("worst layout:\n{}\n{}", layouts[layouts.len()-1].layout, layouts[layouts.len()-1].score)
	}
}

#[derive(Default)]
struct LayoutScore {
	layout: Layout,
	score: f64
}

impl std::cmp::Eq for LayoutScore {}

impl std::cmp::PartialEq for LayoutScore {
	fn eq(&self, other: &Self) -> bool {
		self.score == other.score
	}
}

impl PartialOrd<Self> for LayoutScore {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl std::cmp::Ord for LayoutScore {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		return if self.score > other.score {
			std::cmp::Ordering::Less
		} else if self.score == other.score {
			std::cmp::Ordering::Equal
		} else {
			std::cmp::Ordering::Greater
		}
	}
}

pub struct PerCharStats {
	sfbs: HashMap<char, f64>,
	dsfbs: HashMap<char, f64>,
	trigrams: HashMap<char, TrigramFreq>
}

impl PerCharStats {
	pub fn new() -> PerCharStats {
		PerCharStats {
			sfbs: HashMap::new(),
			dsfbs: HashMap::new(),
			trigrams: HashMap::new()
		}
	}

	pub fn from_layout(layout: Layout) {
		for col in layout.matrix {
			for char in col {
				println!("{}", layout.char_to_finger.get(&char).unwrap());
			}
		}
	}
}