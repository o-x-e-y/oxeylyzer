use fxhash::FxHashMap;

use crate::language_data::LanguageData;
use crate::analyze::TrigramStats;
use crate::generate::BasicLayout;

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

pub struct AnalyzeCached {
	data: LanguageData,
	layout: CachedLayout,
}