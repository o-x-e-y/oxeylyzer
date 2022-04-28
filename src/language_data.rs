use std::collections::HashMap;

pub type CharacterData = HashMap<char, f64>;
pub type BigramData = HashMap<u64, f64>;
pub type TrigramData = Vec<([char; 3], f64)>;

pub mod json {
	use super::{CharacterData, BigramData, TrigramData};
	use std::fs::File;
	use std::io::prelude::*;
	use std::collections::HashMap;
	use serde::Deserialize;
	use serde_json;

	#[derive(Deserialize)]
	struct LanguageDataInter {
		pub characters: HashMap<char, f64>,
		pub bigrams: HashMap<String, f64>,
		pub skipgrams: HashMap<String, f64>,
		pub trigrams: HashMap<String, f64>,
		#[serde(default)]
		pub language: String
	}

	impl LanguageDataInter {
		pub fn get_bigram_data(data: HashMap<String, f64>) -> BigramData {
			let mut res = BigramData::new();
			for (bigram, freq) in data {
				let bigram_vec = bigram.chars().collect::<Vec<char>>();
				let new_bigram = ((bigram_vec[0] as u64) << 32) + bigram_vec[1] as u64;
				res.insert(new_bigram, freq);
			}
			res
		}

		pub fn get_trigram_data(data: HashMap<String, f64>) -> TrigramData {
			let mut res = TrigramData::new();
			for (trigram, freq) in data {
				let trigram_vec = trigram.chars().collect::<Vec<char>>();
				let new_trigram = [trigram_vec[0], trigram_vec[1], trigram_vec[2]];
				res.push((new_trigram, freq));
			}
			res
		}
	}

	#[derive(Deserialize)]
	pub struct LanguageData {
		pub characters: CharacterData,
		pub bigrams: BigramData,
		pub skipgrams: BigramData,
		pub trigrams: TrigramData,
		pub language: String
	}

	impl From<LanguageDataInter> for LanguageData {
		fn from(inter: LanguageDataInter) -> Self {
			let bigrams = LanguageDataInter::get_bigram_data(inter.bigrams);
			let skipgrams = LanguageDataInter::get_bigram_data(inter.skipgrams);
			let trigrams = LanguageDataInter::get_trigram_data(inter.trigrams);
			Self {characters: inter.characters, bigrams, skipgrams, trigrams, language: inter.language}
		}
	}

	impl LanguageData {
		pub fn new(language: &str) -> LanguageData {
			LanguageData::read_language_data(String::from(language))
		}

		fn read_language_data(language: String) -> LanguageData {
			let file_path = format!("static/language_data/{}.json", language.to_lowercase());
			let mut file = File::open(file_path).expect("couldn't read file!");
			let mut contents = String::new();

			file.read_to_string(&mut contents).expect("whoopsie");
			let mut data: LanguageDataInter = serde_json::from_str(contents.as_str()).unwrap();
			data.language = language.to_lowercase();
			LanguageData::from(data)
		}
	}
}

pub mod text {
	use super::{CharacterData, BigramData, TrigramData};
	use crate::translation::Translator;
	
	use serde::Serialize;
	use std::collections::{HashMap, BTreeMap};
	use std::io::Write;
	use std::iter;
	use std::fs;
	use std::sync::atomic::{AtomicUsize, Ordering};
	use itertools::Itertools;
	use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};

	#[derive(Serialize)]
	struct DataToSave {
		pub language: String,
		pub characters: BTreeMap<String, f64>,
		pub bigrams: BTreeMap<String, f64>,
		pub skipgrams: BTreeMap<String, f64>,
		pub trigrams: BTreeMap<String, f64>
	}

	impl DataToSave {
		pub fn format_stat(stat: HashMap<String, f64>, length: u64) -> BTreeMap<String, f64> {
			let length = length as f64;
			let mut stat = stat
				.into_iter()
				.map(|(c, mut freq)| {
					(c, freq/length)
				})
				.filter(|(item, freq)| {
					!freq.is_nan() && freq > &0f64
				})
				.collect::<Vec<_>>();

			stat.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());

			BTreeMap::from_iter(stat)
		}

		fn get_skipgrams(trigrams: &HashMap<String, f64>) -> HashMap<String, f64> {
			let mut skipgrams: HashMap<String, f64> = HashMap::new();
			
			for (trigram, freq) in trigrams.iter() {
				let mut chars = trigram.chars();
				let skip1 = chars.next().unwrap();
				chars.next();
				let skip2 = chars.next().unwrap();

				let new_skipgram = String::from_iter([skip1, skip2]);
				let bigram_entry = skipgrams.entry(new_skipgram).or_insert_with(|| 0.0);
				*bigram_entry += freq;
			}
			skipgrams
		}
	}

	impl core::convert::From<(DataToSaveInter, &str)> for DataToSave {
		fn from((data, language): (DataToSaveInter, &str)) -> Self {
			let skipgrams = Self::get_skipgrams(&data.trigrams);
			Self {
				characters: Self::format_stat(data.characters, data.total_length),
				bigrams: Self::format_stat(data.bigrams, data.total_length),
				skipgrams: Self::format_stat(skipgrams, data.total_length),
				trigrams: Self::format_stat(data.trigrams, data.total_length),
				language: language.to_string()
			}
		}
	}

	#[derive(Default)]
	struct DataToSaveInter {
		pub characters: HashMap<String, f64>,
		pub bigrams: HashMap<String, f64>,
		pub trigrams: HashMap<String, f64>,
		pub total_length: u64
	}

	impl std::ops::Add for DataToSaveInter {
		type Output = Self;

		fn add(self, rhs: Self) -> Self::Output {
			Self {
				characters: add_stat(self.characters, rhs.characters),
				bigrams: add_stat(self.bigrams, rhs.bigrams),
				trigrams: add_stat(self.trigrams, rhs.trigrams),
				total_length: self.total_length + rhs.total_length
			}
		}
	}

	fn add_stat(mut stat1: HashMap<String, f64>, stat2: HashMap<String, f64>) -> HashMap<String, f64> {
		for (item, freq) in stat2.into_iter() {
			let stat_entry = stat1.entry(item).or_insert_with(|| 0.0);
			*stat_entry += freq;
		}
		stat1
	}

	pub fn generate_data(language: &str, translator: Translator) {
		let paths = fs::read_dir(format!("static/text/{language}/")).unwrap();

		let paths = paths
			.map(|p| {
				p.ok().unwrap()
			})
			.collect::<Vec<fs::DirEntry>>();
		
		let grams = paths.into_iter()
			.map(|p| {
				println!("path: {}", p.path().display());
				let text = fs::read_to_string(p.path()).unwrap();
				let trans_text = translator.translate(text);
				load_language_data(trans_text)
			})
			.reduce(|a, b| a + b)
			.unwrap_or_default();

		let to_save = DataToSave::from((grams, language));
		save_stats(to_save);
	}

	fn get_trigrams(s: String) -> Vec<(char, char, char)> {
		let it_1 = iter::once(' ').chain(iter::once(' ')).chain(s.chars());
		let it_2 = iter::once(' ').chain(s.chars());
		let it_3 = s.chars();

		let res: Vec<(char, char, char)> = it_1
			.zip(it_2)
			.zip(it_3)
			.map(|((a, b), c): ((char, char), char)| (a, b, c))
			.collect();
		res
	}

	fn load_language_data(text: String) -> DataToSaveInter {
		let mut characters: HashMap<String, f64> = HashMap::new();
		let mut bigrams: HashMap<String, f64> = HashMap::new();
		let mut trigrams: HashMap<String, f64> = HashMap::new();
		let length = text.len();

		for trigram in get_trigrams(text) {
			let char_entry = characters.entry(String::from(trigram.2)).or_insert_with(|| 0.0);
			*char_entry += 1.0;

			let new_bigram = String::from_iter([trigram.1, trigram.2]);
			let bigram_entry = bigrams.entry(new_bigram).or_insert_with(|| 0.0);
			*bigram_entry += 1.0;

			let new_trigram = String::from_iter([trigram.0, trigram.1, trigram.2]);
			let trigram_entry = trigrams.entry(new_trigram).or_insert_with(|| 0.0);
			*trigram_entry += 1.0;
		}

		DataToSaveInter {
			characters,
			bigrams,
			trigrams,
			total_length: length as u64
		}
	}

	fn save_stats(stats: DataToSave) {
		let buf = Vec::new();
		let formatter = serde_json::ser::PrettyFormatter::with_indent(b"\t");
		let mut ser = serde_json::Serializer::with_formatter(buf, formatter);
		stats.serialize(&mut ser).unwrap();

		let mut file = fs::OpenOptions::new()
			.write(true)
			.create(true)
			.open(format!("static/langdat/{}.json", stats.language))
			.unwrap();
		
		file.write(ser.into_inner().as_ref());
	}
}