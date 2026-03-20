#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};

use itertools::Itertools;
use oxeylyzer_core::corpus_cleaner::CorpusCleaner;
use oxeylyzer_core::data::Data;
use oxeylyzer_core::{OxeylyzerError, OxeylyzerResultExt};
use oxeylyzer_core::{
    fast_layout::*,
    generate::Oxeylyzer,
    layout::{Layout, PosPair},
    rayon,
    weights::Config,
};
use rustyline::DefaultEditor;
use rustyline::config::Configurer;
use rustyline::error::ReadlineError;
use serde::Serialize;
use serde_json::Serializer;
use serde_json::ser::PrettyFormatter;
use thiserror::Error;

use crate::corpus_transposition::CorpusConfig;
use crate::display::*;

const EXIT_MESSAGE: &str = "Exiting analyzer...";
const BASE_PATH: &str = concat!(std::env!("CARGO_MANIFEST_DIR"), "/..");

#[derive(Debug, Error)]
pub enum ReplError {
    #[error("Layout '{0}' not found. It might exist, but it's not currently loaded.")]
    UnknownLayout(String),
    #[error("Could not find a placeholder name, try `save <index> <your own name>` instead")]
    FailedToFindPlaceholderName,
    #[error("Path '{0}' either doesn't exist or is not a directory")]
    NotADirectory(PathBuf),
    #[error("Invalid quotation marks")]
    ShlexError,
    #[error("Index '{0}' is out of bounds after generating {1} layouts")]
    IndexOutOfBounds(usize, usize),
    #[error("Invalid ngram length, found length {0}. Allowed lengths: 1, 2, 3")]
    InvalidNgramLength(usize),
    #[error("Failed to parse lisp expression: {0}")]
    SexpError(String), // TODO: Make these errors fancy with line numbers and such
    #[error(
        "{} {}",
        "Missing <language> flag. The language flag can only be omitted in combination with",
        "`--all`.\nRun `load help` for more information about the command."
    )]
    MissingLanguageFlag,
    #[error("Could not serialize layout:\n{}\n", .0.formatted_string())]
    CouldNotSerializeLayout(Box<FastLayout>),
    #[error("Could not find corpus config for corpus '{0}'")]
    CouldNotFindCorpusConfig(String),
    #[error(
        "Shift key can only be a single char, found '{}' with length {}", .0, .0.chars().count()
    )]
    WrongShiftKeyLength(String),
    #[error(
        "{}\n{} '{}'",
        "Failed to get corpus path: the process is ./path/corpus.json -> ./path/<new_lang>.json,",
        "But this is not possible for",
        .0.display()
    )]
    FailedToGetCorpusPath(PathBuf),
    #[error(
        "Could not get file name for corpus config file '{}'. Is it even a file?", .0.display()
    )]
    NoCorpusConfigFileName(PathBuf),

    #[error(transparent)]
    XflagsError(#[from] xflags::Error),
    #[error(transparent)]
    OxeylyzerError(#[from] OxeylyzerError),
    #[error(transparent)]
    ReadlineError(#[from] rustyline::error::ReadlineError),
}

pub type Result<T> = std::result::Result<T, ReplError>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReplStatus {
    Continue,
    Quit,
}

pub struct Repl {
    language: String,
    layout_gen: Oxeylyzer,
    saved: HashMap<String, Layout>,
    temp_generated: Vec<FastLayout>,
    temp_command_layouts: HashMap<String, Layout>,
    thread_pool: rayon::ThreadPool,
    corpus_configs: PathBuf,
    language_data: PathBuf,
}

impl Repl {
    pub fn new<P>(config_name: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let base = PathBuf::from(BASE_PATH);

        let config = Config::with_loaded_weights(base.join(config_name))?;
        let data = Data::load(base.join(&config.corpus)).unwrap();
        let language = data.name.clone();

        let corpus_configs = config.corpus_configs.clone();
        let language_data = config
            .corpus
            .parent()
            .unwrap_or_else(|| Path::new("./"))
            .to_path_buf();

        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(config.max_cores)
            .build()
            .unwrap();

        let saved = config
            .layouts
            .iter()
            .flat_map(|p| {
                load_layouts(p)
                    .inspect_err(|e| println!("Error loading layout at '{}': {e}", p.display()))
            })
            .flat_map(|h| h.into_iter())
            .collect();

        let layout_gen = Oxeylyzer::new(data, config);

        Ok(Self {
            saved,
            language,
            layout_gen,
            temp_generated: Vec::new(),
            temp_command_layouts: HashMap::new(),
            thread_pool,
            corpus_configs,
            language_data,
        })
    }

    pub fn run() -> Result<()> {
        let mut env = Self::new("config.toml")?;

        let mut rl = DefaultEditor::new()?;

        rl.set_history_ignore_space(true);

        let history_path = PathBuf::from(BASE_PATH).join("static/history.txt");
        if rl.load_history(&history_path).is_err() {
            println!("Welcome to Oxeylyzer!");
        }

        loop {
            let readline = rl.readline("> ");

            match readline {
                Ok(line) => {
                    if !matches!(line.as_str(), "quit" | "q" | "exit") {
                        rl.add_history_entry(&line)?;
                    }

                    match env.respond(&line) {
                        Ok(ReplStatus::Quit) => {
                            println!("{EXIT_MESSAGE}");
                            break;
                        }
                        Ok(ReplStatus::Continue) => continue,
                        Err(err) => {
                            println!("{err}");
                        }
                    }
                }
                Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                    println!("{EXIT_MESSAGE}");
                    break;
                }
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }

        if let Err(e) = rl.save_history(&history_path) {
            rl.history().iter().for_each(|line| println!("{line}"));

            println!("Could not save history: {e}");
        }

        Ok(())
    }

    pub fn insert_temp_layout(&mut self, line: &str, layout: Layout) {
        let hash = format!("{:x}", md5::compute(line));
        self.temp_command_layouts.insert(hash, layout);
    }

    pub fn clear_temp_layouts(&mut self) {
        self.temp_command_layouts.clear();
    }

    pub fn layout(&self, name: &str) -> Result<FastLayout> {
        self.saved
            .get(&name.to_lowercase())
            .or_else(|| self.temp_command_layouts.get(name))
            .map(|layout| self.layout_gen.fast_layout(layout, &[]))
            .ok_or(ReplError::UnknownLayout(name.into()))
    }

    pub fn nth_layout(&self, index: usize) -> Result<&FastLayout> {
        self.temp_generated
            .get(index)
            .ok_or(ReplError::IndexOutOfBounds(
                index,
                self.temp_generated.len(),
            ))
    }

    pub fn analyze(&self, name_or_nr: &str) -> Result<Option<FastLayout>> {
        let layout = match name_or_nr.parse::<usize>() {
            Ok(nr) => self.nth_layout(nr)?,
            Err(_) => &self.layout(name_or_nr)?,
        };

        println!("{}", name_or_nr);
        self.analyze_layout(layout);

        Ok(Some(layout.clone()))
    }

    pub fn rank(&self) -> Option<FastLayout> {
        self.saved
            .iter()
            .map(|(n, l)| {
                let fast = self.layout_gen.fast_layout(l, &[]);
                let s = self.layout_gen.score(&fast);
                let score = (s as f64) / (self.layout_gen.data.char_total as f64) / 100.0;
                (n, score)
            })
            .sorted_by(|(_, a), (_, b)| a.total_cmp(b))
            .for_each(|(n, s)| println!("{n: <15} {s:.3}"));

        None
    }

    pub fn pin_positions(&self, layout: &FastLayout, pin_chars: String) -> Vec<usize> {
        let m = HashSet::<char>::from_iter(pin_chars.chars());

        layout
            .keys
            .iter()
            .map(|u| self.layout_gen.mapping.get_c(*u))
            .enumerate()
            .filter_map(|(i, k)| m.contains(&k).then_some(i))
            .collect()
    }

    pub fn generate(
        &mut self,
        name: &str,
        count: Option<usize>,
        pin_chars: Option<String>,
    ) -> Result<Option<FastLayout>> {
        let layout = self.layout(name)?.clone();

        let count = count.unwrap_or(2500);
        let pins = match pin_chars {
            Some(chars) => self.pin_positions(&layout, chars),
            None => vec![],
        };

        self.thread_pool.install(|| {
            self.temp_generated = generate_n_with_pins(&self.layout_gen, count, layout, &pins)
        });

        Ok(None)
    }

    fn placeholder_name(&self, layout: &FastLayout) -> Result<String> {
        for i in 1..1000usize {
            let new_name = layout
                .keys
                .iter()
                .skip(10)
                .take(4)
                .map(|u| self.layout_gen.mapping.get_c(*u))
                .chain(i.to_string().chars())
                .collect::<String>()
                .replace('*', "-");

            if !self.saved.contains_key(&new_name) {
                return Ok(new_name);
            }
        }
        Err(ReplError::FailedToFindPlaceholderName)
    }

    pub fn save(&mut self, n: usize, name: Option<String>) -> Result<Option<FastLayout>> {
        let mut layout = self.nth_layout(n)?.clone();
        let new_name = match name {
            Some(name) => name,
            None => self.placeholder_name(&layout)?,
        };

        layout.name = Some(new_name.clone());
        let name_path = new_name.replace(' ', "_").to_lowercase();
        let path = PathBuf::from(BASE_PATH)
            .join("static/layouts")
            .join(&self.language)
            .join(name_path)
            .with_extension("dof");

        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .path_context(&path)?;

        let formatter = PrettyFormatter::with_indent(b"    ");
        let mut ser = Serializer::with_formatter(vec![], formatter);
        layout
            .serialize(&mut ser)
            .map_err(|_| ReplError::CouldNotSerializeLayout(Box::new(layout.clone())))?;

        f.write_all(ser.into_inner().as_slice())
            .path_context(path)?;

        println!("saved {}\n{}", new_name, layout.formatted_string());

        self.saved.insert(new_name, layout.clone().into());

        Ok(Some(layout))
    }

    pub fn analyze_layout(&self, layout: &FastLayout) {
        let stats = self.layout_gen.get_layout_stats(layout);

        let layout_str = heatmap_string(layout, &self.layout_gen.data);

        println!("{layout_str}\n");
        print_layout_stats(&stats, &self.layout_gen.data);
    }

    pub fn compare(&self, name1: &str, name2: &str) -> Result<Option<FastLayout>> {
        let l1 = self.layout(name1)?;
        let l2 = self.layout(name2)?;

        println!("\n{: <32}{}", name1, name2);
        print_compare_layouts(&l1, &l2, &self.layout_gen.data);

        let s1 = self.layout_gen.get_layout_stats(&l1);
        let s2 = self.layout_gen.get_layout_stats(&l2);

        print_compare_stats(&s1, &s2, &self.layout_gen.data);

        Ok(None)
    }

    pub fn swap(&self, name: &str, swaps: &[String]) -> Result<Option<FastLayout>> {
        let mut layout = self.layout(name)?.clone();

        swaps
            .iter()
            .filter(|swap| swap.len() >= 2)
            .flat_map(|swap| swap.chars().zip(swap.chars().skip(1)).collect::<Vec<_>>())
            .for_each(|(c1, c2)| {
                let p1 = layout
                    .keys
                    .iter()
                    .position(|&k| k == self.layout_gen.mapping.get_u(c1));
                let p2 = layout
                    .keys
                    .iter()
                    .position(|&k| k == self.layout_gen.mapping.get_u(c2));

                match (p1, p2) {
                    (Some(p1), Some(p2)) => assert!(layout.swap(p1 as u8, p2 as u8).is_some()),
                    (Some(_), None) => {
                        println!("Couldn't swap {c1}{c2} because {c1} is not on the layout.")
                    }
                    (None, Some(_)) => {
                        println!("Couldn't swap {c1}{c2} because {c2} is not on the layout.")
                    }
                    (None, None) => {
                        println!("Couldn't swap {c1}{c2} because neither key is on the layout.")
                    }
                }
            });

        self.analyze_layout(&layout);

        Ok(Some(layout.clone()))
    }

    pub fn sfr_freq(&self) -> f64 {
        let total = (0..self.layout_gen.data.len() as u8)
            .map(|u| self.layout_gen.data.get_bigram_u([u, u]))
            .sum::<i64>();

        total as f64 / self.layout_gen.data.bigram_total as f64
    }

    fn bigram_stat(
        &self,
        pairs: &[BigramPair],
        freq: impl Fn(&Oxeylyzer, &FastLayout, &BigramPair) -> i64,
        layout: &FastLayout,
        count: usize,
        is_percent: bool,
    ) {
        let fmt_freq = |v| {
            let f = v as f64 / self.layout_gen.data.bigram_total as f64;
            match is_percent {
                true => format!("{:.3}%", f),
                false => format!("{:.3}", f),
            }
        };

        pairs
            .iter()
            .flat_map(|pair| {
                let u1 = layout.char(pair.pair.0)?;
                let u2 = layout.char(pair.pair.1)?;

                let bigram = self
                    .layout_gen
                    .mapping
                    .map_us(&[u1, u2])
                    .collect::<String>();

                let bigram2 = self
                    .layout_gen
                    .mapping
                    .map_us(&[u2, u1])
                    .collect::<String>();

                let fmt = format!("{bigram}/{bigram2}");

                let freq = match is_percent {
                    true => 100 * freq(&self.layout_gen, layout, pair),
                    false => freq(&self.layout_gen, layout, pair),
                };

                Some((fmt, freq))
            })
            .sorted_by(|(_, f1), (_, f2)| match is_percent {
                true => f2.cmp(f1),
                false => f1.cmp(f2),
            })
            .take(count)
            .for_each(|(bigram, freq)| println!("{bigram}: {}", fmt_freq(freq)));
    }

    fn percent_stat(&self, layout: &FastLayout, count: usize, indices: &[PosPair]) {
        let pairs = indices
            .iter()
            .map(|p| BigramPair { pair: *p, dist: 1 })
            .collect::<Vec<_>>();

        self.bigram_stat(&pairs, Oxeylyzer::pair_sfb, layout, count, true);
    }

    pub fn sfbs(&self, name: &str, top_n: Option<usize>) -> Result<Option<FastLayout>> {
        let layout = self.layout(name)?;
        let count = top_n.unwrap_or(10).min(layout.fspeed_indices.all.len());

        println!("top {} sfbs for {name}:", count);

        self.bigram_stat(
            &layout.fspeed_indices.all,
            Oxeylyzer::pair_sfb,
            &layout,
            count,
            true,
        );

        Ok(None)
    }

    pub fn pinky_ring(&self, name: &str, top_n: Option<usize>) -> Result<Option<FastLayout>> {
        let layout = self.layout(name)?;
        let count = top_n
            .unwrap_or(10)
            .min(layout.pinky_ring_indices.pairs.len());

        println!("top {} pinky-ring bigrams for {name}:", count);

        self.percent_stat(&layout, count, &layout.pinky_ring_indices.pairs);

        Ok(None)
    }

    pub fn fspeed(&self, name: &str, top_n: Option<usize>) -> Result<Option<FastLayout>> {
        let layout = self.layout(name)?;
        let count = top_n.unwrap_or(10).min(layout.fspeed_indices.all.len());

        println!("top {} fspeed pairs for {name}:", count);

        self.bigram_stat(
            &layout.fspeed_indices.all,
            Oxeylyzer::pair_fspeed,
            &layout,
            count,
            false,
        );

        Ok(None)
    }

    pub fn stretches(&self, name: &str, top_n: Option<usize>) -> Result<Option<FastLayout>> {
        let layout = self.layout(name)?;
        let count = top_n
            .unwrap_or(10)
            .min(layout.stretch_indices.all_pairs.len());

        println!("top {} stretch pairs for {name}:", count);

        self.bigram_stat(
            &layout.stretch_indices.all_pairs,
            Oxeylyzer::pair_stretch,
            &layout,
            count,
            false,
        );

        Ok(None)
    }

    pub fn scissors(&self, name: &str, top_n: Option<usize>) -> Result<Option<FastLayout>> {
        let layout = self.layout(name)?;
        let count = top_n.unwrap_or(10).min(layout.scissor_indices.pairs.len());

        println!("top {} scissor pairs for {name}:", count);

        self.percent_stat(&layout, count, &layout.scissor_indices.pairs);

        Ok(None)
    }

    pub fn lsbs(&self, name: &str, top_n: Option<usize>) -> Result<Option<FastLayout>> {
        let layout = self.layout(name)?;
        let count = top_n.unwrap_or(10).min(layout.lsb_indices.pairs.len());

        println!("top {} lsbs for {name}:", count);

        self.percent_stat(&layout, count, &layout.lsb_indices.pairs);

        Ok(None)
    }

    pub fn language<P: AsRef<Path>>(&mut self, language: Option<P>) -> Result<Option<FastLayout>> {
        let language = match language {
            Some(l) => l,
            None => {
                println!("Current language: {}", self.language);
                return Ok(None);
            }
        };

        let language = language.as_ref().display().to_string();
        match self.reset_with_language(&language) {
            Ok(_) => println!(
                "Set language to {}. Sfr: {:.2}%",
                &language,
                self.sfr_freq() * 100.0
            ),
            Err(e) => println!("Failed to set language: {}", e),
        }

        Ok(None)
    }

    pub fn include<P: AsRef<Path>>(&mut self, languages: &[P]) -> Result<Option<FastLayout>> {
        let layouts_base_path = PathBuf::from(BASE_PATH).join("static/layouts");

        for language in languages {
            let language = language.as_ref().display().to_string();

            load_layouts(layouts_base_path.join(language))?
                .into_iter()
                .for_each(|(name, l)| {
                    self.saved.insert(name, l);
                });
        }

        Ok(None)
    }

    pub fn languages(&self) -> Result<Option<FastLayout>> {
        let path = PathBuf::from(BASE_PATH).join("static/language_data");

        std::fs::read_dir(&path)
            .path_context(path)?
            .flatten()
            .for_each(|p| {
                let name = p
                    .file_name()
                    .to_string_lossy()
                    .replace('_', " ")
                    .replace(".json", "");

                if name != "test" {
                    println!("{}", name);
                }
            });

        Ok(None)
    }

    fn load_one_with_cleaner<P: AsRef<Path>>(
        &mut self,
        language: &str,
        cleaner: CorpusCleaner,
        corpus_paths: &[P],
    ) -> Result<()> {
        let language_data_path = PathBuf::from(BASE_PATH).join(&self.language_data);
        match Data::from_paths(corpus_paths, language, &cleaner) {
            Ok(data) => match data.save(language_data_path) {
                Ok(_) => println!("Saved data for {language}!"),
                Err(e) => println!("Failed to save data for {language}: {e}"),
            },
            Err(e) => println!("Couldn't convert language: {e}"),
        };

        Ok(())
    }

    pub fn load(&mut self, language: String, all: bool, raw: bool) -> Result<Option<FastLayout>> {
        let corpus_configs = PathBuf::from(BASE_PATH).join(&self.corpus_configs);

        match (all, raw) {
            (true, true) => {
                glob::glob(&corpus_configs.to_string_lossy())
                    .path_context(&corpus_configs)?
                    .flat_map(|p| p.inspect_err(|e| eprintln!("{e}")))
                    .map(|path| {
                        let config = CorpusConfig::load(&path)?;
                        let sources = config.sources().to_vec();
                        let cleaner = CorpusCleaner::raw();
                        let language = path
                            .file_stem()
                            .ok_or_else(|| ReplError::NoCorpusConfigFileName(path.clone()))?
                            .to_string_lossy();

                        println!("loading raw data for language: {language}...");

                        self.load_one_with_cleaner(&language, cleaner, &sources)
                    })
                    .for_each(|res| {
                        let _ = res.inspect_err(|e| eprintln!("{e}"));
                    });
            }
            (true, false) => {
                glob::glob(&corpus_configs.to_string_lossy())
                    .path_context(&corpus_configs)?
                    .flat_map(|p| p.inspect_err(|e| eprintln!("{e}")))
                    .map(|path| {
                        let config = CorpusConfig::load(&path)?;
                        let sources = config.sources().to_vec();
                        let cleaner = config.into();
                        let language = path
                            .file_stem()
                            .ok_or_else(|| ReplError::NoCorpusConfigFileName(path.clone()))?
                            .to_string_lossy();

                        println!("loading data for language: {language}...");

                        self.load_one_with_cleaner(&language, cleaner, &sources)
                    })
                    .for_each(|res| {
                        let _ = res.inspect_err(|e| eprintln!("{e}"));
                    });
            }
            (false, true) => {
                let config_path = glob::glob(&corpus_configs.to_string_lossy())
                    .path_context(&corpus_configs)?
                    .flatten()
                    .find(|path| {
                        path.file_stem()
                            .map(|n| n == language.as_str())
                            .unwrap_or(false)
                    })
                    .ok_or_else(|| ReplError::CouldNotFindCorpusConfig(language.clone()))?;

                let config = CorpusConfig::load(config_path)?;
                let sources = config.sources();
                let cleaner = CorpusCleaner::raw();

                println!("loading raw data for language: {language}...");

                self.load_one_with_cleaner(&language, cleaner, sources)?;
            }
            (false, false) => {
                let config_path = glob::glob(&corpus_configs.to_string_lossy())
                    .path_context(&corpus_configs)?
                    .flatten()
                    .find(|path| {
                        path.file_stem()
                            .map(|n| n == language.as_str())
                            .unwrap_or(false)
                    })
                    .ok_or_else(|| ReplError::CouldNotFindCorpusConfig(language.clone()))?;

                let config = CorpusConfig::load(config_path)?;
                let sources = config.sources().to_vec();
                let cleaner = CorpusCleaner::from(config);

                println!("loading data for {language}...");

                self.load_one_with_cleaner(&language, cleaner, &sources)?;
                self.language(Some(language))?;
            }
        };

        Ok(None)
    }

    pub fn ngram(&self, ngram: &str) -> Result<Option<FastLayout>> {
        let data = &self.layout_gen.data;

        match ngram.chars().count() {
            1 => {
                let c = ngram.chars().next().unwrap();
                let u = data.mapping.get_u(c);
                let occ = (data.get_char_u(u) as f64 / data.char_total as f64) * 100.0;
                println!("{ngram}: {occ:.3}%")
            }
            2 => {
                let bigram: [char; 2] = ngram.chars().collect::<Vec<char>>().try_into().unwrap();
                let c1 = data.mapping.get_u(bigram[0]);
                let c2 = data.mapping.get_u(bigram[1]);

                let rev = bigram.into_iter().rev().collect::<String>();

                let occ_b1 =
                    (data.get_bigram_u([c1, c2]) as f64 / data.bigram_total as f64) * 100.0;
                let occ_b2 =
                    (data.get_bigram_u([c2, c1]) as f64 / data.bigram_total as f64) * 100.0;
                let occ_s1 =
                    (data.get_skipgram_u([c1, c2]) as f64 / data.skipgram_total as f64) * 100.0;
                let occ_s2 =
                    (data.get_skipgram_u([c2, c1]) as f64 / data.skipgram_total as f64) * 100.0;

                println!(
                    "{ngram} + {rev}: {:.3}%,\n  {ngram}: {occ_b1:.3}%\n  {rev}: {occ_b2:.3}%\n\
                    {ngram} + {rev} (skipgram): {:.3}%,\n  {ngram}: {occ_s1:.3}%\n  {rev}: {occ_s2:.3}%",
                    occ_b1 + occ_b2,
                    occ_s1 + occ_s2
                )
            }
            3 => {
                let trigram: [char; 3] = ngram.chars().collect::<Vec<char>>().try_into().unwrap();
                let t = [
                    data.mapping.get_u(trigram[0]),
                    data.mapping.get_u(trigram[1]),
                    data.mapping.get_u(trigram[2]),
                ];
                let &(_, occ) = data
                    .gen_trigrams()
                    .iter()
                    .find(|&&(tf, _)| tf == t)
                    .unwrap_or(&(t, 0));
                println!(
                    "{ngram}: {:.3}%",
                    (occ as f64) / (data.trigram_total as f64) * 100.0
                )
            }
            n => return Err(ReplError::InvalidNgramLength(n)),
        };

        Ok(None)
    }

    fn reset_with_language(&mut self, language: &str) -> Result<()> {
        let config = Config::with_loaded_weights(PathBuf::from(BASE_PATH).join("config.toml"))?;
        let corpus_configs = config.corpus_configs.clone();
        let language_data = config
            .corpus
            .parent()
            .ok_or_else(|| ReplError::FailedToGetCorpusPath(config.corpus.clone()))?
            .to_path_buf();
        let corpus_path = PathBuf::from(BASE_PATH)
            .join(&language_data)
            .join(language)
            .with_extension("json");

        let data = Data::load(corpus_path)?;

        let saved = config
            .layouts
            .iter()
            .flat_map(|p| {
                load_layouts(p)
                    .inspect_err(|e| println!("Error loading layout at '{}': {e}", p.display()))
            })
            .flat_map(|h| h.into_iter())
            .chain(std::mem::take(&mut self.saved))
            .collect();

        let generator = Oxeylyzer::new(data, config);

        self.language_data = language_data;
        self.corpus_configs = corpus_configs;
        self.layout_gen = generator;
        self.language = language.to_string();
        self.saved = saved;
        self.temp_generated.iter_mut().for_each(|l| {
            let layout = Layout::from(l.clone());
            *l = self.layout_gen.fast_layout(&layout, &[]);
        });

        Ok(())
    }

    pub fn reload(&mut self) -> Result<Option<FastLayout>> {
        self.reset_with_language(&self.language.clone())?;

        Ok(None)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_layouts<P: AsRef<Path>>(path: P) -> Result<HashMap<String, Layout>> {
    let base = PathBuf::from(BASE_PATH).join(&path);
    let pattern = match base.is_dir() {
        true => base.join("*.dof"),
        false => base,
    };
    let pattern_str = pattern.to_string_lossy();

    let map = glob::glob(&pattern_str)
        .str_context(&pattern_str)?
        .flatten()
        .flat_map(|p| {
            Layout::load(&p)
                .inspect_err(|e| println!("Error loading layout from '{}': {e}", p.display()))
        })
        .map(|l| (l.name.to_lowercase(), l))
        .collect();

    Ok(map)
}

#[cfg(test)]
mod tests {
    use once_cell::sync::Lazy;

    use super::*;

    static REPL: Lazy<Repl> = Lazy::new(|| Repl::new("config.toml").unwrap());

    static QWERTY: Lazy<FastLayout> = Lazy::new(|| {
        let dof_str = r#"
            {
                "name": "Qwerty",
                "board": "ansi",
                "layers": {
                    "main": [
                        "q w e r t  y u i o p",
                        "a s d f g  h j k l ;",
                        "z x c v b  n m , . /"
                    ]
                },
                "fingering": "traditional"
            }
        "#;

        let layout = serde_json::from_str::<Layout>(dof_str).unwrap();

        REPL.layout_gen.fast_layout(&layout, &[])
    });

    #[test]
    fn pins() {
        let pins = REPL.pin_positions(&QWERTY, "qwerty".to_string());
        assert_eq!(pins, vec![0, 1, 2, 3, 4, 5]);

        let pins = REPL.pin_positions(&QWERTY, "wasd".to_string());
        assert_eq!(pins, vec![1, 10, 11, 12]);
    }
}
