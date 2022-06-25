use std::io::{Write, Read};
use std::fs::File;

use crate::language_data::*;
use crate::language_data::LanguageData;
use crate::analysis::*;
use crate::trigram_patterns::*;
use crate::generate::{Layout, BasicLayout};

use anyhow::Result;
use indexmap::IndexMap;
use serde::Deserialize;
use ansi_rgb::{rgb, Colorable};

#[derive(Clone, Default)]
pub struct TrigramStats {
	pub alternates: f64,
	pub alternates_sfs: f64,
	pub inrolls: f64,
	pub outrolls: f64,
	pub onehands: f64,
	pub redirects: f64,
	pub bad_redirects: f64,
	pub other: f64,
	pub invalid: f64
}

impl std::fmt::Display for TrigramStats {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
"Inrolls: {:.3}%
Outrolls: {:.3}% 
Total Rolls: {:.3}%
Onehands: {:.3}%\n
Alternates: {:.3}%
Alternates (sfs): {:.3}%
Total Alternates: {:.3}%\n
Redirects: {:.3}%
Bad Redirects: {:.3}%
Total Redirects: {:.3}%",
			self.inrolls*100.0,
			self.outrolls*100.0,
			(self.inrolls + self.outrolls)*100.0,
			self.onehands*100.0,
			self.alternates*100.0,
			self.alternates_sfs*100.0,
			(self.alternates + self.alternates_sfs)*100.0,
			self.redirects*100.0,
			self.bad_redirects*100.0,
			(self.redirects + self.bad_redirects)*100.0
		)
	}
}

impl std::fmt::Debug for TrigramStats {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"Inrolls: {:.3}%\n
			Outrolls: {:.3}%\n
			Total Rolls: {:.3}%\n
			Onehands: {:.3}%\n\n\
			Alternates: {:.3}%\n
			Alternates (sfs): {:.3}%\n
			Total Alternates: {:.3}%\n\n
			Redirects: {:.3}%\n\
			Bad Redirects: {:.3}%\n
			Total Redirects: {:.3}%\n\n
			Other: {:.3}%\n
			Invalid: {:.3}%",
			self.inrolls*100.0,
			self.outrolls*100.0,
			(self.inrolls + self.outrolls)*100.0,
			self.onehands*100.0,
			self.alternates*100.0,
			self.alternates_sfs*100.0,
			(self.alternates + self.alternates_sfs)*100.0,
			self.redirects*100.0,
			self.bad_redirects*100.0,
			(self.redirects + self.bad_redirects)*100.0,
			self.other*100.0,
			self.invalid*100.0
		)
	}
}

#[derive(Clone)]
struct LayoutStats {
	sfb: f64,
	dsfb: f64,
	scissors: f64,
	trigram_stats: TrigramStats,
	// finger_speed: [f64; 8]
}

impl std::fmt::Display for LayoutStats {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		// const BASE: String = String::new();
		// let mut fspeed: [String; 4] = [BASE; 4];
		// for i in 0..4 {
		// 	fspeed[i] = format!("{:.1} {:.1}", self.finger_speed[i], self.finger_speed[7-i]);
		// }
		// let fspeed_print = fspeed.join("\n");
		write!(
			f, "Sfb:  {:.3}%\nDsfb: {:.3}%\nScissors: {:.3}%\n\n{}",
			self.sfb * 100.0, self.dsfb * 100.0, self.scissors * 100.0, self.trigram_stats
		)
	}
}

#[derive(Deserialize)]
pub struct Defaults {
	pub language: String,
	trigram_precision: usize
}

#[derive(Deserialize, Clone)]
pub struct Weights {
	pub heatmap: f64,
	pub lateral_penalty: f64,
	pub sfb: f64,
	pub dsfb: f64,
	pub scissors: f64,
	pub inrolls: f64,
	pub outrolls: f64,
	pub onehands: f64,
	pub alternates: f64,
	pub alternates_sfs: f64,
	pub redirects: f64,
	pub bad_redirects: f64,
	pub max_finger_use: MaxFingerUse
}

#[derive(Deserialize, Clone)]
pub struct MaxFingerUse {
	pub penalty: f64,
	pub pinky: f64,
	pub ring: f64,
	pub middle: f64,
	pub index: f64
}

#[derive(Deserialize)]
struct ConfigLoad {
	pub pins: String,
	pub defaults: Defaults,
	pub weights: Weights
}

impl ConfigLoad {
	pub fn new() -> Self {
		let mut f = File::open("config.toml")
			.expect("The config.toml is missing! Help!");

		let mut buf = Vec::new();
		f.read_to_end(&mut buf)
			.expect("Failed to read config.toml for some reason");

		let mut res: Self = toml::from_slice(&buf)
			.expect("Failed to parse config.toml. Values might be missing.");
		res.pins = res.pins.trim().replace(' ', "").replace('\n', "");
		res
	}
}

pub struct Config {
	pub pins: Vec<usize>,
	pub defaults: Defaults,
	pub weights: Weights
}

impl Config {
	pub fn new() -> Self {
		let mut load = ConfigLoad::new();
		load.weights.max_finger_use = MaxFingerUse {
			penalty: load.weights.max_finger_use.penalty,
			pinky: load.weights.max_finger_use.pinky / 100.0,
			ring: load.weights.max_finger_use.ring / 100.0,
			middle: load.weights.max_finger_use.middle / 100.0,
			index: load.weights.max_finger_use.index / 100.0,
		};
		let mut pins = Vec::new();
		for (i, c) in load.pins.chars().enumerate() {
			if c == 'x' {
				pins.push(i);
			}
		}
		Self {
			pins,
			defaults: load.defaults,
			weights: load.weights
		}
	}

	pub fn default() -> Self {
		Self {
			defaults: Defaults {
				language: "english".to_string(),
				trigram_precision: 1000
			},
			weights: Weights {
				heatmap: 0.85,
				lateral_penalty: 1.3,
				sfb: 15.0,
				dsfb: 2.5,
				scissors: 5.0,
				inrolls: 1.6,
				outrolls: 1.3,
				onehands: 0.8,
				alternates: 0.7,
				alternates_sfs: 0.35,
				redirects: 1.5,
				bad_redirects: 6.5,
				max_finger_use: MaxFingerUse {
					penalty: 2.5,
					pinky: 9.0,
					ring: 16.0,
					middle: 19.5,
					index: 18.0
				}
			},
			pins: Vec::new(),
		}
	}

	pub fn trigram_precision(&self) -> usize {
		self.defaults.trigram_precision
	}
}

pub struct LayoutAnalysis {
	language: String,
	layouts: IndexMap<String, BasicLayout>,
	pub language_data: LanguageData,
	sfb_indices: [(usize, usize); 48],
	scissor_indices: [(usize, usize); 16],
	weights: Weights,
	i_to_col: [usize; 30],
	col_distance: [f64; 6],
	index_distance: [f64; 30]
}

impl LayoutAnalysis {
	pub fn new(language: &str, weights_opt: Option<Weights>) -> Result<LayoutAnalysis> { 
		let weights = if weights_opt.is_none() {
			crate::analyze::Config::new().weights
		} else {
			weights_opt.unwrap()
		};

		let mut new_analysis = LayoutAnalysis {
			language: String::new(),
			layouts: IndexMap::new(),
			language_data: LanguageData::new(language)?,
			sfb_indices: get_sfb_indices(),
			scissor_indices: get_scissor_indices(),
			col_distance: [1.0, 2.0, 1.0, 1.0, 2.0, 1.0],
			index_distance: get_index_distance(weights.lateral_penalty),
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
			.replace("\n", "")
			.replace("\r", "")
			.replace(" ", "")
	}

	fn load_layouts(&mut self) -> Result<IndexMap<String, BasicLayout>> {
		let mut res: IndexMap<String, BasicLayout> = IndexMap::new();

		if let Ok(paths) = std::fs::read_dir(format!("static/layouts/{}", self.language)) {
			for p in paths {
				if let Ok(entry) = p &&
				Self::is_kb_file(&entry) &&
				let Some(name) = Self::layout_name(&entry) {
					let content = std::fs::read_to_string(entry.path())?;
					let layout_str = Self::format_layout_str(content);

					if let Ok(mut layout) = BasicLayout::try_from(layout_str.as_str()) {
						layout.score = self.score(&layout, usize::MAX);
						res.insert(name, layout);
					} else {
						println!("layout {} is not formatted correctly", name);
					}
				}
			}
			res.sort_by(|_, a, _, b| {
				b.score.partial_cmp(&a.score).unwrap()
			});
		} else {
			std::fs::create_dir(format!("static/layouts/{}", self.language))?;
		}
		Ok(res)
	}

	fn get_layout_stats(&self, layout: &BasicLayout) -> LayoutStats {
		let sfb = self.bigram_percent(layout, &self.language_data.bigrams);
		let dsfb = self.bigram_percent(layout, &self.language_data.skipgrams);
		let scissors = self.scissor_percent(layout);
		let trigram_stats = self.trigram_stats(layout, usize::MAX);
		LayoutStats { sfb, dsfb, scissors, trigram_stats }
	}

	pub fn rank(&self) {
		for (name, layout) in self.layouts.iter() {
			println!("{:10}{}", format!("{:.3}:", layout.score), name);
		}
	}

	pub fn layout_by_name(&self, name: &str) -> Option<&BasicLayout> {
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

	fn placeholder_name(&self, layout: &BasicLayout) -> Result<String, ()> {
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
		let layout = BasicLayout::try_from(layout_str.as_str()).unwrap();
		self.analyze(&layout);
	}

	pub fn save(&mut self, mut layout: BasicLayout, name: Option<String>) -> Result<()> {
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
			b.score.partial_cmp(&a.score).unwrap()
		});

		Ok(())
	}

	fn heatmap_heat(&self, c: &char) -> String {
		let complement = 215.0 - *self.language_data.characters
			.get(c)
			.unwrap_or(&0.0) * 1720.0;
		let complement = complement.max(0.0) as u8;
		let heat = rgb(215, complement, complement);
		format!("{}", (*c).to_string().fg(heat))
	}

	pub fn analyze(&self, layout: &BasicLayout) {
		let stats = self.get_layout_stats(layout);
		let score = if layout.score == 0.000 {
			self.score(layout, usize::MAX)
		} else {
			layout.score
		};

		let mut layout_str = String::new();
		for (i, c) in layout.matrix.iter().enumerate() {
			if i % 10 == 0 && i > 0 {
				layout_str.push('\n');
			}
			if (i + 5) % 10 == 0 {
				layout_str.push(' ');
			}
			layout_str.push_str(self.heatmap_heat(c).as_str());
			layout_str.push(' ');
		}
		
		println!("{}\n{}\nScore: {:.3}", layout_str, stats, score);
		// let x = get_trigram_combinations2();
		// for (i, combination) in x.iter().enumerate() {
		// 	let c1 = i & 0b111;
		// 	let c2 = (i >> 3) & 0b111;
		// 	let c3 = (i >> 6) & 0b111;
		// 	if *combination == TrigramPattern::Other && c1 != c2 && c2 != c3 {
		// 		println!("{:<27} where c1: {}, c2: {}, c3: {}", format!("patterns[{}]: {:?}", i, combination), c1, c2, c3);
		// 	}
		// }
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
					print!("{} ", self.heatmap_heat(&layout.matrix[x + 10*y]));
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
		// let fs1 = &layouts[0].stats.finger_speed;
		// let fs2 = &layouts[1].stats.finger_speed;
		// const BASE: String = String::new();
		// let mut fspeed: [String; 4] = [BASE; 4];
		// for i in 0..4 {
		// 	fspeed[i] = format!("{:<28} {:.1}, {:.1}",
		// 						format!("{:.1} {:.1}", fs1[i], fs2[7-i]), fs2[i], fs2[7-i]);
		// }
		// let fspeed_print = fspeed.join("\n");
		println!(
			concat!(
			"Sfb:              {:.3}%     Sfb:              {:.3}%\n",
			"Dsfb:             {:.3}%     Dsfb:             {:.3}%\n",
			"Scissors          {:.3}%     Scissors:         {:.3}%\n\n",
			"Inrolls:          {:.2}%     Inrolls:          {:.2}%\n",
			"Outrolls:         {:.2}%     Outrolls:         {:.2}%\n",
			"Total Rolls:      {:.2}%     Total Rolls:      {:.2}%\n",
			"Onehands:         {:.3}%     Onehands:         {:.3}%\n\n",
			"Alternates:       {:.2}%     Alternates:       {:.2}%\n",
			"Alternates (sfs): {:.2}%      Alternates (sfs): {:.2}%\n",
			"Total Alternates: {:.2}%     Total Alternates: {:.2}%\n\n",
			"Redirects:        {:.3}%     Redirects:        {:.2}%\n",
			"Bad Redirects:    {:.3}%     Bad Redirects:    {:.2}%\n",
			"Total Redirects:  {:.3}%     Total Redirects:  {:.2}%\n\n",
			// "Other:            {:.3}%     Other:            {:.2}%\n",
			// "Invalid:          {:.3}%     Invalid:          {:.2}%\n\n",
			//"{}\n\n",
			"Score:            {:.3}     Score:            {:.3}\n"
		),
			s1.sfb*100.0, s2.sfb*100.0,
			s1.dsfb*100.0, s2.dsfb*100.0,
			s1.scissors*100.0, s2.scissors*100.0,
			ts1.inrolls*100.0, ts2.inrolls*100.0,
			ts1.outrolls*100.0, ts2.outrolls*100.0,
			(ts1.inrolls + ts1.outrolls)*100.0, (ts2.inrolls + ts2.outrolls)*100.0,
			ts1.onehands*100.0, ts2.onehands*100.0,
			ts1.alternates*100.0, ts2.alternates*100.0,
			ts1.alternates_sfs*100.0, ts2.alternates_sfs*100.0,
			(ts1.alternates + ts1.alternates_sfs)*100.0, (ts2.alternates + ts2.alternates_sfs)*100.0,
			ts1.redirects*100.0, ts2.redirects*100.0,
			ts1.bad_redirects*100.0, ts2.bad_redirects*100.0,
			(ts1.redirects + ts1.bad_redirects)*100.0, (ts2.redirects + ts2.bad_redirects)*100.0,
			// ts1.other*100.0, ts2.other*100.0,
			// ts1.invalid*100.0, ts2.invalid*100.0,
			//fspeed_print,
			l1.score, l2.score
		);
	}

	pub fn finger_speed(&self, _: &BasicLayout) -> [f64; 8] {
		let res = [0.0; 8];
		res
	}

	pub fn effort(&self, layout: &BasicLayout) -> f64 {
		let mut cols = [0.0; 8];
		let mut res: f64 = 0.0;
		for ((c, e), col) in layout.matrix.iter().zip(EFFORT_MAP).zip(self.i_to_col) {
			let c_freq = self.language_data.characters.get(c).unwrap_or(&0.0);
			res += e * c_freq;
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

	pub fn scissor_percent(&self, layout: &BasicLayout) -> f64 {
		let mut res = 0.0;
		for (i1, i2) in self.scissor_indices {
			let c1 = layout.matrix[i1];
			let c2 = layout.matrix[i2];
			res += self.language_data.bigrams.get(&[c1, c2]).unwrap_or(&0.0);
			res += self.language_data.bigrams.get(&[c2, c1]).unwrap_or(&0.0);
		}
		res
	}

	pub fn bigram_percent(&self, layout: &BasicLayout, data: &BigramData) -> f64 {
		let mut res = 0.0;
		for (i1, i2) in self.sfb_indices {
			let c1 = layout.matrix[i1];
			let c2 = layout.matrix[i2];
			res += data.get(&[c1, c2]).unwrap_or(&0.0);
			res += data.get(&[c2, c1]).unwrap_or(&0.0);
		}
		res
	}

	pub fn trigram_stats(&self, layout: &BasicLayout, trigram_precision: usize) -> TrigramStats {
		let mut freqs = TrigramStats::default();
		for (trigram, freq) in self.language_data.trigrams.iter().take(trigram_precision) {
			match layout.get_trigram_pattern(trigram) {
				TrigramPattern::Alternate => freqs.alternates += freq,
				TrigramPattern::AlternateSfs => freqs.alternates_sfs += freq,
				TrigramPattern::Inroll => freqs.inrolls += freq,
				TrigramPattern::Outroll => freqs.outrolls += freq,
				TrigramPattern::Onehand => freqs.onehands += freq,
				TrigramPattern::Redirect => freqs.redirects += freq,
				TrigramPattern::BadRedirect => freqs.bad_redirects += freq,
				TrigramPattern::Other => freqs.other += freq,
				TrigramPattern::Invalid => freqs.invalid += freq
			}
		}
		freqs
	}

	pub fn score(&self, layout: &BasicLayout, trigram_precision: usize) -> f64 {
		let mut score: f64 = 0.0;
		let heatmap = self.effort(layout);
		let sfb = self.bigram_percent(layout, &self.language_data.bigrams);
		let dsfb = self.bigram_percent(layout, &self.language_data.skipgrams);
		let scissors = self.scissor_percent(layout);
		let trigram_data = self.trigram_stats(layout, trigram_precision);

		score -= self.weights.heatmap * heatmap;
		score -= self.weights.sfb * sfb;
		score -= self.weights.dsfb * dsfb;
		score -= self.weights.scissors * scissors;
		score += self.weights.inrolls * trigram_data.inrolls;
		score += self.weights.outrolls * trigram_data.outrolls;
		score += self.weights.onehands * trigram_data.onehands;
		score += self.weights.alternates * trigram_data.alternates;
		score += self.weights.alternates_sfs * trigram_data.alternates_sfs;
		score -= self.weights.redirects * trigram_data.redirects;
		score -= self.weights.bad_redirects * trigram_data.bad_redirects;
		score
	}
}

