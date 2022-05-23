use std::collections::HashMap;

use std::fs::File;
use std::io::prelude::*;
use serde::Deserialize;
use serde_json;

pub type CharacterData = HashMap<char, f64>;
pub type BigramData = HashMap<[char; 2], f64>;
pub type TrigramData = Vec<([char; 3], f64)>;

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
			let new_bigram = [bigram_vec[0], bigram_vec[1]];
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