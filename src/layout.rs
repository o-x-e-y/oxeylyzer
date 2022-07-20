use crate::utility::*;
use crate::generate::{Matrix, CharToFinger};
use crate::trigram_patterns::{TrigramPattern, TRIGRAM_COMBINATIONS};

#[inline]
fn shuffle_pins<T>(slice: &mut [T], pins: &[usize]) {
    let mapping: Vec<_> = (0..slice.len()).filter(|x| !pins.contains(x)).collect();

	for (m, &swap1) in mapping.iter().enumerate() {
        let swap2 = fastrand::usize(m..mapping.len());
        slice.swap(swap1, mapping[swap2]);
    }
}

pub trait Layout<T: Copy + Default> {
	fn new() -> Self;

	fn random(available_chars: [T; 30]) -> Self;

	fn random_pins(layout_chars: [T; 30], pins: &[usize]) -> Self;

	fn char(&self, x: usize, y: usize) -> T;

	fn char_by_index(&self, i: usize) -> T;

	fn swap(&mut self, i1: usize, i2: usize) -> Option<(T, T)>;

	fn swap_no_bounds(&mut self, i1: usize, i2: usize) -> (T, T);

	fn swap_pair(&mut self, pair: &PosPair) -> Option<(T, T)>;

	fn swap_pair_no_bounds(&mut self, pair: &PosPair) -> (T, T);

	fn swap_cols_no_bounds(&mut self, col1: usize, col2: usize);

	fn swap_indexes(&mut self);

	fn get_index(&self, index: usize) -> [T; 6];

	fn get_trigram_pattern(&self, trigram: &[T; 3]) -> TrigramPattern;

	unsafe fn get_trigram_pattern_unchecked(&self, trigram: &[char; 3]) -> TrigramPattern;
}

#[derive(Default, Clone)]
pub struct FastLayout {
	pub matrix: Matrix<char>,
	pub char_to_finger: CharToFinger<char>,
	pub score: f64
}

impl From<[char; 30]> for FastLayout {
    fn from(layout: [char; 30]) -> Self {
        let mut new_layout = FastLayout::new();

		for (i, c) in layout.into_iter().enumerate() {
			new_layout.matrix[i] = c;
			new_layout.char_to_finger.insert(c, COL_TO_FINGER[i%10]);
		}
		new_layout
    }
}

impl TryFrom<&str> for FastLayout {
    type Error = anyhow::Error;

    fn try_from(layout_str: &str) -> Result<Self, Self::Error> {  
        let mut new_layout = FastLayout::new();
		if layout_str.chars().count() == 30 {
			for (i, c) in layout_str.chars().enumerate() {
				new_layout.matrix[i] = c;
				new_layout.char_to_finger.insert(c, COL_TO_FINGER[i%10]);
			}
			Ok(new_layout)
		} else {
			Err(anyhow::Error::msg("string to create a layout should be 30 chars long"))
		}
    }
}

impl std::fmt::Display for FastLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut res = String::new();

        for (i, c) in self.matrix.iter().enumerate() {
			if i % 10 == 0 && i > 0 {
				res.push('\n');
			}
			if (i + 5) % 10 == 0 {
				res.push(' ');
			}
			res.push(*c);
			res.push(' ');
		};
		write!(f, "{res}")
    }
}

impl FastLayout {
	pub fn layout_str(&self) -> String {
		String::from_iter(self.matrix)
	}
}

impl Layout<char> for FastLayout {
	fn new() -> FastLayout {
		FastLayout {
			matrix: ['.'; 30],
			char_to_finger: CharToFinger::default(),
			score: 0.0
		}
	}

	fn random(mut available_chars: [char; 30]) -> FastLayout {
		fastrand::shuffle(&mut available_chars);
		FastLayout::from(available_chars)
	}

	fn random_pins(mut layout_chars: [char; 30], pins: &[usize]) -> FastLayout {
		shuffle_pins(&mut layout_chars, pins);
		FastLayout::from(layout_chars)
	}

	fn char(&self, x: usize, y: usize) -> char {
		assert!(x < 10 && y < 3);
		self.matrix[x + 10*y]
	}

	fn char_by_index(&self, i: usize) -> char {
		self.matrix[i]
	}

	fn swap(&mut self, i1: usize, i2: usize) -> Option<(char, char)> {
		if i1 < 30 && i2 < 30 {

			let char1 = self.matrix[i1];
			let char2 = self.matrix[i2];

			self.matrix[i1] = char2;
			self.matrix[i2] = char1;
			self.char_to_finger.insert(char1, COL_TO_FINGER[i2 % 10]);
			self.char_to_finger.insert(char2, COL_TO_FINGER[i1 % 10]);

			return Some((char1, char2))
		} else {
			println!("Invalid coordinate, swap was cancelled");
			None
		}
	}

	fn swap_no_bounds(&mut self, i1: usize, i2: usize) -> (char, char) {
		let char1 = self.matrix[i1];
		let char2 = self.matrix[i2];

		self.matrix[i1] = char2;
		self.matrix[i2] = char1;
		self.char_to_finger.insert(char1, COL_TO_FINGER[i2 % 10]);
		self.char_to_finger.insert(char2, COL_TO_FINGER[i1 % 10]);
		(char1, char2)
	}

	fn swap_pair(&mut self, pair: &PosPair) -> Option<(char, char)> {
		self.swap(pair.0, pair.1)
	}

	fn swap_pair_no_bounds(&mut self, pair: &PosPair) -> (char, char) {
		self.swap_no_bounds(pair.0, pair.1)
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
		let a = *self.char_to_finger.get(&trigram[0]).unwrap_or(&usize::MAX);
		let b = *self.char_to_finger.get(&trigram[1]).unwrap_or(&usize::MAX);
		let c = *self.char_to_finger.get(&trigram[2]).unwrap_or(&usize::MAX);
		if (a | b | c) == usize::MAX {
			return TrigramPattern::Invalid
		}
		// a, b and c are numbers between 0 and 7. This means they fit in exactly 3 bits (7 == 0b111)
		let combination = (a << 6) | (b << 3) | c;
		TRIGRAM_COMBINATIONS[combination]
	}

	unsafe fn get_trigram_pattern_unchecked(&self, trigram: &[char; 3]) -> TrigramPattern {
		let a = *self.char_to_finger.get(&trigram[0]).unwrap_unchecked();
		let b = *self.char_to_finger.get(&trigram[1]).unwrap_unchecked();
		let c = *self.char_to_finger.get(&trigram[2]).unwrap_unchecked();
		// a, b and c are numbers between 0 and 7. This means they fit in exactly 3 bits (7 == 0b111)
		let combination = (a << 6) | (b << 3) | c;
		TRIGRAM_COMBINATIONS[combination]
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn layout_str() {
		let qwerty = FastLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
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
		let mut qwerty = FastLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
		qwerty.swap(10, 11);
		assert_eq!(qwerty.layout_str(), "qwertyuiopsadfghjkl;zxcvbnm,./".to_owned());
	}

	#[test]
	fn swap_no_bounds() {
		let mut qwerty = FastLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
		qwerty.swap_no_bounds(9, 12);
		assert_eq!(qwerty.layout_str(), "qwertyuiodaspfghjkl;zxcvbnm,./".to_string());
	}

	#[test]
	fn swap_cols_no_bounds() {
		let mut qwerty = FastLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
		qwerty.swap_cols_no_bounds(1, 9);
		assert_eq!(
			qwerty.layout_str(), "qpertyuiowa;dfghjklsz/cvbnm,.x".to_string()
		);
	}

	#[test]
	fn swap_pair() {
		let mut qwerty = FastLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
		let new_swap = PosPair::new(0, 29);
		qwerty.swap_pair(&new_swap);
		assert_eq!(qwerty.layout_str(), "/wertyuiopasdfghjkl;zxcvbnm,.q".to_string());
	}

	#[test]
	fn swap_pair_no_bounds() {
		let mut qwerty = FastLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
		let new_swap = PosPair::new(0, 29);
		qwerty.swap_pair_no_bounds(&new_swap);
		assert_eq!(qwerty.layout_str(), "/wertyuiopasdfghjkl;zxcvbnm,.q".to_string());
	}

	#[test]
	fn char_to_finger() {
		let qwerty = FastLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
		assert_eq!(qwerty.char_to_finger.get(&'a'), Some(&0usize));
		assert_eq!(qwerty.char_to_finger.get(&'w'), Some(&1usize));
		assert_eq!(qwerty.char_to_finger.get(&'c'), Some(&2usize));

		assert_eq!(qwerty.char_to_finger.get(&'r'), Some(&3usize));
		assert_eq!(qwerty.char_to_finger.get(&'b'), Some(&3usize));

		assert_eq!(qwerty.char_to_finger.get(&'h'), Some(&4usize));
		assert_eq!(qwerty.char_to_finger.get(&'u'), Some(&4usize));

		assert_eq!(qwerty.char_to_finger.get(&'i'), Some(&5usize));
		assert_eq!(qwerty.char_to_finger.get(&'.'), Some(&6usize));
		assert_eq!(qwerty.char_to_finger.get(&';'), Some(&7usize));
	}

	#[test]
	fn char() {
		let qwerty = FastLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
		assert_eq!(qwerty.char(4, 1), 'g');
		assert_eq!(qwerty.char(9, 2), '/');
		assert_eq!(qwerty.char(8, 1), 'l');
	}

	#[test]
	fn char_by_index() {
		let qwerty = FastLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
		assert_eq!(qwerty.char_by_index(10), 'a');
		assert_eq!(qwerty.char_by_index(24), 'b');
		assert_eq!(qwerty.char_by_index(22), 'c');
	}

	#[test]
	fn get_trigram_pattern() {
		let qwerty = FastLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
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
		let qwerty = FastLayout::try_from("qwertyuiopasdfghjkl;zxcvbnm,./").unwrap();
		assert_eq!(qwerty.score, 0.0);
	}

	#[test]
	fn random_layouts() {
		use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
		use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
		use std::io::Write;
		use crate::analyze::LayoutAnalysis;

		let anal = LayoutAnalysis::new("english", None).unwrap();
		let available_chars = available_chars("english");

		let pb = ProgressBar::new(10_000_000);
		pb.set_style(ProgressStyle::default_bar()
			.template("[{elapsed_precise}] [{bar:40.white/white}] [eta: {eta}] - {per_sec:>4} {pos:>6}/{len}")
			.progress_chars("=>-"));
		
		let mut res = Vec::with_capacity(10_000_000);

		let start = std::time::Instant::now();

		(0..10_000_000)
			.into_par_iter()
			.progress_with(pb)
			.map(|_| -> f32 {
				let r = FastLayout::random(available_chars);
				anal.score(&r, 5_000) as f32
			})
			.collect_into_vec(&mut res);
		
		let end = std::time::Instant::now();
		res.sort_unstable_by(|a, b| b.partial_cmp(a).unwrap());
		println!("that took {}s.", (end - start).as_secs_f64());
		
		let mut f = std::fs::OpenOptions::new()
			.write(true)
			.create(true)
			.truncate(true)
			.open("10mil_scores")
			.unwrap();
		
		let mut to_save_vec = Vec::new();
		res
			.into_par_iter()
			.map(|v| v.to_string())
			.collect_into_vec(&mut to_save_vec);
		let to_save = to_save_vec.join("\n");

		f.write(to_save.as_bytes()).unwrap();
	}
}