use std::fs::File;
use std::io::prelude::*;
use std::collections::HashMap;
use json::JsonValue;

pub type CharacterData = HashMap<char, f64>;
pub type BigramData = HashMap<u64, f64>;
pub type TrigramData = HashMap<[char; 3], f64>;

pub struct LanguageData {
	pub characters: CharacterData,
	pub bigrams: BigramData,
	pub skipgrams: BigramData,
	pub trigrams: TrigramData,
	pub language: String
}

impl LanguageData {
    pub fn new(language: &str) -> LanguageData {
        LanguageData::read_language_data(String::from(language))
    }

    pub fn print(&self) {
        println!("{{\n\t\"characters\": {{");
		for (elem, prev) in self.characters.iter() {
			println!("\t\t\"{}\": {},", *elem, prev);
		}
		println!("\t}}\n}}")
    }

	fn read_language_data(language: String) -> LanguageData {
    	let file_path = format!("language_data/{}.json", language.to_lowercase());
    	let mut file = File::open(file_path).expect("couldn't read file!");
    	let mut contents = String::new();

    	file.read_to_string(&mut contents).expect("whoopsie");
    	let language_obj = json::parse(contents.as_str()).unwrap();

		let mut characters = get_characters(&language_obj["characters"]);
		let mut bigrams = get_bigrams(&language_obj["bigrams"], "bigram");
		let mut skipgrams= get_bigrams(&language_obj["skipgrams"], "skipgram");
		let mut trigrams = get_trigrams(&language_obj["trigrams"]);

		LanguageData{characters, bigrams, skipgrams, trigrams, language}
	}
}

fn get_characters(character_obj: &JsonValue) -> CharacterData {
	let mut characters: CharacterData = HashMap::new();
	for (character, prev) in character_obj.entries() {
		if character.chars().count() == 1 {
			characters
				.insert(character.chars().collect::<Vec<char>>()[0], prev
					.as_f64()
					.expect("Data file formatted incorrectly"));
		} else {
			println!("Yo this -- {} -- character was not formatted completely right.\n\
			It has not been added.", character);
		}
	}
	characters
}

fn get_bigrams(bigram_obj: &JsonValue, kind: &str) -> BigramData {
	let mut bigrams: BigramData = HashMap::new();
	for (bigram, prev) in bigram_obj.entries() {
		if bigram.chars().count() == 2 {
			let bi_n = bigram
				.chars()
				.collect::<Vec<char>>();
			bigrams
				.insert(
					((bi_n[0] as u64) << 32) + bi_n[1] as u64,
					prev
						.as_f64()
						.expect("Data file formatted incorrectly")
				);
		} else {
			println!("Yo this -- {} -- {} was not formatted completely right.\n\
			It has not been added.", bigram, kind);
		}
	}
	bigrams
}

fn get_trigrams(trigram_obj: &JsonValue) -> TrigramData {
	let mut trigrams: TrigramData = HashMap::new();
	for (trigram, prev) in trigram_obj.entries() {
		if trigram.chars().count() == 3 {
			let mut tri: [char; 3] = [' '; 3];
			for (i, c) in trigram.chars().enumerate() {
				tri[i] = c;
			}
			trigrams.insert(tri,
							prev
								.as_f64()
								.expect("Data file formatted incorrectly")
				);
		} else {
			println!("This -- {} -- trigram was not formatted completely right. \
			It has not been added.", trigram);
		}
	}
	trigrams
}