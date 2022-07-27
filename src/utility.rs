use itertools::Itertools;
use crate::languages_cfg::read_cfg;

pub static COL_TO_FINGER: [usize; 10] = [0, 1, 2, 3, 3, 4, 4, 5, 6, 7];

#[derive(Copy, Clone, Debug)]
pub struct PosPair(pub usize, pub usize);

impl PosPair {
	pub const fn default() -> Self {
		Self(0, 0)
	}

	pub const fn new(x1: usize, x2: usize) -> Self {
		Self(x1, x2)
	}
}

impl std::fmt::Display for PosPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
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

pub fn get_effort_map(heatmap: f64) -> [f64; 30] {
	let mut res = [
		3.3, 2.3, 1.9, 1.5, 2.3, 3.2, 2.0, 1.8, 2.2, 2.8,
		1.5, 1.1, 0.9, 0.8, 2.4, 2.4, 0.8, 0.9, 1.1, 1.5,
		2.9, 2.5, 2.2, 1.8, 1.8, 1.9, 1.6, 2.2, 2.5, 3.1
	];
	
	for i in 0..30 {
		res[i] /= 4.0;
		res[i] *= heatmap;
	}

	res
}

pub fn get_fspeed(lat_multiplier: f64) -> [(PosPair, f64); 48] {
    let mut res = Vec::new();
    for (b, dist) in get_sfb_indices().iter().zip(get_distances(lat_multiplier)) {
		// println!("{}: {}", b, dist);
        res.push((*b, dist));
    }
    res.try_into().unwrap()
}

fn get_distances(lat_multiplier: f64) -> [f64; 48] {
    let mut res = Vec::new();
    let help = |f: f64, r: f64| f.powi(2).powf(0.65) * r;
    
    for fweight in [1.4, 3.6, 4.8, 4.8, 3.6, 1.4] {
		let ratio = 5.5/fweight;
        res.append(&mut vec![help(1.0, ratio), help(2.0, ratio), help(1.0, ratio)]);
    }

    for _ in 0..2 {
        for c in [
			(0, (0i32, 0)), (1, (0i32, 1)), (2, (0, 2)), (3, (1, 0)), (4, (1, 1)), (5, (1, 2))
		].iter().combinations(2) {
            let (_i1, xy1) = c[0];
            let (_i2, xy2) = c[1];

			// if i == 0 && (*i1 >= 4 && *i2 < 4 || *i2 >= 4 && *i1 < 4) && *i1 + *i2 != 9 {
			// 	res.push(0.0);
			// } else if i == 0 && *i1 + *i2 == 9 {
			// 	res.push(3.5);
			// } else {
				let x_dist = (xy1.0 - xy2.0) as f64;
				let y_dist = (xy1.1 - xy2.1) as f64;
				let distance = (x_dist.powi(2)*lat_multiplier + y_dist.powi(2)).powf(0.65);
				
				res.push(distance);
			// }
        }
    }
    res.try_into().unwrap()
}

pub fn get_sfb_indices() -> [PosPair; 48] {
	let mut res: Vec<PosPair> = Vec::new();
	for i in [0, 1, 2, 7, 8, 9] {
		let chars = [i, i+10, i+20];
		for c in chars.into_iter().combinations(2) {
			res.push(PosPair(c[0], c[1]));
		}
	}
	for i in [0, 2] {
		let chars = [3+i, 13+i, 23+i, 4+i, 14+i, 24+i];
		for c in chars.into_iter().combinations(2) {
			res.push(PosPair(c[0], c[1]));
		}
	}
	res.try_into().unwrap()
}

pub fn get_scissor_indices() -> [PosPair; 15] {
	let mut res: Vec<PosPair> = Vec::new();
	//these two are top pinky to ring homerow
	res.push(PosPair(0, 11));
	res.push(PosPair(9, 18));
	//these four are inner index stretches
	res.push(PosPair(2, 24));
	res.push(PosPair(22, 4));
	res.push(PosPair(5, 27));
	//these add normal stretching between ajacent columns that stretch between 2 rows except for
	//qwerty mi and cr (assuming c is typed with index)
	for i in [0, 1, 2, 6, 7, 8] {
		if i != 2 {
			res.push(PosPair(i, i+21));
		}
		if i != 6 {
			res.push(PosPair(i+1, i+20));
		}
	}
	res.try_into().unwrap()
}

pub fn available_chars(language: &str) -> [char; 30] {
	let languages_cfg_map = read_cfg();

	if let Some(cfg) = languages_cfg_map.get(language) {
		cfg.chars().collect::<Vec<char>>().try_into().unwrap()
	} else {
		let default = languages_cfg_map.get(&String::from("default")).unwrap();
		default.chars().collect::<Vec<char>>().try_into().unwrap()
	}
}