use crate::translation::Translator;

use std::collections::HashMap;
use std::iter::FromIterator;
use std::fs::{File, read_dir};
use std::time::Instant;

use itertools::Itertools;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use file_chunker::FileChunker;
use anyhow::Result;
use indexmap::IndexMap;
use serde::{Serialize, Deserialize};
use smartstring::{SmartString, Compact};

const TWO_MB: u64 = 1024 * 1024 * 2;

pub fn load_raw(language: &str) {
    let translator = Translator::new()
        .raw()
        .build();
    load_data(language, translator).unwrap();
}

pub fn load_default(language: &str) {
    let translator = Translator::language_or_raw(language);
	if let Err(e) = load_data(language, translator) {
        println!("{} failed to update: '{}'", language, e);
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
    println!("path: {}", format!("static/text/{language}"));

    let all_trigrams = read_dir(format!("static/text/{language}"))?
        .filter_map(Result::ok)
        .map(|dir_entry| -> Result<TextTrigrams> {
            let f = File::open(dir_entry.path())?;
            TextTrigrams::try_from(f)
        })
        .filter_map(Result::ok)
        .reduce(|accum, new| accum.combine_with(new))
        .unwrap_or(TextTrigrams::default());

    let is_raw = translator.is_raw;
    let res = TextData::from((all_trigrams, translator, language));
    res.save(is_raw)?;
    println!("loading {} took {}ms", language, (Instant::now() - start_total).as_millis());
    Ok(res)
}

#[derive(Default, Debug)]
pub struct TextTrigrams {
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

        let mut chars = "  ".chars().chain(s.chars().chain("  ".chars()))
            .tuple_windows::<(_, _, _)>();

        while let Some((c1, c2, c3)) = chars.next() {
            *trigrams.entry([c1, c2, c3]).or_insert_with(|| 0) += 1;
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

#[derive(Default, Debug, Serialize, Deserialize)]
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
            let s: SmartString<Compact> = SmartString::from_iter(trigram);
            let translated = translator.translate(s.as_str());
            let trans_len = translated.chars().count();

            let mut it_count = if let Some(f) = translator.table.get(&trigram[0]) {
                f.len()
            } else { 0 };

            if it_count > 0 {
                match trans_len {
                    3.. => {
                        let mut it = translated.chars()
                            .tuple_windows::<(_, _, _)>();
    
                        while let Some((c1, c2, c3)) = it.next() && it_count > 0 {
                            res.add_from_three_subsequent(c1, c2, c3, freq as f64);
                            it_count -= 1;
                        }
                    },
                    2 => {
                        let (c1, c2) = translated.chars().next_tuple::<(_, _)>().unwrap();
                        res.add_from_two_subsequent(c1, c2, freq as f64)
                    },
                    1 => {
                        let c1 = translated.chars().next().unwrap();
                        res.add_character(c1, freq as f64);
                    },
                    _ => {}
                }
            }
        }

        // IndexMaps have the property of being sorted based on insertion, so they're sortable:
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

    fn add_from_two_subsequent(&mut self, c1: char, c2: char, freq: f64) {
        if c1 != ' ' {
            self.add_character(c1, freq);
            // take first, first 2 etc chars of the trigram every time for the appropriate stat
            // as long as they don't contain spaces
            if c2 != ' ' {
                self.add_bigram([c1, c2], freq);
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

    fn save(&self, pass: bool) -> Result<()> {
        use std::fs::OpenOptions;
        use std::io::Write;

        let buf = Vec::new();
        let formatter = serde_json::ser::PrettyFormatter::with_indent(b"\t");
        let mut ser = serde_json::Serializer::with_formatter(buf, formatter);
        self.serialize(&mut ser).unwrap();

        let data_dir = format!("static/language_data{}", if pass { "_raw" } else { "" });

        if let Ok(t) = std::fs::try_exists(&data_dir) && !t {
            std::fs::create_dir_all(&data_dir)?;
        }

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(format!("{}/{}.json", data_dir, self.language))?;
        
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
