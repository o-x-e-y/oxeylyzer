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
    load_data(language, Translator::raw()).unwrap();
}

pub fn load_default(language: &str) {
    let translator = Translator::language_or_raw(language);
	if let Err(error) = load_data(language, translator) {
        println!("{language} failed to update: '{error}'");
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

    let all_trigrams = read_dir(format!("static/text/{language}"))?
        .filter_map(Result::ok)
        .map(|dir_entry| -> Result<TextNgrams<5>> {
            let f = File::open(dir_entry.path())?;
            TextNgrams::try_from(f)
        })
        .filter_map(Result::ok)
        .reduce(|accum, new| accum.combine_with(new))
        .unwrap_or(TextNgrams::default());

    let is_raw = translator.is_raw;
    let res = TextData::from((all_trigrams, translator, language));
    res.save(is_raw)?;
    println!("loading {} took {}ms", language, (Instant::now() - start_total).as_millis());
    Ok(res)
}

#[derive(Default, Debug)]
pub struct TextNgrams<const N: usize> {
    pub ngrams: HashMap<[char; N], usize>,
}

impl TryFrom<File> for TextNgrams<5> {
    type Error = anyhow::Error;

    fn try_from(f: File) -> Result<Self, Self::Error> {
        let thread_count = (f.metadata()?.len() / TWO_MB + 1).min(12);
        
        let chunker = FileChunker::new(&f)?;

        let ngrams = chunker.chunks(thread_count as usize, None)?
            .into_par_iter()
            .map(|chunk| {
                let text = String::from_utf8_lossy(chunk);
                TextNgrams::from(text.as_ref())
            })
            .reduce(
                || TextNgrams::default(),
                |accum, new| accum.combine_with(new)
            );
        Ok(ngrams)
    }
}

impl From<&str> for TextNgrams<5> {
    fn from(s: &str) -> Self {
        let mut ngrams: HashMap<[char; 5], usize> = HashMap::new();

        let mut chars = s.chars().chain("    ".chars())
            .tuple_windows::<(_, _, _, _, _)>();

        while let Some((c1, c2, c3, c4, c5)) = chars.next() {
            ngrams.entry([c1, c2, c3, c4, c5]).and_modify(|e| *e += 1).or_insert(1);
        }
        
        Self { ngrams }
    }
}

impl<const N: usize> TextNgrams<N> {
    fn combine_with(mut self, rhs: Self) -> Self {
        for (trigram, freq) in rhs.ngrams.into_iter() {
            self.ngrams.entry(trigram).and_modify(|e| *e += freq).or_insert(freq);
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
    skipgrams2: IndexMap<String, f64>,
    skipgrams3: IndexMap<String, f64>,
    trigrams: IndexMap<String, f64>,

    #[serde(skip)]
    char_sum: f64,
    #[serde(skip)]
    bigram_sum: f64,
    #[serde(skip)]
    skipgram_sum: f64,
    #[serde(skip)]
    skipgram2_sum: f64,
    #[serde(skip)]
    skipgram3_sum: f64,
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
\"skipgrams2\": {:#?},
\"skipgrams3\": {:#?},
\"trigrams\": {:#?}
}}",
            self.language,
            self.characters,
            self.bigrams,
            self.skipgrams,
            self.skipgrams2,
            self.skipgrams3,
            self.trigrams
        )
    }
}

impl From<(TextNgrams<5>, Translator, &str)> for TextData {
    fn from((data, translator, language): (TextNgrams<5>, Translator, &str)) -> Self {
        let mut res = TextData::default();
        res.language = language.replace(" ", "_");

        for (pentagram, freq) in data.ngrams.into_iter() {
            let s: SmartString<Compact> = SmartString::from_iter(pentagram);
            let translated = translator.translate(s.as_str());
            
            let mut it_count = if let Some(f) = translator.table.get(&pentagram[0]) {
                f.chars().count()
            } else { 0 };

            match translated.chars().count() {
                5.. => {
                    let mut it = translated.chars()
                        .tuple_windows::<(_, _, _, _, _)>();

                    while let Some(p) = it.next() && it_count > 0 {
                        let pentagram = [p.0, p.1, p.2, p.3, p.4];
                        res.add_from_n_subsequent::<5>(pentagram, freq as f64);
                        it_count -= 1;
                    }
                },
                4 => {
                    let q: (char, char, char, char) = translated.chars().collect_tuple().unwrap();
                    let quadgram = [q.0, q.1, q.2, q.3];
                    res.add_from_n_subsequent::<4>(quadgram, freq as f64);
                },
                3 => {
                    let t: (char, char, char) = translated.chars().collect_tuple().unwrap();
                    let trigram = [t.0, t.1, t.2];
                    res.add_from_n_subsequent::<3>(trigram, freq as f64);
                },
                2 => {
                    let b: (char, char) = translated.chars().collect_tuple().unwrap();
                    let bigram = [b.0, b.1];
                    res.add_from_n_subsequent::<2>(bigram, freq as f64);
                }
                1 => {
                    let c1 = translated.chars().next().unwrap();
                    res.add_character(c1, freq as f64);
                },
                _ => {}
            }
        }

        // IndexMaps have the property of being sorted based on insertion, so they're sortable:
        res.characters.iter_mut().for_each(|(_, f)| *f /= res.char_sum);
        res.bigrams.iter_mut().for_each(|(_, f)| *f /= res.bigram_sum);
        res.skipgrams.iter_mut().for_each(|(_, f)| *f /= res.skipgram_sum);
        res.skipgrams2.iter_mut().for_each(|(_, f)| *f /= res.skipgram2_sum);
        res.skipgrams3.iter_mut().for_each(|(_, f)| *f /= res.skipgram3_sum);
        res.trigrams.iter_mut().for_each(|(_, f)| *f /= res.trigram_sum);
        
        res.characters.sort_by(|_, f1, _, f2| f2.partial_cmp(f1).unwrap());
        res.bigrams.sort_by(|_, f1, _, f2| f2.partial_cmp(f1).unwrap());
        res.skipgrams.sort_by(|_, f1, _, f2| f2.partial_cmp(f1).unwrap());
        res.skipgrams2.sort_by(|_, f1, _, f2| f2.partial_cmp(f1).unwrap());
        res.skipgrams3.sort_by(|_, f1, _, f2| f2.partial_cmp(f1).unwrap());
        res.trigrams.sort_by(|_, f1, _, f2| f2.partial_cmp(f1).unwrap());

        res
    }
}

impl TextData {
    fn add_from_n_subsequent<const N: usize>(&mut self, ngram: [char; N], freq: f64) {
        if N > 0 && let c1 = ngram[0] && c1 != ' ' {
            self.add_character(c1, freq);
            // take first, first 2 etc chars of the trigram every time for the appropriate stat
            // as long as they don't contain spaces
            if N > 1 && let c2 = ngram[1] && c2 != ' ' {
                self.add_bigram([c1, c2], freq);

                if N > 2 && let c3 = ngram[2] && c3 != ' ' {
                    self.add_trigram([c1, c2, c3], freq);
                }
            }
            // c1 and c3 for skipgrams
            if N > 2 && let c3 = ngram[2] && c3 != ' ' {
                self.add_skipgram([c1, c3], freq);

                if N > 3 && let c4 = ngram[3] && c4 != ' ' {
                    self.add_skipgram2([c1, c4], freq);

                    if N > 4 && let c5 = ngram[4] && c5 != ' ' {
                        self.add_skipgram3([c1, c5], freq);
                    }
                }
            }
        }
    }

    pub(crate) fn add_character(&mut self, c1: char, freq: f64) {
        self.characters.entry(String::from(c1))
            .and_modify(|e| *e += freq).or_insert(freq);
        self.char_sum += freq;
    }

    pub(crate) fn add_bigram(&mut self, bigram: [char; 2], freq: f64) {
        self.bigrams.entry(String::from_iter(bigram))
            .and_modify(|e| *e += freq).or_insert(freq);
        self.bigram_sum += freq;
    }

    pub(crate) fn add_skipgram(&mut self, skipgram: [char; 2], freq: f64) {
        self.skipgrams.entry(String::from_iter(skipgram))
            .and_modify(|e| *e += freq).or_insert(freq);
        self.skipgram_sum += freq;
    }

    pub(crate) fn add_skipgram2(&mut self, skipgram: [char; 2], freq: f64) {
        self.skipgrams2.entry(String::from_iter(skipgram))
            .and_modify(|e| *e += freq).or_insert(freq);
        self.skipgram2_sum += freq;
    }

    pub(crate) fn add_skipgram3(&mut self, skipgram: [char; 2], freq: f64) {
        self.skipgrams3.entry(String::from_iter(skipgram))
            .and_modify(|e| *e += freq).or_insert(freq);
        self.skipgram3_sum += freq;
    }

    pub(crate) fn add_trigram(&mut self, trigram: [char; 3], freq: f64) {
        self.trigrams.entry(String::from_iter(trigram))
            .and_modify(|e| *e += freq).or_insert(freq);
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
    use crate::*;

	#[test]
	fn load_language_data() {
        use language_data::*;

		load_default("test");

		let data = LanguageData::from_file("static/language_data","test")
			.expect("'test.json' in static/language_data/ was not created");
		
		assert!(data.language == "test");

		let total_c = 1.0/data.characters.iter().map(|&(_, f)| f).reduce(f64::min).unwrap();
        
        assert_eq!(data.characters.get(&'e'), Some(&(2.0/total_c)));
        assert_eq!(data.characters.get(&'\''), Some(&(1.0/total_c)));

        let total_b = 1.0/data.bigrams.iter().map(|(_, &f)| f).reduce(f64::min).unwrap();

        assert_eq!(data.bigrams.get(&['\'', '*']), Some(&(1.0/total_b)));
        assert_eq!(data.bigrams.get(&['1', ':']), None);

		let total_s = 1.0/data.skipgrams.iter().map(|(_, &f)| f).reduce(f64::min).unwrap();

		assert_eq!(data.skipgrams.get(&[';', 'd']), Some(&(1.0/total_s)));
		assert_eq!(data.skipgrams.get(&['*', 'e']), Some(&(1.0/total_s)));
		assert_eq!(data.skipgrams.get(&['t', 'e']), Some(&(1.0/total_s)));
		assert_eq!(data.skipgrams.get(&['\'', 't']), None);
	}

	#[test]
	fn get_generator() {

		let a = generate::LayoutGeneration::new("test", "static", 1000, None);
		assert!(a.is_ok());
	}
}
