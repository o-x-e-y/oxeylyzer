#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Write as _;
use std::io::Write as _;
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

pub const EXIT_MESSAGE: &str = "Exiting analyzer...";
pub const BASE_PATH: &str = concat!(std::env!("CARGO_MANIFEST_DIR"), "/..");
pub const MD5_HASH_LEN: usize = 16;

fn get_subcommand(cmd: &str) -> String {
    cmd.chars()
        .skip_while(|c| *c != '(')
        .take_while_inclusive(|c| *c != ')')
        .collect::<String>()
}

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
    #[error(
        "Failed to parse lisp expression: {err_message}\n{line}\n{}",
        std::iter::repeat(" ").take(idx.saturating_sub(2)).chain(["^"]).collect::<String>()
    )]
    SexpError {
        err_message: String,
        line: String,
        idx: usize,
    },
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
    #[error(
        "Attempting to execute command '{}' when '{}' does not return a layout",
        .0, get_subcommand(&.0)
    )]
    CommandDoesNotReturnLayout(String),

    #[error(transparent)]
    FmtError(#[from] std::fmt::Error),
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

#[derive(Debug, Clone, Default, PartialEq)]
pub enum ReplResponse {
    NoLayout {
        printable: String,
    },
    SingleLayout {
        layout: Box<Layout>,
        printable: String,
    },
    MultipleLayouts {
        layouts: Vec<Layout>,
        printable: String,
    },
    #[default]
    Nothing,
}

impl ReplResponse {
    pub fn no_layout(printable: String) -> Self {
        Self::NoLayout { printable }
    }

    pub fn single_layout(layout: FastLayout, printable: String) -> Self {
        Self::SingleLayout {
            layout: Box::new(layout.into()),
            printable,
        }
    }

    pub fn multiple_layouts(layouts: &[FastLayout], printable: String) -> Self {
        Self::MultipleLayouts {
            layouts: layouts.iter().cloned().map(Into::into).collect(),
            printable,
        }
    }
}

pub struct Repl {
    language: String,
    layout_gen: Oxeylyzer,
    saved: HashMap<String, Layout>,
    temp_generated: Vec<Layout>,
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
        let hash = md5_hash(line);
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
            .ok_or_else(|| match is_md5_hash(name) {
                true => ReplError::CommandDoesNotReturnLayout(name.to_string()),
                false => ReplError::UnknownLayout(name.into()),
            })
    }

    pub fn nth_layout(&self, index: usize) -> Result<FastLayout> {
        self.temp_generated
            .get(index)
            .map(|l| self.layout_gen.fast_layout(l, &[]))
            .ok_or(ReplError::IndexOutOfBounds(
                index,
                self.temp_generated.len(),
            ))
    }

    pub fn analyze(&self, name_or_nr: &str) -> Result<ReplResponse> {
        let mut buf = String::new();

        let layout = match name_or_nr.parse::<usize>() {
            Ok(nr) => self.nth_layout(nr)?,
            Err(_) => self.layout(name_or_nr)?,
        };

        writeln!(&mut buf, "{}", name_or_nr)?;
        write!(&mut buf, "{}", self.analyze_layout(&layout)?)?;

        Ok(ReplResponse::single_layout(layout, buf))
    }

    pub fn rank(&self) -> Result<ReplResponse> {
        let mut buf = String::new();

        self.saved
            .iter()
            .map(|(n, l)| {
                let fast = self.layout_gen.fast_layout(l, &[]);
                let s = self.layout_gen.score(&fast);
                let score = (s as f64) / (self.layout_gen.data.char_total as f64) / 100.0;
                (n, score)
            })
            .sorted_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(n, s)| writeln!(&mut buf, "{n: <15} {s:.3}"))
            .try_for_each(|e| e)?;

        Ok(ReplResponse::no_layout(buf))
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
    ) -> Result<ReplResponse> {
        let layout = self.layout(name)?.clone();

        let count = count.unwrap_or(2500);
        let pins = match pin_chars {
            Some(chars) => self.pin_positions(&layout, chars),
            None => vec![],
        };

        let response = self
            .thread_pool
            .install(|| generate_n_with_pins(&self.layout_gen, count, layout, &pins))?;

        use ReplResponse as RR;

        match response {
            RR::MultipleLayouts { layouts, printable } => {
                self.temp_generated = layouts.clone();
                Ok(RR::MultipleLayouts { layouts, printable })
            }
            response => Ok(response),
        }
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

    pub fn save(&mut self, n: usize, name: Option<String>) -> Result<ReplResponse> {
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

        Ok(ReplResponse::single_layout(layout, String::new()))
    }

    pub fn analyze_layout(&self, layout: &FastLayout) -> Result<String> {
        let mut buf = String::new();
        let stats = self.layout_gen.get_layout_stats(layout);

        let layout_str = heatmap_string(layout, &self.layout_gen.data);

        writeln!(&mut buf, "{layout_str}\n")?;
        write!(
            &mut buf,
            "{}",
            get_print_layout_stats(&stats, &self.layout_gen.data)?
        )?;

        Ok(buf)
    }

    pub fn compare(&self, name1: &str, name2: &str) -> Result<ReplResponse> {
        let mut buf = String::new();

        let l1 = self.layout(name1)?;
        let l2 = self.layout(name2)?;

        writeln!(&mut buf, "\n{: <32}{}", name1, name2)?;
        write!(
            &mut buf,
            "{}",
            get_print_compare_layouts(&l1, &l2, &self.layout_gen.data)?
        )?;

        let s1 = self.layout_gen.get_layout_stats(&l1);
        let s2 = self.layout_gen.get_layout_stats(&l2);

        write!(
            &mut buf,
            "{}",
            get_print_compare_stats(&s1, &s2, &self.layout_gen.data)?
        )?;

        Ok(ReplResponse::multiple_layouts(&[l1, l2], buf))
    }

    pub fn swap(&self, name: &str, swaps: &[String]) -> Result<ReplResponse> {
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

        let printable = self.analyze_layout(&layout)?;

        Ok(ReplResponse::single_layout(layout, printable))
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
    ) -> Result<String> {
        let fmt_freq = |v| {
            let f = v as f64 / self.layout_gen.data.bigram_total as f64;
            match is_percent {
                true => format!("{:.3}%", f),
                false => format!("{:.3}", f),
            }
        };

        let mut buf = String::new();

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
            .map(|(bigram, freq)| writeln!(&mut buf, "{bigram}: {}", fmt_freq(freq)))
            .try_for_each(|e| e)?;

        Ok(buf)
    }

    fn percent_stat(
        &self,
        layout: &FastLayout,
        count: usize,
        indices: &[PosPair],
    ) -> Result<String> {
        let pairs = indices
            .iter()
            .map(|p| BigramPair { pair: *p, dist: 1 })
            .collect::<Vec<_>>();

        self.bigram_stat(&pairs, Oxeylyzer::pair_sfb, layout, count, true)
    }

    pub fn sfbs(&self, name: &str, top_n: Option<usize>) -> Result<ReplResponse> {
        let mut buf = String::new();
        let layout = self.layout(name)?;
        let count = top_n.unwrap_or(10).min(layout.fspeed_indices.all.len());

        writeln!(&mut buf, "top {} sfbs for {name}:", count)?;

        let sfbs = self.bigram_stat(
            &layout.fspeed_indices.all,
            Oxeylyzer::pair_sfb,
            &layout,
            count,
            true,
        )?;

        write!(&mut buf, "{}", sfbs)?;

        Ok(ReplResponse::no_layout(buf))
    }

    pub fn pinky_ring(&self, name: &str, top_n: Option<usize>) -> Result<ReplResponse> {
        let mut buf = String::new();
        let layout = self.layout(name)?;
        let count = top_n
            .unwrap_or(10)
            .min(layout.pinky_ring_indices.pairs.len());

        writeln!(&mut buf, "top {} pinky-ring bigrams for {name}:", count)?;

        let pinky_ring = self.percent_stat(&layout, count, &layout.pinky_ring_indices.pairs)?;

        write!(&mut buf, "{}", pinky_ring)?;

        Ok(ReplResponse::no_layout(buf))
    }

    pub fn fspeed(&self, name: &str, top_n: Option<usize>) -> Result<ReplResponse> {
        let mut buf = String::new();
        let layout = self.layout(name)?;
        let count = top_n.unwrap_or(10).min(layout.fspeed_indices.all.len());

        writeln!(&mut buf, "top {} fspeed pairs for {name}:", count)?;

        let fspeed = self.bigram_stat(
            &layout.fspeed_indices.all,
            Oxeylyzer::pair_fspeed,
            &layout,
            count,
            false,
        )?;

        write!(&mut buf, "{}", fspeed)?;

        Ok(ReplResponse::no_layout(buf))
    }

    pub fn stretches(&self, name: &str, top_n: Option<usize>) -> Result<ReplResponse> {
        let mut buf = String::new();
        let layout = self.layout(name)?;
        let count = top_n
            .unwrap_or(10)
            .min(layout.stretch_indices.all_pairs.len());

        writeln!(&mut buf, "top {} stretch pairs for {name}:", count)?;

        let stretches = self.bigram_stat(
            &layout.stretch_indices.all_pairs,
            Oxeylyzer::pair_stretch,
            &layout,
            count,
            false,
        )?;

        write!(&mut buf, "{}", stretches)?;

        Ok(ReplResponse::no_layout(buf))
    }

    pub fn scissors(&self, name: &str, top_n: Option<usize>) -> Result<ReplResponse> {
        let mut buf = String::new();
        let layout = self.layout(name)?;
        let count = top_n.unwrap_or(10).min(layout.scissor_indices.pairs.len());

        writeln!(&mut buf, "top {} scissor pairs for {name}:", count)?;

        let scissors = self.percent_stat(&layout, count, &layout.scissor_indices.pairs)?;

        write!(&mut buf, "{}", scissors)?;

        Ok(ReplResponse::no_layout(buf))
    }

    pub fn lsbs(&self, name: &str, top_n: Option<usize>) -> Result<ReplResponse> {
        let mut buf = String::new();
        let layout = self.layout(name)?;
        let count = top_n.unwrap_or(10).min(layout.lsb_indices.pairs.len());

        writeln!(&mut buf, "top {} lsbs for {name}:", count)?;

        let lsbs = self.percent_stat(&layout, count, &layout.lsb_indices.pairs)?;

        write!(&mut buf, "{}", lsbs)?;

        Ok(ReplResponse::no_layout(buf))
    }

    pub fn language<P: AsRef<Path>>(&mut self, language: Option<P>) -> Result<ReplResponse> {
        let language = match language {
            Some(l) => l,
            None => {
                println!("Current language: {}", self.language);
                return Ok(ReplResponse::Nothing);
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

        Ok(ReplResponse::Nothing)
    }

    pub fn include<P: AsRef<Path>>(&mut self, languages: &[P]) -> Result<ReplResponse> {
        let layouts_base_path = PathBuf::from(BASE_PATH).join("static/layouts");

        let layouts = languages
            .iter()
            .flat_map(|language| {
                let layouts = load_layouts(layouts_base_path.join(language))
                    .inspect_err(|e| eprintln!("{e}"))?;

                println!(
                    "Included layouts from language '{}'",
                    language.as_ref().display()
                );

                Ok::<_, ReplError>(layouts)
            })
            .flatten()
            .map(|(name, l)| {
                self.saved.insert(name, l.clone());
                self.layout_gen.fast_layout(&l, &[])
            })
            .collect::<Vec<_>>();

        Ok(ReplResponse::multiple_layouts(&layouts, String::new()))
    }

    pub fn languages(&self) -> Result<ReplResponse> {
        let path = PathBuf::from(BASE_PATH).join("static/language_data");

        let mut buf = String::new();

        std::fs::read_dir(&path)
            .path_context(path)?
            .flatten()
            .map(|p| {
                let name = p
                    .file_name()
                    .to_string_lossy()
                    .replace('_', " ")
                    .replace(".json", "");

                match name.as_str() {
                    "test" => writeln!(&mut buf, "{}", name),
                    _ => Ok(()),
                }
            })
            .try_for_each(|e| e)?;

        Ok(ReplResponse::no_layout(buf))
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

    pub fn load(&mut self, language: String, all: bool, raw: bool) -> Result<ReplResponse> {
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

        Ok(ReplResponse::Nothing)
    }

    pub fn ngram(&self, ngram: &str) -> Result<ReplResponse> {
        let mut buf = String::new();

        let data = &self.layout_gen.data;

        match ngram.chars().count() {
            1 => {
                let c = ngram.chars().next().unwrap();
                let u = data.mapping.get_u(c);
                let occ = (data.get_char_u(u) as f64 / data.char_total as f64) * 100.0;
                writeln!(&mut buf, "{ngram}: {occ:.3}%")?
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

                writeln!(
                    &mut buf,
                    "{ngram} + {rev}: {:.3}%,\n  {ngram}: {occ_b1:.3}%\n  {rev}: {occ_b2:.3}%\n\
                    {ngram} + {rev} (skipgram): {:.3}%,\n  {ngram}: {occ_s1:.3}%\n  {rev}: {occ_s2:.3}%",
                    occ_b1 + occ_b2,
                    occ_s1 + occ_s2
                )?
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
                writeln!(
                    &mut buf,
                    "{ngram}: {:.3}%",
                    (occ as f64) / (data.trigram_total as f64) * 100.0
                )?
            }
            n => return Err(ReplError::InvalidNgramLength(n)),
        };

        Ok(ReplResponse::no_layout(buf))
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
            let fast_layout = self.layout_gen.fast_layout(l, &[]);
            *l = fast_layout.into();
        });

        Ok(())
    }

    pub fn reload(&mut self) -> Result<ReplResponse> {
        self.reset_with_language(&self.language.clone())?;

        Ok(ReplResponse::Nothing)
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

pub fn md5_hash(value: &str) -> String {
    format!("{:x}", md5::compute(value))
        .chars()
        .take(MD5_HASH_LEN)
        .collect()
}

pub fn is_md5_hash(value: &str) -> bool {
    value.len() == MD5_HASH_LEN && value.chars().all(|c| c.is_ascii_hexdigit())
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
