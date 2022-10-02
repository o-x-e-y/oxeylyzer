use crate::language_data::*;
use crate::language_data::LanguageData;
use crate::utility::*;
use crate::weights::{Config, Weights};
use crate::trigram_patterns::*;
use crate::layout::*;
use crate::generate::*;

use anyhow::Result;
use indexmap::IndexMap;

pub struct LayoutAnalysis {
	pub language: String,
	pub layouts: IndexMap<String, FastLayout>,
	pub language_data: LanguageData,
	pub sfb_indices: [PosPair; 48],
	pub scissor_indices: [PosPair; 15],
	pub fspeed_vals: [(PosPair, f64); 48],
	pub effort_map: [f64; 30],
	pub weights: Weights,
	pub i_to_col: [usize; 30],
}

impl LayoutAnalysis {
	pub fn new(language: &str, weights_opt: Option<Weights>) -> Result<LayoutAnalysis> {
		let weights = if weights_opt.is_none() {
			Config::new().weights
		} else {
			weights_opt.unwrap()
		};

		let mut new_analysis = LayoutAnalysis {
			language: String::new(),
			layouts: IndexMap::new(),
			language_data: LanguageData::from_file("static/language_data", language)?,
			sfb_indices: get_sfb_indices(),
			fspeed_vals: get_fspeed(weights.lateral_penalty),
			effort_map: get_effort_map(weights.heatmap),
			scissor_indices: get_scissor_indices(),
			weights,
			i_to_col: [
				0, 1, 2, 3, 3, 4, 4, 5, 6, 7,
				0, 1, 2, 3, 3, 4, 4, 5, 6, 7,
				0, 1, 2, 3, 3, 4, 4, 5, 6, 7
			],
		};

		new_analysis.language = new_analysis.language_data.language.clone();
		
		new_analysis.layouts = IndexMap::new();
		Ok(new_analysis)
	}

	pub fn effort(&self, layout: &FastLayout) -> f64 {
		let mut cols = [0.0; 8];
		let mut res: f64 = 0.0;

		for ((c, w), col) in layout.matrix.iter()
			.zip(self.effort_map)
			.zip(self.i_to_col) {
				let c_freq = self.language_data.characters.get(c).unwrap_or_else(|| &0.0);
				res += c_freq * w;
				cols[col] += c_freq;
		}

		res += (cols[0] - self.weights.max_finger_use.pinky)
			.max(0.0) * self.weights.max_finger_use.penalty;
		res += (cols[1] - self.weights.max_finger_use.ring)
			.max(0.0) * self.weights.max_finger_use.penalty;
		res += (cols[2] - self.weights.max_finger_use.middle)
			.max(0.0) * self.weights.max_finger_use.penalty;
		res += (cols[3] - self.weights.max_finger_use.index)	
			.max(0.0) * self.weights.max_finger_use.penalty;
		
		res += (cols[7] - self.weights.max_finger_use.pinky)
			.max(0.0) * self.weights.max_finger_use.penalty;
		res += (cols[6] - self.weights.max_finger_use.ring)
			.max(0.0) * self.weights.max_finger_use.penalty;
		res += (cols[5] - self.weights.max_finger_use.middle)
			.max(0.0) * self.weights.max_finger_use.penalty;
		res += (cols[4] - self.weights.max_finger_use.index)	
			.max(0.0) * self.weights.max_finger_use.penalty;

		res
	}

	pub fn scissor_percent(&self, layout: &FastLayout) -> f64 {
		let mut res = 0.0;
		for PosPair(i1, i2) in self.scissor_indices {
			let c1 = layout.matrix[i1];
			let c2 = layout.matrix[i2];
			res += self.language_data.bigrams.get(&[c1, c2]).unwrap_or_else(|| &0.0);
			res += self.language_data.bigrams.get(&[c2, c1]).unwrap_or_else(|| &0.0);
		}
		res
	}

	pub fn bigram_percent(&self, layout: &FastLayout, data: &BigramData) -> f64 {
		let mut res = 0.0;
		for PosPair(i1, i2) in self.sfb_indices {
			let c1 = layout.matrix[i1];
			let c2 = layout.matrix[i2];
			res += data.get(&[c1, c2]).unwrap_or_else(|| &0.0);
			res += data.get(&[c2, c1]).unwrap_or_else(|| &0.0);
		}
		res
	}

	pub fn fspeed(&self, layout: &FastLayout) -> f64 {
		let mut res = 0.0;
		let dsfb_ratio = self.weights.dsfb_ratio;
		let dsfb_ratio2 = self.weights.dsfb_ratio2;
		let dsfb_ratio3 = self.weights.dsfb_ratio3;

		for (PosPair(i1, i2), dist) in self.fspeed_vals {
			let c1 = unsafe { layout.cu(i1) };
			let c2 = unsafe { layout.cu(i2) };

			let (pair, rev) = ([c1, c2], [c2, c1]);

			res += self.language_data.bigrams.get(&pair).unwrap_or_else(|| &0.0) * dist;
			res += self.language_data.bigrams.get(&rev).unwrap_or_else(|| &0.0) * dist;

			res += self.language_data.skipgrams.get(&pair).unwrap_or_else(|| &0.0) * dist * dsfb_ratio;
			res += self.language_data.skipgrams.get(&rev).unwrap_or_else(|| &0.0) * dist * dsfb_ratio;

			res += self.language_data.skipgrams2.get(&pair).unwrap_or_else(|| &0.0) * dist * dsfb_ratio2;
			res += self.language_data.skipgrams2.get(&rev).unwrap_or_else(|| &0.0) * dist * dsfb_ratio2;

			res += self.language_data.skipgrams3.get(&pair).unwrap_or_else(|| &0.0) * dist * dsfb_ratio3;
			res += self.language_data.skipgrams3.get(&rev).unwrap_or_else(|| &0.0) * dist * dsfb_ratio3;			
		}

		res
	}

	pub fn finger_speed(&self, layout: &FastLayout) -> [f64; 8] {
		let mut res = [0.0; 8];
		let dsfb_ratio = self.weights.dsfb_ratio;

		let mut fspeed_i = 0;

		for i in [0, 1, 2, 5, 6, 7] {
			for _ in 0..3 {
				let (PosPair(i1, i2), dist) = self.fspeed_vals[fspeed_i];
				let c1 = layout.matrix[i1];
				let c2 = layout.matrix[i2];

				res[i] += self.language_data.bigrams.get(&[c1, c2]).unwrap_or_else(|| &0.0) * dist;
				res[i] += self.language_data.bigrams.get(&[c2, c1]).unwrap_or_else(|| &0.0) * dist;

				res[i] += self.language_data.bigrams.get(&[c1, c2]).unwrap_or_else(|| &0.0) * dist * dsfb_ratio;
				res[i] += self.language_data.bigrams.get(&[c2, c1]).unwrap_or_else(|| &0.0) * dist * dsfb_ratio;
				
				fspeed_i += 1;
			}
		}

		for col in [3, 4] {
			for _ in 0..15 {
				let (PosPair(i1, i2), dist) = self.fspeed_vals[fspeed_i];
				let c1 = layout.matrix[i1];
				let c2 = layout.matrix[i2];

				res[col] += self.language_data.bigrams.get(&[c1, c2]).unwrap_or_else(|| &0.0) * dist;
				res[col] += self.language_data.bigrams.get(&[c2, c1]).unwrap_or_else(|| &0.0) * dist;

				res[col] += self.language_data.bigrams.get(&[c1, c2]).unwrap_or_else(|| &0.0) * dist * dsfb_ratio;
				res[col] += self.language_data.bigrams.get(&[c2, c1]).unwrap_or_else(|| &0.0) * dist * dsfb_ratio;

				fspeed_i += 1;
			}
		}

		res
	}

	pub fn trigram_stats(&self, layout: &FastLayout, trigram_precision: usize) -> TrigramStats {
		let mut freqs = TrigramStats::default();
		for (trigram, freq) in self.language_data.trigrams.iter()
			.take(trigram_precision) {
			match layout.get_trigram_pattern(trigram) {
				TrigramPattern::Alternate => freqs.alternates += freq,
				TrigramPattern::AlternateSfs => freqs.alternates_sfs += freq,
				TrigramPattern::Inroll => freqs.inrolls += freq,
				TrigramPattern::Outroll => freqs.outrolls += freq,
				TrigramPattern::Onehand => freqs.onehands += freq,
				TrigramPattern::Redirect => freqs.redirects += freq,
				TrigramPattern::BadRedirect => freqs.bad_redirects += freq,
				TrigramPattern::Sfb => freqs.sfbs += freq,
				TrigramPattern::BadSfb => freqs.bad_sfbs += freq,
				TrigramPattern::Sft => freqs.sfts += freq,
				TrigramPattern::Other => freqs.other += freq,
				TrigramPattern::Invalid => freqs.invalid += freq
			}
		}
		freqs
	}

	pub fn trigram_score(&self, layout: &FastLayout, trigram_precision: usize) -> f64 {
		let mut score = 0.0;
		let trigram_data = self.trigram_stats(layout, trigram_precision);

		score += self.weights.inrolls * trigram_data.inrolls;
		score += self.weights.outrolls * trigram_data.outrolls;
		score += self.weights.onehands * trigram_data.onehands;
		score += self.weights.alternates * trigram_data.alternates;
		score += self.weights.alternates_sfs * trigram_data.alternates_sfs;
		score -= self.weights.redirects * trigram_data.redirects;
		score -= self.weights.bad_redirects * trigram_data.bad_redirects;

		score
	}

	pub fn score(&self, layout: &FastLayout, trigram_precision: usize) -> f64 {
		let mut score: f64 = 0.0;
		
		let fspeed = self.fspeed(layout);
		let scissors = self.scissor_percent(layout);
		let trigram_score = self.trigram_score(layout, trigram_precision);

		score -= self.effort(layout);
		score -= self.weights.fspeed * fspeed;
		score -= self.weights.scissors * scissors;
		score += trigram_score;

		score
	}
}