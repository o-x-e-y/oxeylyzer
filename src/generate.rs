use std::collections::HashSet;
use std::fmt::Formatter;
use std::iter::FromIterator;
use std::ops::Deref;
use crate::analysis::*;
use crate::language_data::LanguageData;
use crate::trigram_patterns::{TRIGRAM_COMBINATIONS, TrigramPattern};
use fastrand;
use crate::LayoutAnalysis;

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
		layout.len() == 30 && chars.len() == 30
	}

	pub fn random() -> Layout {
		let mut available_chars =
			['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n','o', 'p',
				'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '\'', ',', '.', ';'];
		fastrand::shuffle(&mut available_chars);
		let layout_str = available_chars.iter().collect::<String>();
		let l = Layout::from_str(layout_str.as_str());
		//println!("{}", l);
		l
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
	pub improved_layout: Layout
}

impl LayoutGeneration {
	pub fn new(language: &str) -> LayoutGeneration {
		LayoutGeneration {
			analysis: LayoutAnalysis::new(language),
			improved_layout: Layout::new()
		}
	}

	pub fn generate(&self) -> Layout {
		let mut layout = Layout::random();
		self.optimize(layout)
	}

	pub fn optimize(&self, mut layout: Layout) -> Layout {
		let mut best_score = f64::MIN / 2.0;
		let mut best_swap = &PosPair::new();
		let mut score = f64::MIN;
		while best_score != score {
			best_score = score;
			for swap in &POSSIBLE_SWAPS {
				layout.swap_pair_no_bounds(swap);
				let current = self.analysis.score(&layout, 500);
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

	pub fn optimize_annealing(&self, layout: &Layout) {}

	pub fn generate_n(&self, amount: usize) {
		let mut layouts: Vec<LayoutScore> = Vec::with_capacity(amount);
		use std::time::Instant;
		let start = Instant::now();
		for i in 0..amount {
			let layout = self.generate();
			let score = self.analysis.score(&layout, 10000);
			layouts.push(LayoutScore{layout, score});
			if i % 10 == 0 {
				println!("i: {}", i);
			}
		}
		println!("generating {} layouts took: {} seconds", amount, start.elapsed().as_secs());
		layouts.sort_unstable();
		for i in 0..10 {
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