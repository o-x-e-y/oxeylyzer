pub static COL_TO_FINGER: [u8; 10] = [0, 1, 2, 3, 3, 4, 4, 5, 6, 7];

#[derive(Copy, Clone)]
pub struct PosPair(pub usize, pub usize);

impl PosPair {
	pub const fn default() -> Self {
		Self(0, 0)
	}

	pub const fn new(x1: usize, x2: usize) -> Self {
		Self(x1, x2)
	}
}

pub static POSSIBLE_SWAPS: [PosPair; 435] = get_possible_swaps();

const fn get_possible_swaps() -> [PosPair; 435] {
	let mut res = [PosPair::default(); 435];
	let mut i = 0;
	let mut pos1 = 0;
	while pos1 < 30 {
		let mut pos2 = pos1 + 1;
		while pos2 < 30 {
			res[i] = PosPair(pos1, pos2);
			i += 1;
			pos2 += 1;
		}
		pos1 += 1;
	}
	res
}

pub static EFFORT_MAP: [f64; 30] = [
	3.5, 2.5, 2.1, 1.7, 2.5, 3.4, 2.1, 2.0, 2.4, 3.0,
	1.7, 1.3, 1.1, 1.0, 2.6, 2.6, 1.0, 1.1, 1.3, 1.7,
	3.1, 2.7, 2.4, 1.8, 3.7, 2.2, 1.8, 2.4, 2.7, 3.3
];