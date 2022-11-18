#[derive(Copy, Clone, PartialEq, Debug)]
pub enum TrigramPattern {
	Alternate,
	AlternateSfs,
	Inroll,
	Outroll,
	Onehand,
	Redirect,
	BadRedirect,
	Sfb,
	BadSfb,
	Sft,
	Other,
	Invalid
}

#[derive(Debug)]
struct Trigram {
	c1: usize, c2: usize, c3: usize,
	lh1: bool, lh2: bool, lh3: bool
}

impl std::fmt::Display for Trigram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.c1, self.c2, self.c3)
    }
}

impl Trigram {
	const fn lh(col: usize) -> bool {
		col < 4
	}

	const fn new(c1: usize, c2: usize, c3: usize) -> Self {
		Trigram {
			c1, c2, c3, lh1: Self::lh(c1), lh2: Self::lh(c2), lh3: Self::lh(c3)
		}
	}

	const fn is_alt(&self) -> bool {
		self.lh1 != self.lh2 && self.lh2 != self.lh3
	}

	const fn get_alternate(&self) -> TrigramPattern {
		if self.c1 == self.c3 {
			TrigramPattern::AlternateSfs
		} else {
			TrigramPattern::Alternate
		}
	}

	const fn is_roll(&self) -> bool {
		let r = self.lh1 as usize + self.lh2 as usize + self.lh3 as usize;
		!self.is_alt() &&
		(r == 1 || r == 2) &&
		self.c1 != self.c2 &&
		self.c2 != self.c3
	}

	const fn get_roll(&self) -> TrigramPattern {
		if self.lh1 && self.lh2 && !self.lh3 {
			Self::particular_roll(self.c2, self.c1)
		} else if !self.lh1 && self.lh2 && self.lh3 {
			Self::particular_roll(self.c3, self.c2)
		} else if !self.lh1 && !self.lh2 && self.lh3 {
			Self::particular_roll(self.c1, self.c2)
		} else if self.lh1 && !self.lh2 && !self.lh3 {
			Self::particular_roll(self.c2, self.c3)
		} else {
			TrigramPattern::Other
		}
	}

	const fn particular_roll(f1: usize, f2: usize) -> TrigramPattern {
		if f1 > f2 {
			return TrigramPattern::Outroll
		}
		TrigramPattern::Inroll
	}

	const fn on_one_hand(&self) -> bool {
		self.lh1 == self.lh2 && self.lh2 == self.lh3
	}

	const fn is_redir(&self) -> bool {
		(self.c1 < self.c2 && self.c2 > self.c3) || (self.c1 > self.c2 && self.c2 < self.c3)
	}

	const fn is_bad_redir(&self) -> bool {
		!(self.c1 == 3 || self.c2 == 3 || self.c3 == 3 || self.c1 == 4 || self.c2 == 4 || self.c3 == 4)
	}

	const fn has_sfb(&self) -> bool {
		self.c1 == self.c2 || self.c2 == self.c3
	}

	const fn is_sft(&self) -> bool {
		self.c1 == self.c2 && self.c2 == self.c3
	}

	const fn get_one_hand(&self) -> TrigramPattern {
		if self.is_sft() {
			TrigramPattern::Sft
		} else if self.has_sfb() {
			TrigramPattern::BadSfb
		}
		else if self.is_redir() {
			if self.is_bad_redir() {
				TrigramPattern::BadRedirect
			} else {
				TrigramPattern::Redirect
			}
		} else if (self.c1 > self.c2 && self.c2 > self.c3) || (self.c1 < self.c2 && self.c2 < self.c3) {
			TrigramPattern::Onehand
		} else {
			TrigramPattern::Other
		}
	}

	const fn get_trigram_pattern(&self) -> TrigramPattern {
		if self.is_alt() {
			self.get_alternate()
		} else if self.on_one_hand() {
			self.get_one_hand()
		} else if self.has_sfb() {
			TrigramPattern::Sfb
		} else if self.is_roll() {
			self.get_roll()
		} else {
			TrigramPattern::Other
		}
	}
}

const fn get_trigram_combinations() -> [TrigramPattern; 512] {
	let mut combinations: [TrigramPattern; 512] = [TrigramPattern::Other; 512];

	let mut c3 = 0;
	while c3 < 8 {
		let mut c2 = 0;
		while c2 < 8 {
			let mut c1 = 0;
			while c1 < 8 {
				let index = c3*64 + c2*8 + c1;
				let trigram = Trigram::new(c1, c2, c3);
				combinations[index] = trigram.get_trigram_pattern();
				c1 += 1;
			}
			c2 += 1;
		}
		c3 += 1;
	}
	combinations
}

pub static TRIGRAM_COMBINATIONS: [TrigramPattern; 512] = get_trigram_combinations();

#[cfg(test)]
mod tests {
	use crate::*;
	use layout::{FastLayout, Layout};
	use trigram_patterns::TrigramPattern::*;
	use utility::ConvertU8;
	use once_cell::sync::Lazy;

	static CON: Lazy<ConvertU8> = Lazy::new(
		|| ConvertU8::with_chars("abcdefghijklmnopqrstuvwxyz',.;")
	);

	#[test]
	fn trigram_combinations() {
		let dvorak_bytes = CON.to_lossy("',.pyfgcrlaoeuidhtns;qjkxbmwvz".chars());
		let dvorak = FastLayout::try_from(dvorak_bytes.as_slice())
			.expect("couldn't create dvorak");

		assert_eq!(dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['h', 'o', 't'])), Alternate);
		assert_eq!(dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['l', 'a', 'z'])), AlternateSfs);

		assert_eq!(dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['a', 'b', 'c'])), Outroll);
		assert_eq!(dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['t', 'h', 'e'])), Inroll);
		assert_eq!(dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['r', 't', 'h'])), Onehand);
		assert_eq!(dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['h', 't', 'r'])), Onehand);

		assert_eq!(dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['c', 'b', 't'])), Redirect);
		assert_eq!(dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['r', 't', 's'])), BadRedirect);

		assert_eq!(dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['g', 'h', 't'])), BadSfb);
		assert_eq!(dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['p', 'u', 'k'])), Sft);
    }
}