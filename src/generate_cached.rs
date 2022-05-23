use fxhash::FxHashMap;

use crate::analyze::TrigramStats;
use crate::generate::Layout;

pub struct CachedLayout {
	layout: Layout,
	char_stats: FxHashMap<char, PerCharStats>
}

pub struct PerCharStats {
	target: char,
	p_score: f64,
	p_sfb: f64,
	p_dsfb: f64,
	p_trigrams: TrigramStats
}

impl PerCharStats {
	pub fn new(t: char) -> PerCharStats {
		PerCharStats {
			target: t,
			p_score: 0.0,
			p_sfb: 0.0,
			p_dsfb: 0.0,
			p_trigrams: TrigramStats::default()
		}
	}

	pub fn from_layout(layout: Layout) {
		for c in layout.matrix {
			println!("{}", layout.char_to_finger.get(&c).unwrap());
		}
	}
}