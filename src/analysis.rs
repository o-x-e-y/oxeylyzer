pub static COL_TO_FINGER: [u8; 10] = [0, 1, 2, 3, 3, 4, 4, 5, 6, 7];
pub const LAYOUT_GENERATION_COUNT: usize = 1000;

#[derive(Debug, Copy, Clone)]
pub struct Pos {
	pub x: usize,
	pub y: usize
}

#[derive(Debug, Copy, Clone)]
pub struct PosPair(pub Pos, pub Pos);


impl PosPair {
	pub const fn new() -> PosPair {
		PosPair(Pos{x: 0, y: 0}, Pos{x: 0, y: 0})
	}
}

pub static POSSIBLE_SWAPS: [PosPair; 435] = get_possible_swaps();

const fn get_possible_swaps() -> [PosPair; 435] {
	let mut res = [PosPair::new(); 435];
	let mut i = 0;
	let mut pos1 = 0;
	while pos1 < 30 {
		let mut pos2 = pos1 + 1;
		while pos2 < 30 {
			res[i].0.x = pos1 % 10;
			res[i].0.y = pos1 / 10;
			res[i].1.x = pos2 % 10;
			res[i].1.y = pos2 / 10;
			i += 1;
			pos2 += 1;
		}
		pos1 += 1;
	}
	res
}

pub static EFFORT_MAP: [f64; 30] = [
	3.3, 2.5, 2.1, 2.3, 2.6, 3.4, 2.2, 2.0, 2.4, 3.0,
	1.6, 1.3, 1.1, 1.0, 2.9, 2.9, 1.0, 1.1, 1.3, 1.6,
	3.1, 2.7, 2.4, 1.8, 3.7, 2.2, 1.8, 2.4, 2.7, 3.3
];