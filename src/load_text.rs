use crate::translation::Translator;

use std::collections::HashMap;
use std::iter::FromIterator;
use std::fs::{File, read_dir};
use std::time::Instant;

use itertools::Itertools;
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

    let quingrams = chunkers.par_iter()
        .flat_map(|(chunker, count)| {
            chunker.chunks(*count, Some(' ')).unwrap()
        })
        .map(|chunk| std::str::from_utf8(chunk).expect(
                "one of the files provided is not encoded as utf-8.\
                Make sure all files in the directory are valid utf-8."
            )
        )
        .map(|s|
            TextNgrams::<5>::from(s)
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
pub struct TextNgrams<const N: usize> {
    pub ngrams: HashMap<[char; N], usize>,
}

impl From<&str> for TextNgrams<5> {
    fn from(s: &str) -> Self {
        let mut quingrams = HashMap::new();
        let it1 = s.chars();
        let it2 = s.chars().skip(1).chain([' ']);
        let it3 = s.chars().skip(2).chain([' ', ' ']);
        let it4 = s.chars().skip(3).chain([' ', ' ', ' ']);
        let it5 = s.chars().skip(4).chain([' ', ' ', ' ', ' ']);
        let it = it1.zip(it2).zip(it3).zip(it4).zip(it5);
        for ((((c1, c2), c3), c4), c5) in it {
            let q = [c1, c2, c3, c4, c5];
            println!("{q:?}");
            quingrams.entry([c1, c2, c3, c4, c5]).and_modify(|f| *f += 1).or_insert(1);
        }
        // for q in s.chars()
        //     .chain([' ', ' ', ' ', ' '])
        //     .tuple_windows::<(_, _, _, _, _)>()
        //     .map(|(c1, c2, c3, c4, c5)| [c1, c2, c3, c4, c5]) {
        //         quingrams.entry(q).and_modify(|f| *f += 1).or_insert(1);
        //     }
        Self { ngrams: quingrams }
    }
}

impl<const N: usize> TextNgrams<N> {
    fn combine_with(mut self, rhs: Self) -> Self {
        for (trigram, freq) in rhs.ngrams.into_iter() {
            self.ngrams.entry(trigram).and_modify(|f| *f += freq).or_insert(freq);
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

impl From<(TextNgrams<5>, &str, Translator)> for TextData {
    fn from((ngrams, language, translator): (TextNgrams<5>, &str, Translator)) -> Self {
        let mut res = TextData::new(language);

        for (quingram, freq) in ngrams.ngrams.into_iter() {
            let q = SmartString::<LazyCompact>::from_iter(quingram);
            let translated = translator.translate(q.as_str());
            let first_char = quingram[0];
            
            let it_count = if let Some(f) = translator.table.get(&first_char) {
                f.chars().count()
            } else { 1 };

            match translated.chars().count() {
                5.. => {
                    translated.chars()
                        .tuple_windows::<(_, _, _, _, _)>()
                        .take(it_count)
                        .for_each(|quin|
                            res.add_from_n_subsequent::<5>([quin.0, quin.1, quin.2, quin.3, quin.4], freq as f64)
                        );
                }
                4 => res.add_from_n_subsequent::<4>(
                    Self::collect_str_into_arr::<4>(translated.as_str()), freq as f64
                ),
                3 => res.add_from_n_subsequent::<3>(
                    Self::collect_str_into_arr::<3>(translated.as_str()), freq as f64
                ),
                2 => res.add_from_n_subsequent::<2>(
                    Self::collect_str_into_arr::<2>(translated.as_str()), freq as f64
            ),
                1 => {
                    let c1 = translated.chars().next().unwrap();
                    res.add_character(c1, freq as f64);
                }
                _ => {}
            }
        }

        // IndexMaps have the property of keeping order based on insertion, so they're sortable:
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
    fn collect_str_into_arr<const N: usize>(string: &str) -> [char; N] {
        let mut res = [' '; N];
        for (i, c) in string.chars().enumerate() {
            res[i] = c;
        }
        res
    }

    fn add_from_n_subsequent<const N: usize>(&mut self, ngram: [char; N], freq: f64) {
        if N > 0 && let c1 = ngram[0] && c1 != ' ' {
            self.add_character(c1, freq);
            // take first, first 2 etc chars of the trigram every time for the appropriate stat
            // as long as they don't contain spaces. return `c2` so I don't iter.next() too much
            let c2 = if N > 1 && let c2 = ngram[1] && c2 != ' ' {
                self.add_bigram([c1, c2], freq);
                c2
            } else { ' ' };
            // c1 and c3 for skipgrams
            if N > 2 && let c3 = ngram[2] && c3 != ' ' {
                self.add_skipgram([c1, c3], freq);

                if c2 != ' ' { self.add_trigram([c1, c2, c3], freq); }

                if N > 3 && let c4 = ngram[3] && c4 != ' ' {
                    self.add_skipgram2([c1, c4], freq);

                    if N > 4 && let c5 = ngram[4] && c5 != ' ' {
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
        ngrams.ngrams.insert(['A', 'm', 'o', 'g', 'u'], 1);
        ngrams.ngrams.insert(['m', 'o', 'g', 'u', 's'], 1);
        ngrams.ngrams.insert(['o', 'g', 'u', 's', ' '], 1);
        ngrams.ngrams.insert(['g', 'u', 's', ' ', ' '], 1);
        ngrams.ngrams.insert(['u', 's', ' ', ' ', ' '], 1);
        ngrams.ngrams.insert(['s', ' ', ' ', ' ', ' '], 1);
        let translator = Translator::new()
            .letters_to_lowercase("amogus")
            .build();
        let data = TextData::from((ngrams, "among", translator));
        
        assert_eq!(data.char_sum, 6.0,);
        assert_eq!(data.bigram_sum, 5.0,);
        assert_eq!(data.skipgram_sum, 4.0,);
        assert_eq!(data.skipgram2_sum, 3.0,);
        assert_eq!(data.skipgram3_sum, 2.0,);
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
