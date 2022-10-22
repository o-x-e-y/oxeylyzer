use crate::translation::Translator;

use std::collections::HashMap;
use std::iter::FromIterator;
use std::fs::{File, read_dir};
use std::time::Instant;

use rayon::iter::{ParallelIterator, IntoParallelRefIterator};
use file_chunker::FileChunker;
use anyhow::Result;
use indexmap::IndexMap;
use serde::{Serialize, Deserialize};
use smartstring::{LazyCompact, SmartString};

const FOUR_MB: u64 = 1024 * 1024 * 4;

pub fn load_raw(language: &str) {
    load_data(language, Translator::raw(true)).unwrap();
}

#[allow(dead_code)]
pub(crate) fn load_default(language: &str) {
    let translator = Translator::language_or_raw(language);
	if let Err(error) = load_data(language, translator) {
        println!("{language} failed to update: '{error}'");
    }
}

#[allow(dead_code)]
pub(crate) fn load_all_default() -> Result<()> {
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

pub fn load_data(language: &str, translator: Translator) -> Result<()> {
    let start_total = Instant::now();
    let is_raw = translator.is_raw;

    let chunkers = read_dir(format!("static/text/{language}"))?
        .filter_map(Result::ok)
        .flat_map(|dir_entry| File::open(dir_entry.path()))
        .map(|f| {
            let len = f.metadata().unwrap().len() + 1;
            let count = if len > FOUR_MB { len / FOUR_MB } else { 1 };
            (FileChunker::new(&f).unwrap(), count as usize)
        })
        .collect::<Vec<_>>();
    
    let texts = chunkers.par_iter()
        .flat_map(|(chunker, count)| {
            chunker.chunks(*count, Some(' ')).unwrap()
        })
        .map(|chunk| std::str::from_utf8(chunk).expect(
                "one of the files provided is not encoded as utf-8.\
                Make sure all files in the directory are valid utf-8."
            )
        )
        .map(|s| {
            let mut buf = [' '; 5];
            for (i, c) in s.chars().rev().take(5).enumerate() {
                buf[4 - i] = c;
            }
            (s, SmartString::<LazyCompact>::from_iter(buf) + "     ")
        })
        .collect::<Vec<_>>();

        let quingrams = texts.par_iter()
            .map(|(s, last5)|
                TextNgrams::<5>::from((*s, last5.as_str()))
            )
            .reduce(
                || TextNgrams::default(),
                |accum, new| accum.combine_with(new)
        );

    TextData::from((quingrams, language, translator)).save(is_raw)?;
    println!("loading {} took {}ms", language, ((Instant::now() - start_total) * 100).as_millis());

    Ok(())
}

#[derive(Default, Debug)]
pub struct TextNgrams<'a, const N: usize> {
    pub ngrams: HashMap<&'a str, usize>,
}

impl<'a, const N: usize> From<(&'a str, &'a str)> for TextNgrams<'a, N> {
    fn from((s, s_end): (&'a str, &'a str)) -> Self {
        let mut pentagrams = HashMap::new();

        let start_i = s.char_indices()
            .map(|(i, _)| i);
        let end_i = s.char_indices()
            .skip(N)
            .map(|(i, _)| i);
        
        let iter_first = start_i.zip(end_i)
            .map(|(i1, i2)| &s[i1..i2]);

        let start_i = s_end.char_indices()
            .map(|(i, _)| i);
        let end_i = s_end.char_indices()
            .skip(N)
            .map(|(i, _)| i);

        let iter = iter_first.chain(
            start_i.zip(end_i)
                .map(|(i1, i2)| &s_end[i1..i2])
        );

        for s in iter {
            pentagrams.entry(s).and_modify(|p| *p += 1).or_insert(1);
        }
        
        Self { ngrams: pentagrams }
    }
}

impl<'a, const N: usize> TextNgrams<'a, N> {
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

    characters: IndexMap<char, f64>,
    bigrams: IndexMap<SmartString<LazyCompact>, f64>,
    skipgrams: IndexMap<SmartString<LazyCompact>, f64>,
    skipgrams2: IndexMap<SmartString<LazyCompact>, f64>,
    skipgrams3: IndexMap<SmartString<LazyCompact>, f64>,
    trigrams: IndexMap<SmartString<LazyCompact>, f64>,

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
            "{{\
                \"language\": {},\
                \"characters\": {:#?},\
                \"bigrams\": {:#?},\
                \"skipgrams\": {:#?},\
                \"skipgrams2\": {:#?},\
                \"skipgrams3\": {:#?},\
                \"trigrams\": {:#?}\
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

impl TextData {
    pub fn new(language: &str) -> Self {
        let mut res = Self::default();
        res.language = language.replace(" ", "_").to_lowercase().to_string();
        res
    }
}

impl From<(TextNgrams<'_, 5>, &str, Translator)> for TextData {
    fn from((ngrams, language, translator): (TextNgrams<5>, &str, Translator)) -> Self {
        let mut res = TextData::new(language);

        for (quingram, freq) in ngrams.ngrams.into_iter() {
            let mut translated = translator.translate(quingram);
            let first_char = quingram.chars().next().unwrap();
            
            let it_count = if let Some(f) = translator.table.get(&first_char) {
                f.chars().count()
            } else { 1 };

            match translated.chars().count() {
                5.. => {
                    translated.push(' ');
                    let start_i = translated.char_indices()
                        .map(|(i, _)| i);
                    let end_i = translated.char_indices()
                        .skip(5)
                        .map(|(i, _)| i);
                    
                    start_i.zip(end_i)
                        .map(|(i1, i2)| &translated[i1..i2])
                        .take(it_count)
                        .for_each(|quin| res.add_from_n_subsequent::<5>(quin, freq as f64));
                }
                4 => res.add_from_n_subsequent::<4>(translated.as_str(), freq as f64),
                3 => res.add_from_n_subsequent::<3>(translated.as_str(), freq as f64),
                2 => res.add_from_n_subsequent::<2>(translated.as_str(), freq as f64),
                1 => {
                    let c1 = translated.chars().next().unwrap();
                    res.add_character(c1, freq as f64);
                }
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
    fn add_from_n_subsequent<const N: usize>(&mut self, ngram: &str, freq: f64) {
        let mut iter = ngram.chars();
        if N > 0 && let c1 = iter.next().unwrap() && c1 != ' ' {
            self.add_character(c1, freq);
            // take first, first 2 etc chars of the trigram every time for the appropriate stat
            // as long as they don't contain spaces. return `c2` so I don't iter.next() too much
            let c2 = if N > 1 && let c2 = iter.next().unwrap() && c2 != ' ' {
                self.add_bigram([c1, c2], freq);
                c2
            } else { ' ' };
            // c1 and c3 for skipgrams
            if N > 2 && let c3 = iter.next().unwrap() && c3 != ' ' {
                self.add_skipgram([c1, c3], freq);

                if c2 != ' ' {
                    self.add_trigram([c1, c2, c3], freq);
                }

                if N > 3 && let c4 = iter.next().unwrap() && c4 != ' ' {
                    self.add_skipgram2([c1, c4], freq);

                    if N > 4 && let c5 = iter.next().unwrap() && c5 != ' ' {
                        self.add_skipgram3([c1, c5], freq);
                    }
                }
            }
        }
    }

    pub(crate) fn add_character(&mut self, c: char, freq: f64) {
        self.characters.entry(c)
            .and_modify(|e| *e += freq).or_insert(freq);
        self.char_sum += freq;
    }

    pub(crate) fn add_bigram(&mut self, bigram: [char; 2], freq: f64) {
        self.bigrams.entry(SmartString::from_iter(bigram))
            .and_modify(|e| *e += freq).or_insert(freq);
        self.bigram_sum += freq;
    }

    pub(crate) fn add_skipgram(&mut self, skipgram: [char; 2], freq: f64) {
        self.skipgrams.entry(SmartString::from_iter(skipgram))
            .and_modify(|e| *e += freq).or_insert(freq);
        self.skipgram_sum += freq;
    }

    pub(crate) fn add_skipgram2(&mut self, skipgram: [char; 2], freq: f64) {
        self.skipgrams2.entry(SmartString::from_iter(skipgram))
            .and_modify(|e| *e += freq).or_insert(freq);
        self.skipgram2_sum += freq;
    }

    pub(crate) fn add_skipgram3(&mut self, skipgram: [char; 2], freq: f64) {
        self.skipgrams3.entry(SmartString::from_iter(skipgram))
            .and_modify(|e| *e += freq).or_insert(freq);
        self.skipgram3_sum += freq;
    }

    pub(crate) fn add_trigram(&mut self, trigram: [char; 3], freq: f64) {
        self.trigrams.entry(SmartString::from_iter(trigram))
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
    use crate::{*, utility::ApproxEq};

    #[test]
    fn from_textngrams() {
        let mut ngrams = TextNgrams::<5>::default();
        ngrams.ngrams.insert("Amogu", 1);
        ngrams.ngrams.insert("mogus", 1);
        ngrams.ngrams.insert("ogus", 1);
        ngrams.ngrams.insert("gus", 1);
        ngrams.ngrams.insert("us", 1);
        ngrams.ngrams.insert("s", 1);
        let translator = Translator::new()
            .letters_to_lowercase("amogus")
            .build();
        let data = TextData::from((ngrams, "among", translator));
        
        assert_eq!(data.char_sum, 6.0,);
        assert_eq!(data.bigram_sum, 5.0,);
        assert_eq!(data.skipgram_sum, 4.0,);
        assert_eq!(data.skipgram2_sum, 3.0,);
        assert_eq!(data.skipgram3_sum, 2.0,);
        assert_eq!(data.trigram_sum, 4.0);
        assert_eq!(data.trigram_sum, data.skipgram_sum);

        for (_, f) in data.characters {
            assert!(f.approx_eq_dbg(1.0/6.0, 15));
        }
        for (_, f) in data.bigrams {
            assert!(f.approx_eq_dbg(1.0/5.0, 15));
        }
        for (_, f) in data.skipgrams {
            assert!(f.approx_eq_dbg(1.0/4.0, 15));
        }
        for (_, f) in data.skipgrams2 {
            assert!(f.approx_eq_dbg(1.0/3.0, 15));
        }
        for (_, f) in data.skipgrams3 {
            assert!(f.approx_eq_dbg(1.0/2.0, 15));
        }
        for (_, f) in data.trigrams {
            assert!(f.approx_eq_dbg(1.0/4.0, 15));
        }
    }

    #[test]
    fn test() {
        let s = "1: d'Être";
        let n = 5;

        let start_i = s.char_indices()
            .map(|(i, _)| i);
        let end_i = s.char_indices()
            .skip(n)
            .map(|(i, _)| i);
        
        let iter_first = start_i.zip(end_i)
            .map(|(i1, i2)| &s[i1..i2]);

        let mut buf = [' '; 5];
        for (i, c) in s.chars().rev().take(5).enumerate() {
            buf[4 - i] = c;
        }
        let s_end = String::from_iter(buf) + "     ";

        let start_i = s_end.char_indices()
            .map(|(i, _)| i);
        let end_i = s_end.char_indices()
            .skip(n)
            .map(|(i, _)| i);

        let iter = iter_first.chain(
            start_i.zip(end_i)
                .map(|(i1, i2)| &s_end[i1..i2])
        );
        for s in iter {
            println!("str: '{s}'");
        }
    }

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
		let a = generate::LayoutGeneration::new(
            "test",
            "static",
            None,
        );
		assert!(a.is_ok());
	}
}
