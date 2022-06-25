use fxhash::FxHashMap;
use anyhow::Result;
use rayon::prelude::*;

use crate::language_data::*;
use crate::analyze::{TrigramStats, LayoutAnalysis, Config};
use crate::generate::{Layout, BasicLayout};
use crate::analysis::*;

pub struct PerCharStats {
	target: char,
	p_score: f64,
	p_sfb: f64,
	p_dsfb: f64,
	p_trigrams: TrigramStats
}

impl PerCharStats {
	pub fn new(target: char) -> Self {
		Self {
			target,
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
	char_score: [f64; 30]
}

impl From<BasicLayout> for CachedLayout {
    fn from(l: BasicLayout) -> Self {
		Self {
			layout: l,
			char_score: [0.0; 30]
		}
    }
}

// pub available_chars: [char; 30],
// pub analysis: LayoutAnalysis,
// pub improved_layout: BasicLayout,
// pub temp_generated: Option<Vec<String>>,
// cols: [usize; 6],

type CharTrigrams = FxHashMap<char, TrigramData>;

fn per_char_trigrams(n_trigrams: TrigramData, available_chars: &[char; 30]) -> CharTrigrams {
	let mut thingy = Vec::new();
	available_chars
		.par_iter()
		.map(|c| {
			let per_char = n_trigrams
				.iter()
				.map(|(t, f)| (t.clone(), f.clone()))
				.filter(|(t, _)| t.contains(c))
				.collect::<Vec<([char; 3], f64)>>();
			(*c, per_char)
		})
		.collect_into_vec(&mut thingy);
	CharTrigrams::from_iter(thingy)
}

pub struct GenerateCached {
	available_chars: [char; 30],
	analysis: LayoutAnalysis,
	layout: CachedLayout,
	pub char_trigrams: CharTrigrams
}

impl GenerateCached {
	pub fn new(language: &str, trigram_precision: usize) -> Result<Self> {
		let available = available_chars(language);
		let new_config = Config::new();
		let analyzer = LayoutAnalysis::new(language, Some(new_config.weights))?;
		let n_trigrams = analyzer.language_data.trigrams.clone()
			.into_iter()
			.take(trigram_precision)
			.collect::<TrigramData>();

		Ok(
			Self {
				available_chars: available,
				char_trigrams: per_char_trigrams(n_trigrams, &available),
				analysis: analyzer,
				layout: CachedLayout::from(BasicLayout::random(available)),
			}
		)
	}
}