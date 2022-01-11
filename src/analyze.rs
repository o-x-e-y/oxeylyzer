use crate::language_data::*;
use std::collections::HashMap;
use std::fmt::Formatter;
use itertools::Itertools;
use crate::analysis::EFFORT_MAP;
use crate::trigram_patterns::*;
use crate::generate::Layout;

#[derive(Clone)]
pub struct TrigramFreq {
	alternates: f64,
	alternates_sfs: f64,
	inrolls: f64,
	outrolls: f64,
	onehands: f64,
	redirects: f64,
	bad_redirects: f64,
	other: f64,
	invalid: f64
}

impl TrigramFreq {
	pub fn new() -> TrigramFreq {
		TrigramFreq{alternates: 0.0, alternates_sfs: 0.0, inrolls: 0.0, outrolls: 0.0, onehands: 0.0,
			redirects: 0.0, bad_redirects: 0.0, other: 0.0, invalid: 0.0}
	}
}

impl std::fmt::Display for TrigramFreq {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write![f, "\nInrolls: {:.3}%\nOutrolls: {:.3}%\nTotal Rolls: {:.3}%\nOnehands: {:.3}%\n\n\
		Alternates: {:.3}%\nAlternates (sfs): {:.3}%\nTotal Alternates: {:.3}%\n\nRedirects: {:.3}%\n\
		Bad Redirects: {:.3}%\nTotal Redirects: {:.3}%\n\nOther: {:.3}%\nInvalid: {:.3}%",
			   self.inrolls*100.0, self.outrolls*100.0, (self.inrolls + self.outrolls)*100.0,
			   self.onehands*100.0, self.alternates*100.0, self.alternates_sfs*100.0,
			   (self.alternates + self.alternates_sfs)*100.0, self.redirects*100.0,self.bad_redirects*100.0,
			   (self.redirects + self.bad_redirects)*100.0, self.other*100.0, self.invalid*100.0]
	}
}

#[derive(Clone)]
struct LayoutStats {
	sfb: f64,
	dsfb: f64,
	trigram_data: TrigramFreq
}

impl std::fmt::Display for LayoutStats {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "Sfb:  {:.3}\nDsfb: {:.3}%\n{}\n",
			   self.sfb * 100.0, self.dsfb * 100.0, self.trigram_data)
	}
}

#[derive(Clone)]
pub struct NameLayout {
	layout: Layout,
	name: String,
	stats: LayoutStats,
	score: f64
}

impl NameLayout {
	fn new(layout: Layout, name: String, stats: LayoutStats, score: f64) -> NameLayout {
		NameLayout {layout, name, stats, score}
	}
}

impl std::fmt::Display for NameLayout {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}{}\n{}\nScore: {:.3}", self.name, self.layout, self.stats, self.score)
	}
}

impl std::cmp::Eq for NameLayout {}

impl std::cmp::PartialEq for NameLayout {
	fn eq(&self, other: &Self) -> bool {
		self.score == other.score
	}
}

impl PartialOrd<Self> for NameLayout {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl std::cmp::Ord for NameLayout {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		return if self.score > other.score {
			std::cmp::Ordering::Less
		} else if self.score == other.score {
			std::cmp::Ordering::Equal
		} else {
			std::cmp::Ordering::Greater
		}
	}
}

pub struct LayoutAnalysis {
	layouts: Vec<NameLayout>,
	stored: HashMap<String, usize>,
	language_data: LanguageData,
}

impl LayoutAnalysis {
	pub fn new(language: &str) -> LayoutAnalysis {
		let mut new_analysis = LayoutAnalysis {
			layouts: Vec::new(),
			stored: HashMap::new(),
			language_data: LanguageData::new(language)
		};
		new_analysis.layouts = new_analysis.import_layouts();
		new_analysis
	}

	fn import_layouts(&mut self) -> Vec<NameLayout> {
		use std::fs;
		let paths = fs::read_dir("layouts").unwrap();
		let mut res: Vec<NameLayout> = Vec::new();
		for path in paths {
			let path = path.unwrap();
			let contents = fs::read_to_string(path.path()).unwrap();
			let layout_str = contents
				.replace("\r", "")
				.replace("\n", "")
				.replace(" ", "");
			let name = path.path();
			let name = name.to_str().unwrap().split('\\').collect::<Vec<&str>>();
			let name = String::from(name[name.len()-1]);
			let name = name.split('.').collect::<Vec<&str>>()[0].to_string();
			let layout = Layout::from_str(layout_str.as_str());
			let precision = self.language_data.trigrams.len();
			let score = self.score(&layout, precision);
			let stats = self.layout_stats(&layout, precision);
			res.push(NameLayout::new(layout, name.clone(), stats, score));
		}
		res.sort_unstable();
		for (i, l) in res.iter().enumerate() {
			let name = l.name.clone();
			self.stored.insert(name, i);
		}
		res
	}

	pub fn rank(&mut self) {
		self.layouts.sort_unstable();
		for layout in self.layouts.iter() {
			println!("{:12 }{}", format!("{:.2}:", layout.score), layout.name);
		}
		println!();
	}

	pub fn name_layout_by_name(&self, name: &str) -> Option<NameLayout> {
		let index: usize = match self.stored.get(name) {
			None => usize::MAX,
			Some(n) => *n
		};
		return if index != usize::MAX {
			Some(self.layouts[index].clone())
		} else {
			None
		}
	}

	pub fn layout_by_name(&self, name: &str) -> Option<Layout> {
		let name_layout = self.name_layout_by_name(name);
		return match name_layout {
			Some(n) => Some(n.layout),
			_ => None
		}
	}

	pub fn analyze_name(&self, name: &str) {
		let layout = self.name_layout_by_name(name).expect("layout does not exist");
		self.analyze(&layout);
	}

	pub fn analyze_str(&mut self, layout_str: &str) {
		let layout = Layout::from_str(layout_str);
		let precision = self.language_data.trigrams.len();
		let score = self.score(&layout, precision);
		let stats = self.layout_stats(&layout, precision);
		let name = layout_str[10..17].to_string();
		let name_layout = NameLayout::new(layout, name.clone(), stats, score);
		// self.stored.insert(name, self.layouts.len());
		// self.layouts.push(name_layout);
		self.analyze(&name_layout);
	}

	pub fn analyze(&self, layout: &NameLayout) {
		println!("{}", layout);
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

	pub fn effort(&self, layout: &Layout) -> f64 {
		let mut res: f64 = 0.0;
		for x in 0..10 {
			for (y, c) in layout.matrix[x].iter().enumerate() {
				res += EFFORT_MAP[x][y] * self.language_data.characters.get(c).unwrap_or(&0.0);
			}
		}
		res
	}

	pub fn score(&self, layout: &Layout, trigram_precision: usize) -> f64 {
		let mut score: f64 = 0.0;
		let sfb = self.bigram_percent(&layout, &self.language_data.bigrams);
		let dsfb = self.bigram_percent(&layout, &self.language_data.skipgrams);
		let trigram_data = self.trigram_stats(&layout, trigram_precision);
		score -= 1.5 * (self.effort(&layout) - 1.0);
		score -= 10.0 * sfb;
		score -= 3.0 * dsfb;
		score += 1.2 * trigram_data.inrolls;
		score += 0.4 * trigram_data.outrolls;
		score += 0.5 * trigram_data.onehands;
		score += 0.95 * trigram_data.alternates;
		score += 0.3 * trigram_data.alternates_sfs;
		score -= 2.0 * trigram_data.redirects;
		score -= 8.0 * trigram_data.bad_redirects;
		score
	}

	pub fn bigram_percent(&self, layout: &Layout, data: &BigramData) -> f64 {
		let mut sfb: f64 = 0.0;
		for i in 0..3 {
			sfb += self._sfb_for_finger_iter(layout.matrix[i].iter(), data);
		}
		for i in 0..2 {
			sfb += self._sfb_for_finger_iter(layout.get_index(i).iter(), data);
		}
		for i in 7..10 {
			sfb += self._sfb_for_finger_iter(layout.matrix[i].iter(), data);
		}
		sfb
	}

	fn _sfb_for_finger_iter<'a>(&self, finger: impl Iterator<Item=&'a char>, data: &BigramData) -> f64 {
		let mut sfb = 0.0;
		finger
			.permutations(2)
			.for_each(|x| {
				let bi = ((*x[0] as u64) << 32) + *x[1] as u64;
				sfb += data.get(&bi).unwrap_or(&0.0);
			});
		sfb
	}

	pub fn trigram_stats(&self, layout: &Layout, trigram_precision: usize) -> TrigramFreq {
		let mut freqs = TrigramFreq::new();
		for (i, (trigram, freq)) in self.language_data.trigrams.iter().enumerate() {
			if i == trigram_precision {
				break;
			}
			match layout.get_trigram_pattern(trigram) {
				TrigramPattern::Alternate => freqs.alternates += freq,
				TrigramPattern::AlternateSfs => freqs.alternates_sfs += freq,
				TrigramPattern::Inroll => freqs.inrolls += freq,
				TrigramPattern::Outroll => freqs.outrolls += freq,
				TrigramPattern::OneHand => freqs.onehands += freq,
				TrigramPattern::Redirect => freqs.redirects += freq,
				TrigramPattern::BadRedirect => freqs.bad_redirects += freq,
				TrigramPattern::Other => freqs.other += freq,
				TrigramPattern::Invalid => freqs.invalid += freq
			}
		}
		freqs
	}

	fn layout_stats(&self, layout: &Layout, trigram_precision: usize) -> LayoutStats {
		let sfb = self.bigram_percent(&layout, &self.language_data.bigrams);
		let dsfb = self.bigram_percent(&layout, &self.language_data.skipgrams);
		let trigram_data = self.trigram_stats(&layout, trigram_precision);
		LayoutStats{ sfb, dsfb, trigram_data }
	}
}

