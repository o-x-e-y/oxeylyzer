use std::io::Write;

use crate::language_data::*;
use crate::language_data::LanguageData;
use crate::utility::*;
use crate::weights::{Config, Weights};
use crate::trigram_patterns::*;
use crate::layout::*;
use crate::generate::*;

use anyhow::{Result, bail};
use indexmap::IndexMap;
use ansi_rgb::{rgb, Colorable};

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
		
		new_analysis.layouts = new_analysis.load_layouts()?;
		Ok(new_analysis)
	}

	fn is_kb_file(entry: &std::fs::DirEntry) -> bool {
		if let Some(ext_os) = entry.path().extension() {
			if let Some(ext) = ext_os.to_str() {
				return ext == "kb"
			}
		}
		false
	}

	fn layout_name(entry: &std::fs::DirEntry) -> Option<String> {
		if let Some(name_os) = entry.path().file_stem() {
			if let Some(name_str) = name_os.to_str() {
				return Some(name_str.to_string())
			}
		}
		None
	}

	fn format_layout_str(layout_str: String) -> String {
		layout_str
			.split("\n")
			.take(3)
			.map(|line| {
				line.split_whitespace()
					.take(10)
					.collect::<String>()
			})
			.collect::<String>()
	}

	fn save_layout_stats<P>(&self, layout: &FastLayout, base_path: P, name: &str)
		where P: AsRef<std::path::Path> {
		let stats = self.get_layout_stats(layout);

		let mut f = std::fs::OpenOptions::new()
			.write(true)
			.create(true)
			.truncate(true)
			.open(format!("static/stats/{}/{}.txt", self.language, name)).expect(
				format!("panic on trying to save {name}").as_str()
			);
		
		f.write(stats.to_string().as_bytes()).unwrap();
	}

	fn load_layouts(&mut self) -> Result<IndexMap<String, FastLayout>> {
		let mut res: IndexMap<String, FastLayout> = IndexMap::new();

		if let Ok(paths) = std::fs::read_dir(format!("static/layouts/{}", self.language)) {
			for p in paths {
				if let Ok(entry) = p &&
				Self::is_kb_file(&entry) &&
				let Some(name) = Self::layout_name(&entry) {
					let content = std::fs::read_to_string(entry.path())?;
					let layout_str = Self::format_layout_str(content);

					if let Ok(mut layout) = FastLayout::try_from(layout_str.as_str()) {
						// self.save_layout_stats(&layout, name.as_str());
						layout.score = self.score(&layout, usize::MAX);
						res.insert(name, layout);
					} else {
						println!("layout {} is not formatted correctly", name);
					}
				}
			}
			res.sort_by(|_, a, _, b| {
				a.score.partial_cmp(&b.score).unwrap()
			});
		} else {
			std::fs::create_dir(format!("static/layouts/{}", self.language))?;
		}
		Ok(res)
	}

	fn get_layout_stats(&self, layout: &FastLayout) -> LayoutStats {
		let sfb = self.bigram_percent(layout, &self.language_data.bigrams);
		let dsfb = self.bigram_percent(layout, &self.language_data.skipgrams);
		let fspeed = self.fspeed(layout);
		let finger_speed = self.finger_speed(layout);
		let scissors = self.scissor_percent(layout);
		let trigram_stats = self.trigram_stats(layout, usize::MAX);
		
		LayoutStats { sfb, dsfb, fspeed, finger_speed, scissors, trigram_stats }
	}

	pub fn rank(&self) {
		for (name, layout) in self.layouts.iter() {
			println!("{:10}{}", format!("{:.3}:", layout.score), name);
		}
	}

	pub fn layout_by_name(&self, name: &str) -> Option<&FastLayout> {
		self.layouts.get(name)
	}

	pub fn analyze_name(&self, name: &str) {
		let l = match self.layout_by_name(name) {
  			Some(layout) => layout,
  			None => {
    			println!("layout {} does not exist!", name);
    			return;
  			}
		};
		println!("{}", name);
		self.analyze(&l);
	}

	fn placeholder_name(&self, layout: &FastLayout) -> Result<String, ()> {
		for i in 1..1000usize {
    		let mut new_name = layout.matrix[10..14].iter().collect::<String>();
			
			new_name.push_str(format!("{}", i).as_str());

			if !self.layouts.contains_key(&new_name) {
				return Ok(new_name);
			}
		}
		Err(())
	}

	pub fn analyze_str(&mut self, layout_str: &str) {
		let layout_str = Self::format_layout_str(layout_str.to_string());
		let layout = FastLayout::try_from(layout_str.as_str()).unwrap();
		self.analyze(&layout);
	}

	pub fn save(&mut self, mut layout: FastLayout, name: Option<String>) -> Result<()> {
		let new_name = if let Some(n) = name {
			n.replace(" ", "_")
		} else {
			self.placeholder_name(&layout).unwrap()
		};

		let mut f = std::fs::OpenOptions::new()
			.write(true)
			.create(true)
			.truncate(true)
			.open(format!("static/layouts/{}/{}.kb", self.language, new_name))?;
		
		let layout_formatted = layout.to_string();
		println!("saved {}\n{}", new_name, layout_formatted);
		f.write(layout_formatted.as_bytes()).unwrap();

		layout.score = self.score(&layout, usize::MAX);
		self.layouts.insert(new_name, layout);
		self.layouts.sort_by(|_, a, _, b| {
			a.score.partial_cmp(&b.score).unwrap()
		});

		Ok(())
	}

	pub fn analyze(&self, layout: &FastLayout) {
		let stats = self.get_layout_stats(layout);
		let score = if layout.score == 0.000 {
			self.score(layout, usize::MAX)
		} else {
			layout.score
		};

		let layout_str = layout.to_string();
		
		println!("{}\n{}\nScore: {:.3}", layout_str, stats, score);
	}

	pub fn compare_name(&self, name1: &str, name2: &str) {
		let l1 = match self.layout_by_name(name1) {
  			Some(layout) => layout,
  			None => {
    			println!("layout {} does not exist!", name1);
    			return;
  			}
		};
		let l2 = match self.layout_by_name(name2) {
  			Some(layout) => layout,
  			None => {
    			println!("layout {} does not exist!", name2);
    			return;
  			}
		};
		println!("\n{:29}{}", name1, name2);
		for y in 0..3 {
			for (n, layout) in [l1, l2].into_iter().enumerate() {
				for x in 0..10 {
					print!("{} ", layout.matrix[x + 10*y]);
					if x == 4 {
						print!(" ");
					}
				}
				if n == 0 {
					print!("        ");
				}
			}
			println!();
		}
		let s1 = self.get_layout_stats(l1);
		let s2 = self.get_layout_stats(l2);
		let ts1 = s1.trigram_stats;
		let ts2 = s2.trigram_stats;
		println!(
			concat!(
			"Sfb:              {: <10} Sfb:              {:.3}%\n",
			"Dsfb:             {: <10} Dsfb:             {:.3}%\n",
			"Finger Speed:     {: <10} Finger Speed:     {:.3}\n",
			"Scissors          {: <10} Scissors:         {:.3}%\n\n",
			"Inrolls:          {: <10} Inrolls:          {:.2}%\n",
			"Outrolls:         {: <10} Outrolls:         {:.2}%\n",
			"Total Rolls:      {: <10} Total Rolls:      {:.2}%\n",
			"Onehands:         {: <10} Onehands:         {:.3}%\n\n",
			"Alternates:       {: <10} Alternates:       {:.2}%\n",
			"Alternates (sfs): {: <10} Alternates (sfs): {:.2}%\n",
			"Total Alternates: {: <10} Total Alternates: {:.2}%\n\n",
			"Redirects:        {: <10} Redirects:        {:.2}%\n",
			"Bad Redirects:    {: <10} Bad Redirects:    {:.2}%\n",
			"Total Redirects:  {: <10} Total Redirects:  {:.2}%\n\n",
			"Bad Sfbs:         {: <10} Bad Sfbs:         {:.2}%\n",
			"Sft:              {: <10} Sft:              {:.3}%\n\n",
			"Score:            {: <10} Score:            {:.3}\n"
		),
			format!("{:.3}%", s1.sfb*100.0), s2.sfb*100.0,
			format!("{:.3}%", s1.dsfb*100.0), s2.dsfb*100.0,
			format!("{:.3}%", s1.fspeed * 100.0), s2.fspeed * 100.0,
			format!("{:.3}", s1.scissors*100.0), s2.scissors*100.0,
			format!("{:.2}%", ts1.inrolls*100.0), ts2.inrolls*100.0,
			format!("{:.2}%", ts1.outrolls*100.0), ts2.outrolls*100.0,
			format!("{:.2}%", (ts1.inrolls + ts1.outrolls)*100.0), (ts2.inrolls + ts2.outrolls)*100.0,
			format!("{:.3}%", ts1.onehands*100.0), ts2.onehands*100.0,
			format!("{:.2}%", ts1.alternates*100.0), ts2.alternates*100.0,
			format!("{:.2}%", ts1.alternates_sfs*100.0), ts2.alternates_sfs*100.0,
			format!("{:.2}%", (ts1.alternates + ts1.alternates_sfs)*100.0), (ts2.alternates + ts2.alternates_sfs)*100.0,
			format!("{:.3}%", ts1.redirects*100.0), ts2.redirects*100.0,
			format!("{:.3}%", ts1.bad_redirects*100.0), ts2.bad_redirects*100.0,
			format!("{:.3}%", (ts1.redirects + ts1.bad_redirects)*100.0), (ts2.redirects + ts2.bad_redirects)*100.0,
			format!("{:.3}%", ts1.bad_sfbs*100.0), ts2.bad_sfbs*100.0,
			format!("{:.3}%", ts1.sfts*100.0), ts2.sfts*100.0,
			format!("{:.3}", l1.score), l2.score
		);
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