use crate::language_data::TrigramData;

use itertools::Itertools;

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
	3.3, 2.3, 1.9, 1.5, 2.3, 3.2, 2.0, 1.8, 2.2, 2.8,
	1.5, 1.1, 0.9, 0.8, 2.4, 2.4, 0.8, 0.9, 1.1, 1.5,
	2.9, 2.5, 2.2, 1.6, 3.5, 1.9, 1.6, 2.2, 2.5, 3.1
];

pub fn get_sfb_indices() -> [(usize, usize); 48] {
	let mut res: Vec<(usize, usize)> = Vec::new();
	for i in [0, 1, 2, 7, 8, 9] {
		let chars = [i, i+10, i+20];
		for c in chars.into_iter().combinations(2) {
			res.push((c[0], c[1]));
		}
	}
	for i in [0, 2] {
		let chars = [3+i, 13+i, 23+i, 4+i, 14+i, 24+i];
		for c in chars.into_iter().combinations(2) {
			res.push((c[0], c[1]));
		}
	}
	res.try_into().unwrap()
}

pub fn get_scissor_indices() -> [(usize, usize); 18] {
	let mut res: Vec<(usize, usize)> = Vec::new();
	//these four are inner index stretches
	res.push((0, 11));
	res.push((9, 18));
	res.push((2, 24));
	res.push((22, 4));
	//these two are top pinky to middle homerow
	res.push((5, 27));
	res.push((25, 7));
	//these add normal stretching between ajacent columns that stretch between 2 rows
	for i in [0, 1, 2, 6, 7, 8] {
		res.push((i, i+21));
		res.push((i+1, i+20));
	}
	res.try_into().unwrap()
}

pub fn available_chars(language: &str) -> [char; 30] {
	let chars = match language {
		"albanian" =>             "abcdefghijklmnopqrstuvëxyz',.ç",
		"bokmal" | "nynorsk" =>   "abcdefghijklmnopærstuvwøyå',.;",
		"czech" =>                "abcdefghijklmnop*rstuvěxyzá,.í",
		"dutch_repeat" =>         "abcdefghijklmnopqrstuvwxyz',.@",
		"english_repeat" =>       "abcdefghijklmnopqrstuvwxyz',.@",
		"esperanto" =>            "abcdefghijklmnopŝrstuvĝĉŭz',.ĵ",
		"english_th" =>           "abcdefghijklmnopqrstuvwxyz',.þ",
		"finnish" =>              "abcdefghijklmnopärstuvwxyz',.ö",
		"finnish_repeat" =>       "abcdefghijklmnopärstuvw@yz',.ö",
		"french" | "french_qu" => "abcdefghijélmnopqrstuvàxy-',.*",
		"german" =>               "abcdefghijklmnoprstuvwxyzüäö.,",
		"italian" =>              "abcdefghijklmnopqrstuvwxyz',.*",
		"russian" =>              "абвгдежзийклмнопрстухцчшыьюя.,",
		"spanish" =>              "abcdefghijklmnopqrstuvwxyz',.*",
		_ =>                      "abcdefghijklmnopqrstuvwxyz',.;"
	};
	chars.chars().collect::<Vec<char>>().try_into().unwrap()
}