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
        .map(|chunk| std::str::from_utf8(chunk)
        .expect(
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
        let it = s.chars()
            .chain([' ', ' ', ' ', ' '])
            .tuple_windows::<(_, _, _, _, _)>()
            .map(|(c1, c2, c3, c4, c5)| [c1, c2, c3, c4, c5]);
        
        for q in it {
                quingrams.entry(q).and_modify(|f| *f += 1).or_insert(1);
            }
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

#[derive(Default)]
struct TextDataElem {
    c: Option<char>,
    b: Option<[char; 2]>,
    t: Option<[char; 3]>,
    s: Option<[char; 2]>,
    s2: Option<[char; 2]>,
    s3: Option<[char; 2]>,
    freq: f64
}

impl FromIterator<TextDataElem> for TextData {
    fn from_iter
        <T: IntoIterator<Item = TextDataElem>>
    (iter: T) -> Self {
        let mut res = Self::default();
        for elem in iter {
            let freq = elem.freq;
            if let Some(c) = elem.c { res.add_character(c, freq) }
            if let Some(b) = elem.b { res.add_bigram(b, freq) }
            if let Some(t) = elem.t { res.add_trigram(t, freq) }
            if let Some(s) = elem.s { res.add_skipgram(s, freq) }
            if let Some(s2) = elem.s2 { res.add_skipgram2(s2, freq) }
            if let Some(s3) = elem.s3 { res.add_skipgram3(s3, freq) }
        }

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



impl TextDataElem {
    fn from_n_subsequent<const N: usize>(ngram: [char; N], freq: f64) -> Self {
        let mut res = Self::default();
        res.freq = freq;

        if N > 0 && let c1 = ngram[0] && c1 != ' ' {
            res.c = Some(c1);
            // take first, first 2 etc chars of the trigram every time for the appropriate stat
            // as long as they don't contain spaces. return `c2` so I don't iter.next() too much
            let c2 = if N > 1 && let c2 = ngram[1] && c2 != ' ' {
                res.b = Some([c1, c2]);
                c2
            } else { ' ' };
            // c1 and c3 for skipgrams
            if N > 2 && let c3 = ngram[2] && c3 != ' ' {
                res.s = Some([c1, c3]);

                if c2 != ' ' {
                    res.t = Some([c1, c2, c3]);
                }

                if N > 3 && let c4 = ngram[3] && c4 != ' ' {
                    res.s2 = Some([c1, c4]);

                    if N > 4 && let c5 = ngram[4] && c5 != ' ' {
                        res.s3 = Some([c1, c5]);
                    }
                }
            }
        }
        res
    }

    // fn from_char_iter<const N: usize>(iter: impl IntoIterator<Item=char>, freq: f64) -> [Option<Self>; N] {
    //     let mut res = [None; N];

    //     res
    // }
}

impl From<(TextNgrams<5>, &str, Translator)> for TextData {
    fn from((ngrams, language, translator): (TextNgrams<5>, &str, Translator)) -> Self {
        let mut res = TextData::new(language);

        let x = ngrams.ngrams.into_iter()
            .map(|(quingram, freq)| {
                let translated = translator.translate_arr(&quingram);
                let first_char = quingram[0];
                
                let it_count = if let Some(f) = translator.table.get(&first_char) {
                    f.chars().count()
                } else { 1 };
                let mut chars = translated.chars();

                
            });
            
            // match translated.chars().count() {
            //         5.. => {
                        
            //             translated.chars()
            //                 .tuple_windows::<(_, _, _, _, _)>()
            //                 .take(it_count)
            //                 .for_each(|quin|
            //                     res.add_from_n_subsequent::<5>([quin.0, quin.1, quin.2, quin.3, quin.4], freq as f64)
            //                 );
            //         }
            //         4 => res.add_from_n_subsequent::<4>(
            //             Self::collect_str_into_arr::<4>(translated.as_str()), freq as f64
            //         ),
            //         3 => res.add_from_n_subsequent::<3>(
            //             Self::collect_str_into_arr::<3>(translated.as_str()), freq as f64
            //         ),
            //         2 => res.add_from_n_subsequent::<2>(
            //             Self::collect_str_into_arr::<2>(translated.as_str()), freq as f64
            //     ),
            //         1 => {
            //             let ngram = translated.chars().next().unwrap();
            //             TextDataElem::from_n_subsequent(ngram, freq)
            //         }
            //         _ => ArrayVec::new()
            //     }
        

        // IndexMaps have the property of keeping order based on insertion, so they're sortable:
        

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

// #[cfg(test)]
#[allow(unused)]
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

		let total_c = 1.0/data.characters.iter()
            .map(|&f| f)
            .reduce(f64::min).unwrap();
        
        assert_eq!(data.characters.get(data.convert_u8.to_single_lossy('e') as usize), Some(&(2.0/total_c)));
        assert_eq!(data.characters.get(data.convert_u8.to_single_lossy('\'') as usize), Some(&(1.0/total_c)));

        let total_b = 1.0/data.bigrams.iter().map(|(_, &f)| f).reduce(f64::min).unwrap();

        assert_eq!(data.bigrams.get(&data.convert_u8.to_bigram_lossy(['\'', '*'])), Some(&(1.0/total_b)));
        assert_eq!(data.bigrams.get(&data.convert_u8.to_bigram_lossy(['1', ':'])), None);

		let total_s = 1.0/data.skipgrams.iter().map(|(_, &f)| f).reduce(f64::min).unwrap();

		assert_eq!(data.skipgrams.get(&data.convert_u8.to_bigram_lossy([';', 'd'])), Some(&(1.0/total_s)));
		assert_eq!(data.skipgrams.get(&data.convert_u8.to_bigram_lossy(['*', 'e'])), Some(&(1.0/total_s)));
		assert_eq!(data.skipgrams.get(&data.convert_u8.to_bigram_lossy(['t', 'e'])), Some(&(1.0/total_s)));
		assert_eq!(data.skipgrams.get(&data.convert_u8.to_bigram_lossy(['\'', 't'])), None);
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