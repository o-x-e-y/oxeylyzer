use fxhash::FxHashMap;
use indexmap::IndexMap;
use anyhow::Result;

use std::fs::File;
use std::io::prelude::*;
use serde::Deserialize;
use serde_json;

pub type CharacterData = FxHashMap<u8, f64>;
pub type BigramData = FxHashMap<[u8; 2], f64>;
pub type TrigramData = Vec<([u8; 3], f64)>;
pub type CharEncode = FxHashMap<char, u8>;
pub type CharDecode = FxHashMap<u8, char>;

#[derive(Deserialize)]
struct LanguageDataInter {
	pub language: String,
	pub characters: FxHashMap<char, f64>,
	pub bigrams: FxHashMap<String, f64>,
	pub skipgrams: FxHashMap<String, f64>,
	pub trigrams: IndexMap<String, f64>,
	#[serde(skip)]
	pub encode: CharEncode,
	#[serde(skip)]
	pub decode: CharDecode
}

impl LanguageDataInter {
	pub fn char_u8(&mut self) {
		let encode = self.characters
			.iter()
			.zip(0u8..)
			.map(
				|((c, _), i)| (*c, i)
			)
			.collect::<CharEncode>();
		
		let decode = encode
			.iter()
			.map(|(f, t)| (*t, *f))
			.collect::<CharDecode>();

		self.encode = encode;
		self.decode = decode;
	}

	pub fn get_character_data(&self, data: &FxHashMap<char, f64>) -> CharacterData {
		let mut res = CharacterData::default();
		for (c, freq) in data {
			let new_c = self.encode.get(&c).unwrap();
			res.insert(*new_c, *freq);
		}
		res
	}

	pub fn get_bigram_data(&self, data: &FxHashMap<String, f64>) -> BigramData {
		let mut res = BigramData::default();
		for (bigram, freq) in data {
			let bv = bigram.chars().collect::<Vec<char>>();

			let b1 = self.encode.get(&bv[0]).unwrap();
			let b2 = self.encode.get(&bv[1]).unwrap();

			let new_bigram = [*b1, *b2];
			res.insert(new_bigram, *freq);
		}
		res
	}

	pub fn get_trigram_data(&self, data: &IndexMap<String, f64>) -> TrigramData {
		let mut res = TrigramData::new();
		for (trigram, freq) in data {
			let tv = trigram.chars().collect::<Vec<char>>();
			if tv[0] != tv[1] && tv[1] != tv[2] {
				let t1 = self.encode.get(&tv[0]).unwrap();
				let t2 = self.encode.get(&tv[1]).unwrap();
				let t3 = self.encode.get(&tv[2]).unwrap();

				let new_trigram = [*t1, *t2, *t3];
				res.push((new_trigram, *freq));
			}
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
	pub language: String,
	pub encode: CharEncode,
	pub decode: CharDecode
}

impl From<LanguageDataInter> for LanguageData {
	fn from(inter: LanguageDataInter) -> Self {
		let characters = inter.get_character_data(&inter.characters);
		let bigrams = inter.get_bigram_data(&inter.bigrams);
		let skipgrams = inter.get_bigram_data(&inter.skipgrams);
		let trigrams = inter.get_trigram_data(&inter.trigrams);

		Self {
			characters, bigrams, skipgrams, trigrams,
			language: inter.language,
			encode: inter.encode,
			decode: inter.decode
		}
	}
}

impl LanguageData {
	pub fn new(language: &str) -> Result<LanguageData> {
		let res = LanguageData::read_language_data(String::from(language))?;
		println!("language read: {}", res.language);
		Ok(res)
	}

	fn read_language_data(language: String) -> Result<LanguageData> {
		let file_path = format!("static/language_data/{}.json", language.to_lowercase());
		let mut file = File::open(file_path)?;
		let mut contents = String::new();

		file.read_to_string(&mut contents)?;
		let mut data: LanguageDataInter = serde_json::from_str(contents.as_str())?;
		data.char_u8();
		Ok(LanguageData::from(data))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn encode_decode() {
		let data = LanguageData::new("dutch").unwrap();
		for i in 0u8.. {
			if let Some(c) = data.decode.get(&i) {
				let new_i = data.encode.get(c).unwrap();
				println!("decode {i}: {c}, encode: {c}: {}", new_i);
				assert_eq!(*new_i, i);
			} else {
				break;
			}
		}
	}
}