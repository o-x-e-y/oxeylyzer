#[derive(Copy, Clone, PartialEq, Debug)]
pub enum TrigramPattern {
	Alternate,
	AlternateSfs,
	Inroll,
	Outroll,
	Onehand,
	Redirect,
	BadRedirect,
	Other,
	Invalid
}

const fn lh(num: usize) -> bool {
	num < 4
}

const fn is_alt(lh1: bool, lh2: bool, lh3: bool) -> bool {
	lh1 != lh2 && lh2 != lh3
}

const fn is_roll(lh1: bool, lh2: bool, lh3: bool, c1: usize, c2: usize, c3: usize) -> bool {
	let r = lh1 as usize + lh2 as usize + lh3 as usize;
	!is_alt(lh1, lh2, lh3) && (r == 1 || r == 2) && c1 != c2 && c2 != c3
}

const fn get_roll(lh1: bool, lh2: bool, lh3: bool, c1: usize, c2: usize, c3: usize) -> TrigramPattern {
	if lh1 && lh2 && !lh3 {
		particular_roll(c2, c1)
	} else if !lh1 && lh2 && lh3 {
		particular_roll(c3, c2)
	} else if !lh1 && !lh2 && lh3 {
		particular_roll(c1, c2)
	} else if lh1 && !lh2 && !lh3 {
		particular_roll(c2, c3)
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

const fn on_one_hand(lh1: bool, lh2: bool, lh3: bool) -> bool {
	lh1 == lh2 && lh2 == lh3
}

const fn is_bad_redir(c1: usize, c2: usize, c3: usize) -> bool {
	!(c1 == 3 || c2 == 3 || c3 == 3 || c1 == 4 || c2 == 4 || c3 == 4)
}

const fn get_one_hand(c1: usize, c2: usize, c3: usize) -> TrigramPattern {
	if (c1 < c2 && c2 > c3) || (c1 > c2 && c2 < c3) {
		if is_bad_redir(c1, c2, c3) {
			return TrigramPattern::BadRedirect
		}
		return TrigramPattern::Redirect
	}
	if (c1 > c2 && c2 > c3) || (c1 < c2 && c2 < c3) {
		return TrigramPattern::Onehand
	}
	TrigramPattern::Other
}

const fn get_trigram_pattern(c1: usize, c2: usize, c3: usize) -> TrigramPattern {
	let lh1 = lh(c1);
	let lh2 = lh(c2);
	let lh3 = lh(c3);

	if is_alt(lh1, lh2, lh3) {
		if c1 == c3 {
			TrigramPattern::AlternateSfs
		} else {
			TrigramPattern::Alternate
		}
	} else if on_one_hand(lh1, lh2, lh3) {
		get_one_hand(c1, c2, c3)
	} else if is_roll(lh1, lh2, lh3, c1, c2, c3) {
		get_roll(lh1, lh2, lh3, c1, c2, c3)
	} else {
		TrigramPattern::Other
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
				combinations[index] = get_trigram_pattern(c1, c2, c3);
				c1 += 1;
			}
			c2 += 1;
		}
		c3 += 1;
	}
	combinations
}

pub static TRIGRAM_COMBINATIONS: [TrigramPattern; 512] = get_trigram_combinations();