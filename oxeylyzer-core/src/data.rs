use indexmap::IndexMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, serde_conv};

use fxhash::FxHashMap as HashMap;

use crate::{corpus_cleaner::CorpusCleanerIterator, OxeylyzerError, REPLACEMENT_CHAR};

#[cfg(not(target_arch = "wasm32"))]
mod exclude_wasm {
    pub use std::{
        fs::{File, OpenOptions},
        io::Write,
        path::Path,
    };

    pub use file_chunker::FileChunker;
    pub use rayon::prelude::*;
    pub use serde_json::ser::PrettyFormatter;

    pub use crate::corpus_cleaner::{CleanCorpus, CorpusCleaner};

    pub const CHUNK_SIZE: usize = 1024 * 1024;
}

#[cfg(not(target_arch = "wasm32"))]
use exclude_wasm::*;

#[cfg(target_arch = "wasm32")]
use gloo_net::http::Request;

serde_conv!(
    BigramAsStr,
    [char; 2],
    |trigram: &[char; 2]| String::from_iter(trigram),
    |value: String| -> Result<_, OxeylyzerError> {
        value
            .chars()
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|v: Vec<_>| OxeylyzerError::InvalidBigramLength(v.len()))
    }
);

serde_conv!(
    TrigramAsStr,
    [char; 3],
    |trigram: &[char; 3]| String::from_iter(trigram),
    |value: String| -> Result<_, OxeylyzerError> {
        value
            .chars()
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|v: Vec<_>| OxeylyzerError::InvalidTrigramLength(v.len()))
    }
);

#[serde_as]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(into = "SaveData")]
pub struct Data {
    pub name: String,

    pub chars: HashMap<char, f64>,
    #[serde_as(as = "HashMap<BigramAsStr, _>")]
    pub bigrams: HashMap<[char; 2], f64>,
    #[serde_as(as = "HashMap<BigramAsStr, _>")]
    pub skipgrams: HashMap<[char; 2], f64>,
    #[serde_as(as = "HashMap<TrigramAsStr, _>")]
    pub trigrams: HashMap<[char; 3], f64>,

    pub char_total: i64,
    pub bigram_total: i64,
    pub skipgram_total: i64,
    pub trigram_total: i64,
}

impl Data {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_char(&self, c: char) -> Option<&f64> {
        self.chars.get(&c)
    }

    pub fn get_bigram(&self, bigram: [char; 2]) -> Option<&f64> {
        self.bigrams.get(&bigram)
    }

    pub fn get_skipgram(&self, skipgram: [char; 2]) -> Option<&f64> {
        self.skipgrams.get(&skipgram)
    }

    pub fn get_trigram(&self, trigram: [char; 3]) -> Option<&f64> {
        self.trigrams.get(&trigram)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Data {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, OxeylyzerError> {
        let content = std::fs::read_to_string(path)?;
        let data = serde_json::from_str::<Self>(&content)?;
        Ok(data)
    }

    pub fn from_path<P: AsRef<Path>>(
        path: P,
        name: &str,
        cleaner: &CorpusCleaner,
    ) -> Result<Self, OxeylyzerError> {
        if path.as_ref().is_file() {
            let f = std::fs::File::open(path)?;
            Self::from_file(f, name, cleaner)
        } else if path.as_ref().is_dir() {
            let mut new = std::fs::read_dir(path)?
                .flatten()
                .par_bridge()
                .filter(|entry| entry.path().is_file())
                .flat_map(|entry| {
                    let f = std::fs::File::open(entry.path())?;
                    IntermediateData::from_file(f, name, cleaner)
                })
                .reduce(IntermediateData::default, |a, b| a + b);

            new.name = name.to_string();

            Ok(new.into())
        } else {
            Err(OxeylyzerError::NotAFile)
        }
    }

    pub fn from_file(
        file: File,
        name: &str,
        cleaner: &CorpusCleaner,
    ) -> Result<Data, OxeylyzerError> {
        IntermediateData::from_file(file, name, cleaner).map(Into::into)
    }

    pub fn save<P: AsRef<Path>>(&self, folder: P) -> Result<(), OxeylyzerError> {
        if self.name.is_empty() {
            return Err(OxeylyzerError::MissingDataName);
        }

        std::fs::create_dir_all(&folder)?;

        let path = folder.as_ref().join(&self.name).with_extension("json");

        let mut f = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(path)?;

        let formatter = PrettyFormatter::with_indent(b"\t");
        let mut ser = serde_json::ser::Serializer::with_formatter(vec![], formatter);
        self.serialize(&mut ser)?;

        f.write_all(ser.into_inner().as_slice())?;

        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
impl Data {
    pub async fn load(url: &str) -> Result<Self, OxeylyzerError> {
        let data = Request::get(url).send().await?.json::<Self>().await?;
        Ok(data)
    }
}

#[serde_as]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct SaveData {
    pub name: String,

    pub char_total: i64,
    pub bigram_total: i64,
    pub skipgram_total: i64,
    pub trigram_total: i64,

    pub chars: IndexMap<char, f64>,
    pub bigrams: IndexMap<String, f64>,
    pub skipgrams: IndexMap<String, f64>,
    pub trigrams: IndexMap<String, f64>,
}

impl From<Data> for SaveData {
    fn from(data: Data) -> Self {
        let chars = data
            .chars
            .into_iter()
            .sorted_by(|(_, f1), (_, f2)| f2.total_cmp(f1))
            .collect();

        let bigrams = data
            .bigrams
            .into_iter()
            .sorted_by(|(_, f1), (_, f2)| f2.total_cmp(f1))
            .map(|(b, f)| (String::from_iter(b), f))
            .collect();

        let skipgrams = data
            .skipgrams
            .into_iter()
            .sorted_by(|(_, f1), (_, f2)| f2.total_cmp(f1))
            .map(|(b, f)| (String::from_iter(b), f))
            .collect();

        let trigrams = data
            .trigrams
            .into_iter()
            .sorted_by(|(_, f1), (_, f2)| f2.total_cmp(f1))
            .map(|(b, f)| (String::from_iter(b), f))
            .collect();

        Self {
            name: data.name,

            char_total: data.char_total,
            bigram_total: data.bigram_total,
            skipgram_total: data.skipgram_total,
            trigram_total: data.trigram_total,

            chars,
            bigrams,
            skipgrams,
            trigrams,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct IntermediateData {
    pub name: String,
    pub chars: HashMap<char, i64>,
    pub bigrams: HashMap<[char; 2], i64>,
    pub skipgrams: HashMap<[char; 2], i64>,
    pub trigrams: HashMap<[char; 3], i64>,
}

impl IntermediateData {
    fn add_char(&mut self, c: char) {
        self.chars.entry(c).and_modify(|f| *f += 1).or_insert(1);
    }

    fn add_bigram(&mut self, c1: char, c2: char) {
        self.bigrams
            .entry([c1, c2])
            .and_modify(|f| *f += 1)
            .or_insert(1);
    }

    fn add_skipgram(&mut self, c1: char, c2: char) {
        self.skipgrams
            .entry([c1, c2])
            .and_modify(|f| *f += 1)
            .or_insert(1);
    }

    fn add_trigram(&mut self, c1: char, c2: char, c3: char) {
        self.trigrams
            .entry([c1, c2, c3])
            .and_modify(|f| *f += 1)
            .or_insert(1);
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl IntermediateData {
    fn from_file(file: File, name: &str, cleaner: &CorpusCleaner) -> Result<Self, OxeylyzerError> {
        let chunker = FileChunker::new(&file).map_err(|_| OxeylyzerError::ChunkerInitError)?;

        let file_len = file.metadata()?.len() as usize;
        let chunk_count = (file_len / CHUNK_SIZE).clamp(1, num_cpus::get());

        let chunks = chunker
            .chunks(chunk_count, Some(' '))
            .map_err(|_| OxeylyzerError::ChunkerChunkError)?;

        let mut intermediate = chunks
            .into_par_iter()
            .flat_map(|chunk| std::str::from_utf8(chunk))
            .map(|s| {
                s.chars()
                    .clean_corpus(cleaner)
                    .flatten()
                    .collect::<IntermediateData>()
            })
            .reduce(IntermediateData::default, |a, b| a + b);

        intermediate.name = name.into();

        Ok(intermediate)
    }
}

impl std::ops::Add for IntermediateData {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        for (c, freq) in rhs.chars.into_iter() {
            self.chars
                .entry(c)
                .and_modify(|f| *f += freq)
                .or_insert(freq);
        }

        for (bigram, freq) in rhs.bigrams.into_iter() {
            self.bigrams
                .entry(bigram)
                .and_modify(|f| *f += freq)
                .or_insert(freq);
        }

        for (skipgram, freq) in rhs.skipgrams.into_iter() {
            self.skipgrams
                .entry(skipgram)
                .and_modify(|f| *f += freq)
                .or_insert(freq);
        }

        for (trigram, freq) in rhs.trigrams.into_iter() {
            self.trigrams
                .entry(trigram)
                .and_modify(|f| *f += freq)
                .or_insert(freq);
        }

        self
    }
}

impl From<IntermediateData> for Data {
    fn from(data: IntermediateData) -> Self {
        let char_total = data.chars.values().sum::<i64>();
        let bigram_total = data.bigrams.values().sum::<i64>();
        let skipgram_total = data.skipgrams.values().sum::<i64>();
        let trigram_total = data.trigrams.values().sum::<i64>();

        let char_total_f = char_total as f64 / 100.0;
        let bigram_total_f = bigram_total as f64 / 100.0;
        let skipgram_total_f = skipgram_total as f64 / 100.0;
        let trigram_total_f = trigram_total as f64 / 100.0;

        let chars = data
            .chars
            .into_iter()
            .map(|(c, f)| (c, f as f64 / char_total_f))
            .collect();

        let bigrams = data
            .bigrams
            .into_iter()
            .map(|(c, f)| (c, f as f64 / bigram_total_f))
            .collect();

        let skipgrams = data
            .skipgrams
            .into_iter()
            .map(|(c, f)| (c, f as f64 / skipgram_total_f))
            .collect();

        let trigrams = data
            .trigrams
            .into_iter()
            .map(|(c, f)| (c, f as f64 / trigram_total_f))
            .collect();

        Self {
            name: data.name,
            chars,
            bigrams,
            skipgrams,
            trigrams,
            char_total,
            bigram_total,
            skipgram_total,
            trigram_total,
        }
    }
}

impl FromIterator<char> for IntermediateData {
    fn from_iter<T: IntoIterator<Item = char>>(iter: T) -> Self {
        let mut res = Self::default();
        let mut iter = iter.into_iter();

        if let Some(mut c1) = iter.next() {
            if c1 != REPLACEMENT_CHAR {
                res.add_char(c1);
            }

            if let Some(mut c2) = iter.next() {
                if c2 != REPLACEMENT_CHAR {
                    res.add_char(c2);

                    if c1 != REPLACEMENT_CHAR {
                        res.add_bigram(c1, c2);
                    }
                }

                for c3 in iter {
                    match (
                        c1 != REPLACEMENT_CHAR,
                        c2 != REPLACEMENT_CHAR,
                        c3 != REPLACEMENT_CHAR,
                    ) {
                        (false, false, true) => {
                            res.add_char(c3);
                        }
                        (false, true, true) => {
                            res.add_char(c3);
                            res.add_bigram(c2, c3);
                        }
                        (true, false, true) => {
                            res.add_char(c3);
                            res.add_skipgram(c1, c3);
                        }
                        (true, true, true) => {
                            res.add_char(c3);
                            res.add_bigram(c2, c3);
                            res.add_skipgram(c1, c3);
                            res.add_trigram(c1, c2, c3);
                        }
                        _ => {}
                    }

                    c1 = c2;
                    c2 = c3;
                }
            }
        }

        res
    }
}

impl FromIterator<char> for Data {
    fn from_iter<T: IntoIterator<Item = char>>(iter: T) -> Self {
        iter.into_iter().collect::<IntermediateData>().into()
    }
}

impl<'a> FromIterator<&'a char> for Data {
    fn from_iter<T: IntoIterator<Item = &'a char>>(iter: T) -> Self {
        iter.into_iter().copied().collect()
    }
}

impl FromIterator<Vec<char>> for Data {
    fn from_iter<T: IntoIterator<Item = Vec<char>>>(iter: T) -> Self {
        iter.into_iter().flatten().collect()
    }
}

impl From<String> for Data {
    fn from(src: String) -> Self {
        src.chars().collect()
    }
}

impl From<&str> for Data {
    fn from(src: &str) -> Self {
        src.chars().collect()
    }
}

impl<'a, I> From<CorpusCleanerIterator<'a, I>> for Data
where
    I: Iterator<Item = char>,
{
    fn from(iter: CorpusCleanerIterator<'a, I>) -> Self {
        iter.flatten().collect()
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl FromParallelIterator<Vec<char>> for Data {
    fn from_par_iter<I>(par_iter: I) -> Self
    where
        I: IntoParallelIterator<Item = Vec<char>>,
    {
        par_iter
            .into_par_iter()
            .map(|v| v.into_iter().collect::<IntermediateData>())
            .reduce(IntermediateData::default, |a, b| a + b)
            .into()
    }
}
