use ahash::AHashMap as HashMap;
use anyhow::Result;
use indexmap::IndexMap;
use serde::Deserialize;
use serde_json;

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use crate::char_mapping::CharMapping;

pub type CharacterData = Box<[f64]>;
pub type SlowBigramData = HashMap<[u8; 2], f64>;
pub type BigramData = Box<[f64]>;
pub type TrigramData = Vec<([u8; 3], f64)>;

#[derive(Deserialize)]
struct LanguageDataInter {
    pub language: String,
    pub characters: HashMap<char, f64>,
    pub bigrams: HashMap<String, f64>,
    pub skipgrams: HashMap<String, f64>,
    pub skipgrams2: HashMap<String, f64>,
    pub skipgrams3: HashMap<String, f64>,
    pub trigrams: IndexMap<String, f64>,
}

fn get_char_data(data: HashMap<char, f64>, con: &mut CharMapping) -> CharacterData {
    let mut chars = vec![0.0; data.len() + 1];

    for (c, f) in data {
        con.push(c);

        let i = con.get_u(c) as usize;
        chars[i] = f;
    }

    assert_eq!(con.len() as usize, chars.len());

    chars.into_boxed_slice()
}

fn get_bigram_data(data: HashMap<String, f64>, con: &mut CharMapping) -> BigramData {
    let len = con.len() as usize;
    let mut bigrams = vec![0.0; len.pow(2)];

    for (s, f) in data {
        let cs = s.chars().collect::<Vec<_>>();

        let u1 = con.get_u(cs[0]) as usize;
        let u2 = con.get_u(cs[1]) as usize;

        bigrams[u1 * len + u2] = f;
    }

    bigrams.into_boxed_slice()
}

fn get_trigram_data(data: IndexMap<String, f64>, con: &mut CharMapping) -> TrigramData {
    let mut res = TrigramData::new();
    for (trigram, freq) in data {
        let tv_u8 = con.map_cs(&trigram).collect::<Vec<_>>();

        // TODO: consider re-adding sfr, which this filters.
        if tv_u8[0] != tv_u8[1] && tv_u8[1] != tv_u8[2] {
            let new_trigram = [tv_u8[0], tv_u8[1], tv_u8[2]];
            res.push((new_trigram, freq));
        }
    }
    res
}
pub struct LanguageData {
    pub characters: CharacterData,
    pub bigrams: BigramData,
    pub skipgrams: BigramData,
    pub skipgrams2: BigramData,
    pub skipgrams3: BigramData,
    pub weighted_bigrams: BigramData,
    pub stretch_weighted_bigrams: BigramData,
    pub trigrams: TrigramData,
    pub language: String,
    pub char_mapping: CharMapping,
}

impl From<LanguageDataInter> for LanguageData {
    fn from(mut inter: LanguageDataInter) -> Self {
        let mut char_mapping = CharMapping::new();

        for c in ['\'', ',', '.', ';', '/', '~'] {
            inter.characters.entry(c).or_insert(0.0);
        }

        let characters = get_char_data(inter.characters, &mut char_mapping);

        let bigrams = get_bigram_data(inter.bigrams, &mut char_mapping);
        let skipgrams = get_bigram_data(inter.skipgrams, &mut char_mapping);
        let skipgrams2 = get_bigram_data(inter.skipgrams2, &mut char_mapping);
        let skipgrams3 = get_bigram_data(inter.skipgrams3, &mut char_mapping);

        let weighted_bigrams = Box::new([]);
        let stretch_weighted_bigrams = Box::new([]);

        let trigrams = get_trigram_data(inter.trigrams, &mut char_mapping);

        Self {
            characters,
            bigrams,
            skipgrams,
            skipgrams2,
            skipgrams3,
            trigrams,
            weighted_bigrams,
            stretch_weighted_bigrams,
            language: inter.language,
            char_mapping,
        }
    }
}

impl LanguageData {
    pub fn new(text: &str) -> Result<LanguageData> {
        let data: LanguageDataInter = serde_json::from_str(text)?;
        Ok(LanguageData::from(data))
    }

    pub fn from_file<P>(base_path: P, language: &str) -> Result<LanguageData>
    where
        P: AsRef<Path>,
    {
        let file_path = base_path.as_ref().join(language.to_lowercase() + ".json");
        let mut file = File::open(file_path)?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let data: LanguageDataInter = serde_json::from_str(contents.as_str())?;
        let res = LanguageData::from(data);

        Ok(res)
    }

    pub fn get_stretch_weighted_bigram(&self, [c1, c2]: [char; 2]) -> f64 {
        let u1 = self.char_mapping.get_u(c1) as usize;
        let u2 = self.char_mapping.get_u(c2) as usize;

        if u1 < self.characters.len() && u2 < self.characters.len() {
            let i = u1 * self.characters.len() + u2;
            self.stretch_weighted_bigrams[i]
        } else {
            0.0
        }
    }

    #[inline]
    pub fn get_stretch_weighted_bigram_u(&self, [c1, c2]: [u8; 2]) -> f64 {
        let u1 = c1 as usize;
        let u2 = c2 as usize;

        if u1 < self.characters.len() && u2 < self.characters.len() {
            let i = u1 * self.characters.len() + u2;
            self.stretch_weighted_bigrams[i]
        } else {
            0.0
        }
    }
}
