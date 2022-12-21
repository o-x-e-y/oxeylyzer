#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TrigramPattern {
	Alternate,
	AlternateSfs,
	AlternateThumb,
	AlternateSfsThumb,
	Inroll,
	InrollThumb,
	Outroll,
	OutrollThumb,
	Onehand,
	OnehandThumb,
	Redirect,
	RedirectThumb,
	RedirectSfs,
	RedirectSfsThumb,
	BadRedirect,
	BadRedirectThumb,
	BadRedirectSfs,
	BadRedirectSfsThumb,
	Sfb,
	BadSfb,
	Sft,
	Other,
	Invalid
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
enum Hand {
	Left,
	Right
}

use Hand::*;

impl Hand {
	const fn eq(self, other: Self) -> bool {
		self as u8 == other as u8
	}
}

impl std::ops::Not for Hand {
	type Output = Self;
	
	fn not(self) -> Self::Output {
    	match self {
    		Left => Right,
    		Right => Left
    	}
	}
}

impl From<Finger> for Hand {
    fn from(value: Finger) -> Self {
        value.hand()
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum Finger {
	LP,
	LR,
	LM,
	LI,
	LT,
	RT,
	RI,
	RM,
	RR,
	RP
}

use Finger::*;

impl std::fmt::Display for Finger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    	let to_write = match self {
    		LP => "left pinky",
    		LR => "left ring",
    		LM => "left middle",
    		LI => "left index",
    		RI => "right index",
    		RM => "right middle",
    		RR => "right ring",
    		RP => "right pinky",
    		LT => "left thumb",
    		RT => "right thumb"
    	};
        write!(f, "{}", to_write)
    }
}

impl Finger {
	pub const fn eq(self, other: Self) -> bool {
		self as u8 == other as u8
	}
	
	pub const fn gt(self, other: Self) -> bool {
		self as u8 > other as u8
	}
	
	pub const fn lt(self, other: Self) -> bool {
		(self as u8) < (other as u8)
	}
	
	const fn hand(&self) -> Hand {
		match self {
			LP | LR | LM | LI | LT => Left,
			_ => Right
		}
	}

	const fn is_bad(&self) -> bool {
		match self {
			LP | LR | LM | RM | RR | RP => true,
			_ => false
		}
	}

	const fn is_index(&self) -> bool {
		self.eq(LI) || self.eq(RI)
	}

	const fn is_thumb(&self) -> bool {
		self.eq(LT) || self.eq(RT)
	}
	
	pub const fn from_usize(value: usize) -> Self {
        match value {
        	0 => LP,
        	1 => LR,
        	2 => LM,
        	3 => LI,
			4 => LT,
        	5 => RT,
        	6 => RI,
        	7 => RM,
        	8 => RR,
        	9 => RP,
			_ => unreachable!()
        }
    }
}

#[derive(Debug)]
struct Trigram {
	f1: Finger,
	f2: Finger,
	f3: Finger,
	h1: Hand,
	h2: Hand,
	h3: Hand
}

impl std::fmt::Display for Trigram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, {}, {}", self.f1, self.f2, self.f3)
    }
}

impl Trigram {
	const fn new(f1: Finger, f2: Finger, f3: Finger) -> Self {
		Trigram {
			f1, f2, f3, h1: f1.hand(), h2: f2.hand(), h3: f3.hand()
		}
	}

	const fn has_thumb(&self) -> bool {
		self.f1.is_thumb() || self.f2.is_thumb() || self.f3.is_thumb()
	}

	const fn is_alt(&self) -> bool {
		match (self.h1, self.h2, self.h3) {
			(Left, Right, Left) => true,
			(Right, Left, Right) => true,
			_ => false
		}
	}

	const fn is_sfs(&self) -> bool {
		self.f1.eq(self.f3)
	}

	const fn get_alternate(&self) -> TrigramPattern {
		use TrigramPattern::*;

		match (self.has_thumb(), self.is_sfs()) {
			(false, false) => Alternate,
			(false, true) => AlternateSfs,
			(true, false) => AlternateThumb,
			(true, true) => AlternateSfsThumb
		}
	}

	const fn is_roll(&self) -> bool {
		match (self.h1, self.h2, self.h3) {
			(Left, Left, Right) => true,
			(Right, Left, Left) => true,
			(Right, Right, Left) => true,
			(Left, Right, Right) => true,
			_ => false
		}
	}

	const fn is_inroll(&self) -> bool {
		match (self.h1, self.h2, self.h3) {
			(Left, Left, Right) => self.f1.gt(self.f2),
			(Right, Left, Left) => self.f2.gt(self.f3),
			(Right, Right, Left) => self.f1.lt(self.f2),
			(Left, Right, Right) => self.f2.lt(self.f3),
			_ => unreachable!()
		}
	}

	const fn get_roll(&self) -> TrigramPattern {
		use TrigramPattern::*;

		match (self.has_thumb(), self.is_inroll()) {
			(false, false) => Outroll,
			(false, true) => Inroll,
			(true, false) => OutrollThumb,
			(true, true) => InrollThumb
		}
	}

	const fn on_one_hand(&self) -> bool {
		match (self.h1, self.h2, self.h3) {
			(Left, Left, Left) | (Right, Right, Right) => true,
			_ => false
		}
	}

	const fn is_redir(&self) -> bool {
		self.f1.lt(self.f2) == self.f2.gt(self.f3)
	}

	const fn is_bad_redir(&self) -> bool {
		self.is_redir() && (self.f1.is_bad() || self.f2.is_bad() || self.f3.is_bad())
	}

	const fn has_sfb(&self) -> bool {
		self.f1.eq(self.f2) || self.f2.eq(self.f3)
	}

	const fn is_sft(&self) -> bool {
		self.f1.eq(self.f2) && self.f2.eq(self.f3)
	}

	const fn get_one_hand(&self) -> TrigramPattern {
		use TrigramPattern::*;

		if self.is_sft() {
			Sft
		} else if self.has_sfb() {
			BadSfb
		} else if self.is_redir() {
			match (self.has_thumb(), self.is_sfs(), self.is_bad_redir()) {
				(false, false, false) => Redirect,
				(false, false, true) => BadRedirect,
				(false, true, false) => RedirectSfs,
				(false, true, true) => BadRedirectSfs,
				(true, false, false) => RedirectThumb,
				(true, false, true) => BadRedirectThumb,
				(true, true, false) => RedirectSfsThumb,
				(true, true, true) => BadRedirectSfsThumb,
			}
		} else {
			if self.has_thumb() {
				OnehandThumb
			} else {
				Onehand
			}
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
			unreachable!()
		}
	}
}

const fn get_trigram_combinations() -> [TrigramPattern; 2457] {
	let mut combinations: [TrigramPattern; 2457] = [TrigramPattern::Other; 2457];

	let mut c3 = 0;
	while c3 < 10 {
		let mut c2 = 0;
		while c2 < 10 {
			let mut c1 = 0;
			while c1 < 10 {
				let index = c3*100 + c2*10 + c1;
				let trigram = Trigram::new(
					Finger::from_usize(c1),
					Finger::from_usize(c2),
					Finger::from_usize(c3)
				);
				combinations[index] = trigram.get_trigram_pattern();
				c1 += 1;
			}
			c2 += 1;
		}
		c3 += 1;
	}
	combinations
}

pub static TRIGRAM_COMBINATIONS: [TrigramPattern; 2457] = get_trigram_combinations();

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