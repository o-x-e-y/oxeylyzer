use fxhash::{FxHashMap, FxHashSet};

use crate::language_data::LanguageData;
use crate::analyze::TrigramStats;
use crate::generate::{Layout, BasicLayout};

pub struct PerCharStats {
	target: char,
	p_score: f64,
	p_sfb: f64,
	p_dsfb: f64,
	p_trigrams: TrigramStats
}

impl PerCharStats {
	pub fn new(t: char) -> Self {
		Self {
			target: t,
			p_score: 0.0,
			p_sfb: 0.0,
			p_dsfb: 0.0,
			p_trigrams: TrigramStats::default()
		}
	}
}

#[derive(Default)]
pub struct CachedLayout {
	layout: BasicLayout,
	char_stats: FxHashMap<char, PerCharStats>
}

impl From<BasicLayout> for CachedLayout {
    fn from(l: BasicLayout) -> Self {
		for c in l.matrix {
			println!("{}", l.char_to_finger.get(&c).unwrap());
		}
		Self::default()
    }
}

// pub available_chars: [char; 30],
// pub analysis: LayoutAnalysis,
// pub improved_layout: BasicLayout,
// pub temp_generated: Option<Vec<String>>,
// cols: [usize; 6],

type CharTrigrams = FxHashMap<char, Vec<[char; 3]>>;

pub struct GenerateCached {
	available_chars: [char; 30],
	data: LanguageData,
	layout: CachedLayout,
	per_char_trigrams: CharTrigrams
}

impl GenerateCached {
	pub fn new(language: &str) -> Self {
		let available = crate::analysis::available_chars(language);
		let data = LanguageData::new(language);
		Self {
			available_chars: available,
			per_char_trigrams: Self::per_char_trigrams(&data, &available),
			data: data,
			layout: CachedLayout::from(BasicLayout::random(available)),
			
		}
	}

	fn per_char_trigrams(data: &LanguageData, available_chars: &[char; 30]) -> CharTrigrams {
		let chars = FxHashSet::from_iter(available_chars);
		let has = |tri: &&[char; 3]| -> bool {
			for c in tri.into_iter() {
				if chars.contains(c) {
					return true;
				}
			}
			false
		};

		let mut res: CharTrigrams = FxHashMap::default();
		data.trigrams
			.iter()
			.take(25)
			.map(|(tri, _)| tri)
			.filter(|tri| has(tri))
			.for_each(|tri| {
				println!("{:?}", tri);
				// for c in tri {
				// 	if let Some(entry) = res.get_mut(c) {
				// 		entry.push(*tri);
				// 	} else {
				// 		res.insert(*c, vec![*tri]);
				// 	}
				// }
			});
		println!("{:#?}", res);
		res
	}
}