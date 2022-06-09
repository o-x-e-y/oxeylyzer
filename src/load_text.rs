use crate::translation::Translator;
use std::collections::HashMap;
use std::iter::FromIterator;
use std::fs::{File, read_dir};
use std::time::Instant;

use rayon::iter::{IntoParallelIterator, ParallelIterator};
use file_chunker::FileChunker;
use anyhow::Result;
use indexmap::IndexMap;
use serde::{Serialize, Deserialize};

const TWO_MB: u64 = 1024 * 1024 * 2;

pub fn load_raw(language: &str) {
    let translator = Translator::new()
        .passthrough()
        .build();
    load_data(language, translator).unwrap();
}

pub fn load_default(language: &str) {
    let start = Instant::now();
    let translator = Translator::language_or_passthrough(language);
    println!("\nBuilding the translator for {} took {}ms", language, (Instant::now() - start).as_millis());
	if let Err(_) = load_data(language, translator) {
        println!("{} failed to update", language);
    }
}

pub fn load_all_default() -> Result<()> {
    let start_total = Instant::now();

    std::fs::read_dir(format!("static/text/"))?
        .filter_map(Result::ok)
        .for_each(|language_dir| {
			let language = language_dir.path().display().to_string().replace("\\", "/");
			let language = language.split("/").last().unwrap();
			load_default(language);
        }
    );
    println!("loading all languages took {}ms", (Instant::now() - start_total).as_millis());
    Ok(())
}

pub fn load_data(language: &str, translator: Translator) -> Result<TextData> {
    let start_total = Instant::now();

    let all_trigrams = read_dir(format!("static/text/{language}/"))?
        .filter_map(Result::ok)
        .map(|dir_entry| -> Result<TextTrigrams> {
            let f = File::open(dir_entry.path())?;
            TextTrigrams::try_from(f)
        })
        .filter_map(Result::ok)
        .reduce(|accum, new| accum.combine_with(new))
        .unwrap_or(TextTrigrams::default());
    
    let res = TextData::from((all_trigrams, translator, language));
    res.save()?;
    println!("loading {} took {}ms", language, (Instant::now() - start_total).as_millis());
    Ok(res)
}

#[derive(Default)]
struct TextTrigrams {
    pub trigrams: HashMap<[char; 3], usize>,
}

impl TryFrom<File> for TextTrigrams {
    type Error = anyhow::Error;

    fn try_from(f: File) -> Result<Self, Self::Error> {
        let thread_count = (f.metadata()?.len() / TWO_MB + 1).min(12);
        
        let chunker = FileChunker::new(&f)?;

        let trigrams = chunker.chunks(thread_count as usize, None)?
            .into_par_iter()
            .map(|chunk| {
                let text = String::from_utf8_lossy(chunk);
                TextTrigrams::from(text.as_ref())
            })
            .reduce(
                || TextTrigrams::default(),
                |accum, new| accum.combine_with(new)
            );
        Ok(trigrams)
    }
}

impl From<&str> for TextTrigrams {
    fn from(s: &str) -> Self {
        let mut trigrams: HashMap<[char; 3], usize> = HashMap::new();
        let mut chars = "  ".chars().chain(s.chars().chain("  ".chars()));

        if let Some(mut c1) = chars.next() {
            if let Some(mut c2) = chars.next() {
                while let Some(c3) = chars.next() {
                    *trigrams.entry([c1, c2, c3]).or_insert_with(|| 0) += 1;
                    c1 = c2;
                    c2 = c3;
                }
            }
        }
        Self { trigrams }
    }
}

impl TextTrigrams {
    fn combine_with(mut self, rhs: Self) -> Self {
        for (trigram, freq) in rhs.trigrams.into_iter() {
            *self.trigrams.entry(trigram).or_insert_with(|| 0) += freq;
        }
        self
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct TextData {
    language: String,

    characters: IndexMap<String, f64>,
    bigrams: IndexMap<String, f64>,
    skipgrams: IndexMap<String, f64>,
    trigrams: IndexMap<String, f64>,

    #[serde(skip)]
    char_sum: f64,
    #[serde(skip)]
    bigram_sum: f64,
    #[serde(skip)]
    skipgram_sum: f64,
    #[serde(skip)]
    trigram_sum: f64
}

impl std::fmt::Display for TextData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{
\"language\": {},
\"characters\": {:#?},
\"bigrams\": {:#?},
\"skipgrams\": {:#?},
\"trigrams\": {:#?}
}}",
            self.language,
            self.characters,
            self.bigrams,
            self.skipgrams,
            self.trigrams
        )
    }
}

impl From<(TextTrigrams, Translator, &str)> for TextData {
    fn from((data, translator, language): (TextTrigrams, Translator, &str)) -> Self {
        let mut res = TextData::default();
        res.language = language.replace(" ", "_");

        for (trigram, freq) in data.trigrams.into_iter() {
            let s = String::from_iter(trigram);
            let c1_as_string = String::from(trigram[0]);
            let first_trans = translator.translate(c1_as_string.as_str());
            let trans = translator.translate(s.as_str());
            let mut chars = trans.chars();
            if first_trans.len() > 0 {
                if let Some(c1) = chars.next() {
                    if let Some(c2) = chars.next() {
                        if let Some(c3) = chars.next() {
                            res.add_from_three_subsequent(c1, c2, c3, freq as f64);
                        }
                    }
                }
            }
            if first_trans.len() > 1 {
                let mut chars_c = first_trans.chars();
                chars_c.next();
                while let Some(c) = chars_c.next() {
                    if c != ' ' {
                        res.add_character(c, freq as f64);
                    }
                }
                let mut chars_b = first_trans.chars();
                chars_b.next();
                if let Some(mut c1) = chars_b.next() {
                    while let Some(c2) = chars_b.next() {
                        if c1 != ' ' && c2 != ' ' {
                            res.add_bigram([c1, c2], freq as f64);
                        }
                        c1 = c2;
                    }
                }
                let mut chars_t = first_trans.chars();
                chars_t.next();
                if let Some(mut c1) = chars_b.next() {
                    if let Some(mut c2) = chars_b.next() {
                        while let Some(c3) = chars_b.next() {
                            if c1 != ' ' && c3 != ' '{
                                res.add_skipgram([c1, c3], freq as f64);
                                if c2 != ' ' {
                                    res.add_trigram([c1, c2, c3], freq as f64);
                                }
                            }
                            c1 = c2;
                            c2 = c3;
                        }
                    } 
                }
            }
        }
        // IndexMaps have the propertiy of being sorted based on insertion, so they're sortable:
        res.characters.iter_mut().for_each(|(_, f)| *f /= res.char_sum);
        res.bigrams.iter_mut().for_each(|(_, f)| *f /= res.bigram_sum);
        res.skipgrams.iter_mut().for_each(|(_, f)| *f /= res.skipgram_sum);
        res.trigrams.iter_mut().for_each(|(_, f)| *f /= res.trigram_sum);
        
        res.characters.sort_by(|_, f1, _, f2| f2.partial_cmp(f1).unwrap());
        res.bigrams.sort_by(|_, f1, _, f2| f2.partial_cmp(f1).unwrap());
        res.trigrams.sort_by(|_, f1, _, f2| f2.partial_cmp(f1).unwrap());
        res.skipgrams.sort_by(|_, f1, _, f2| f2.partial_cmp(f1).unwrap());

        res
    }
}

impl TextData {
    fn add_from_three_subsequent(&mut self, c1: char, c2: char, c3: char, freq: f64) {
        if c1 != ' ' {
            self.add_character(c1, freq);
            // take first, first 2 etc chars of the trigram every time for the appropriate stat
            // as long as they don't contain spaces
            if c2 != ' ' {
                self.add_bigram([c1, c2], freq);
                if c3 != ' ' {
                    self.add_trigram([c1, c2, c3], freq);
                }
            }
            // c1 and c3 for skipgrams
            if c3 != ' ' {
                self.add_skipgram([c1, c3], freq);
            }
        }
    }

    pub(crate) fn add_character(&mut self, c1: char, freq: f64) {
        *self.characters
            .entry(String::from(c1))
            .or_insert_with(|| 0.0) += freq;
        self.char_sum += freq;
    }

    pub(crate) fn add_bigram(&mut self, bigram: [char; 2], freq: f64) {
        *self.bigrams
            .entry(String::from_iter(bigram))
            .or_insert_with(|| 0.0) += freq;
        self.bigram_sum += freq;
    }

    pub(crate) fn add_skipgram(&mut self, skipgram: [char; 2], freq: f64) {
        *self.skipgrams
            .entry(String::from_iter(skipgram))
            .or_insert_with(|| 0.0) += freq;
        self.skipgram_sum += freq;
    }

    pub(crate) fn add_trigram(&mut self, trigram: [char; 3], freq: f64) {
        *self.trigrams
            .entry(String::from_iter(trigram))
            .or_insert_with(|| 0.0) += freq;
        self.trigram_sum += freq;
    }

    fn save(&self) -> Result<()> {
        use std::fs::OpenOptions;
        use std::io::Write;

        let buf = Vec::new();
        let formatter = serde_json::ser::PrettyFormatter::with_indent(b"\t");
        let mut ser = serde_json::Serializer::with_formatter(buf, formatter);
        self.serialize(&mut ser).unwrap();

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(format!("static/language_data/{}.json", self.language))?;
        
        file.write(ser.into_inner().as_slice())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_language() {
        let translator = Translator::new()
            .language("english").unwrap()
            .build();
        
        let data = load_data("test", translator).unwrap();

        let reference = serde_json::from_str::<TextData>(
            "{
                \"language\": \"test\",
                \"characters\": {
                    \".\": 0.6,
                    \"=\": 0.1,
                    \"`\": 0.1,
                    \"a\": 0.1,
                    \"b\": 0.1
                },
                \"bigrams\": {
                    \"..\": 0.6666666666666666,
                    \"a.\": 0.16666666666666666,
                    \"b`\": 0.16666666666666666
                },
                \"skipgrams\": {
                    \"..\": 0.6666666666666666,
                    \"a.\": 0.3333333333333333
                },
                \"trigrams\": {
                    \"...\": 0.6666666666666666,
                    \"a..\": 0.3333333333333333
                }
            }"
        ).unwrap();

        assert_eq!(reference.language, data.language);
        for (c, freq) in reference.characters.iter() {
            assert!((*data.characters.get(c).unwrap() - freq).abs() < 0.000000001);
        }

        for (c, freq) in reference.bigrams.iter() {
            assert!((*data.bigrams.get(c).unwrap() - freq).abs() < 0.000000001);
        }
        for (c, freq) in reference.skipgrams.iter() {
            assert!((*data.skipgrams.get(c).unwrap() - freq).abs() < 0.000000001);
        }
        for (c, freq) in reference.trigrams.iter() {
            assert!((*data.trigrams.get(c).unwrap() - freq).abs() < 0.000000001);
        }
        assert!(data.characters.len() == reference.characters.len());
        assert!(data.bigrams.len() == reference.bigrams.len());
        assert!(data.skipgrams.len() == reference.skipgrams.len());
        assert!(data.trigrams.len() == reference.trigrams.len());

        assert!(data.characters.into_iter().map(|(_, f)| f).sum::<f64>() - 1.0 < 0.000000001);
        assert!(data.bigrams.into_iter().map(|(_, f)| f).sum::<f64>() - 1.0 < 0.000000001);
        assert!(data.skipgrams.into_iter().map(|(_, f)| f).sum::<f64>() - 1.0 < 0.000000001);
        assert!(data.trigrams.into_iter().map(|(_, f)| f).sum::<f64>() - 1.0 < 0.000000001);
    }
}
