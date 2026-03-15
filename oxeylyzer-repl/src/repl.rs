#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};

use itertools::{EitherOrBoth, Itertools};
use oxeylyzer_core::corpus_cleaner::CorpusCleaner;
use oxeylyzer_core::data::Data;
use oxeylyzer_core::{OxeylyzerError, OxeylyzerResultExt};
use oxeylyzer_core::{
    cached_layout::*,
    generate::LayoutGeneration,
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
use crate::tui::*;

const EXIT_MESSAGE: &str = "Exiting analyzer...";

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
        "{} {}",
        "Missing <language> flag. The language flag can only be omitted in combination with",
        "`--all`.\nRun `load help` for more information about the command."
    )]
    MissingLanguageFlag,
    #[error("Could not serialize layout:\n{}\n", .0.formatted_string())]
    CouldNotSerializeLayout(FastLayout),
    #[error("Could not find corpus config for corpus '{0}'")]
    CouldNotFindCorpusConfig(String),

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
    layout_gen: LayoutGeneration,
    saved: HashMap<String, Layout>,
    temp_generated: Vec<FastLayout>,
    thread_pool: rayon::ThreadPool,
}

impl Repl {
    pub fn new<P>(generator_base_path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let config_path = concat!(std::env!("CARGO_MANIFEST_DIR"), "/../config.toml");
        let config = Config::with_loaded_weights(config_path);
        let language = config.language.clone();

        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(config.max_cores)
            .build()
            .unwrap();

        let layout_gen = LayoutGeneration::new(
            config.language.clone().as_str(),
            generator_base_path.as_ref(),
            Some(config),
        )?;

        Ok(Self {
            saved: load_layouts(generator_base_path.as_ref().join("layouts").join(&language))?,
            language,
            layout_gen,
            temp_generated: Vec::new(),
            thread_pool,
        })
    }

    pub fn run() -> Result<()> {
        let mut env = Self::new("static")?;

        let mut rl = DefaultEditor::new()?;

        rl.set_history_ignore_space(true);

        if rl.load_history("./static/history.txt").is_err() {
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

        if let Err(e) = rl.save_history("./static/history.txt") {
            rl.history().iter().for_each(|line| println!("{line}"));

            println!("Could not save history: {e}");
        }

        Ok(())
    }

    pub fn layout(&self, name: &str) -> Result<FastLayout> {
        self.saved
            .get(&name.to_lowercase())
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

    pub fn analyze(&self, name_or_nr: &str) -> Result<()> {
        let layout = match name_or_nr.parse::<usize>() {
            Ok(nr) => self.nth_layout(nr)?,
            Err(_) => &self.layout(name_or_nr)?,
        };

        println!("{}", name_or_nr);
        self.analyze_layout(layout);

        Ok(())
    }

    pub fn rank(&self) {
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

    fn generate(
        &mut self,
        name: &str,
        count: Option<usize>,
        pin_chars: Option<String>,
    ) -> Result<()> {
        let layout = self.layout(name)?.clone();

        let count = count.unwrap_or(2500);
        let pins = match pin_chars {
            Some(chars) => self.pin_positions(&layout, chars),
            None => vec![],
        };

        self.thread_pool.install(|| {
            self.temp_generated = generate_n_with_pins(&self.layout_gen, count, layout, &pins)
        });

        Ok(())
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

    pub fn save(&mut self, n: usize, name: Option<String>) -> Result<()> {
        let mut layout = self.nth_layout(n)?.clone();
        let new_name = match name {
            Some(name) => name,
            None => self.placeholder_name(&layout)?,
        };

        layout.name = Some(new_name.clone());
        let name_path = new_name.replace(' ', "_").to_lowercase();
        let path = PathBuf::from("static/layouts")
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
            .map_err(|_| ReplError::CouldNotSerializeLayout(layout.clone()))?;

        f.write_all(ser.into_inner().as_slice())
            .path_context(path)?;

        println!("saved {}\n{}", new_name, layout.formatted_string());

        self.saved.insert(new_name, layout.into());

        Ok(())
    }

    pub fn analyze_layout(&self, layout: &FastLayout) {
        let fmt_score = |base| (base as f64) / (self.layout_gen.data.char_total as f64) / 100.0;

        let stats = self.layout_gen.get_layout_stats(layout);
        let score = self.layout_gen.score(layout);

        let layout_str = heatmap_string(layout, &self.layout_gen.data);

        println!("{}\n{}\nScore: {:.3}", layout_str, stats, fmt_score(score));
    }

    pub fn compare(&self, name1: &str, name2: &str) -> Result<()> {
        let l1 = self.layout(name1)?;
        let l2 = self.layout(name2)?;

        println!("\n{: <32}{}", name1, name2);

        heatmap_string(&l1, &self.layout_gen.data)
            .split('\n')
            .zip(l1.formatted_string().split('\n'))
            .zip_longest(heatmap_string(&l2, &self.layout_gen.data).split('\n'))
            .for_each(|z| match z {
                EitherOrBoth::Both((r1, f), r2) => {
                    let spaces = std::iter::repeat_n(' ', 32 - f.len()).collect::<String>();
                    println!("{r1}{spaces}{r2}");
                }
                EitherOrBoth::Left((r1, _)) => println!("{r1}",),
                EitherOrBoth::Right(r2) => println!("{: <32}{r2}", ""),
            });

        let s1 = self.layout_gen.get_layout_stats(&l1);
        let s2 = self.layout_gen.get_layout_stats(&l2);
        let ts1 = s1.trigram_stats;
        let ts2 = s2.trigram_stats;

        let fmt_score = |base| (base as f64) / (self.layout_gen.data.char_total as f64) / 100.0;

        println!(
            concat!(
                "\n",
                "Sfb:                {: <11} Sfb:                {:.3}%\n",
                "Dsfb:               {: <11} Dsfb:               {:.3}%\n",
                "Finger Speed:       {: <11} Finger Speed:       {:.3}\n",
                "Stretches:          {: <11} Stretches:          {:.3}\n",
                "Scissors:           {: <11} Scissors:           {:.3}%\n",
                "Lsbs:               {: <11} Lsbs:               {:.3}%\n",
                "Pinky Ring Bigrams: {: <11} Pinky Ring Bigrams: {:.3}%\n\n",
                "Inrolls:            {: <11} Inrolls:            {:.2}%\n",
                "Outrolls:           {: <11} Outrolls:           {:.2}%\n",
                "Total Rolls:        {: <11} Total Rolls:        {:.2}%\n",
                "Onehands:           {: <11} Onehands:           {:.3}%\n\n",
                "Alternates:         {: <11} Alternates:         {:.2}%\n",
                "Alternates Sfs:     {: <11} Alternates Sfs:     {:.2}%\n",
                "Total Alternates:   {: <11} Total Alternates:   {:.2}%\n\n",
                "Redirects:          {: <11} Redirects:          {:.3}%\n",
                "Redirects Sfs:      {: <11} Redirects Sfs:      {:.3}%\n",
                "Bad Redirects:      {: <11} Bad Redirects:      {:.3}%\n",
                "Bad Redirects Sfs:  {: <11} Bad Redirects Sfs:  {:.3}%\n",
                "Total Redirects:    {: <11} Total Redirects:    {:.3}%\n\n",
                "Bad Sfbs:           {: <11} Bad Sfbs:           {:.3}%\n",
                "Sft:                {: <11} Sft:                {:.3}%\n\n",
                "Score:              {: <11} Score:              {:.3}\n"
            ),
            format!("{:.3}%", s1.sfb * 100.0),
            s2.sfb * 100.0,
            format!("{:.3}%", s1.dsfb * 100.0),
            s2.dsfb * 100.0,
            format!("{:.3}", s1.fspeed * 10.0),
            s2.fspeed * 10.0,
            format!("{:.3}", s1.stretches * 10.0),
            s2.stretches * 10.0,
            format!("{:.3}%", s1.scissors * 100.0),
            s2.scissors * 100.0,
            format!("{:.3}%", s1.lsbs * 100.0),
            s2.lsbs * 100.0,
            format!("{:.3}%", s1.pinky_ring * 100.0),
            s2.pinky_ring * 100.0,
            format!("{:.2}%", ts1.inrolls * 100.0),
            ts2.inrolls * 100.0,
            format!("{:.2}%", ts1.outrolls * 100.0),
            ts2.outrolls * 100.0,
            format!("{:.2}%", (ts1.inrolls + ts1.outrolls) * 100.0),
            (ts2.inrolls + ts2.outrolls) * 100.0,
            format!("{:.3}%", ts1.onehands * 100.0),
            ts2.onehands * 100.0,
            format!("{:.2}%", ts1.alternates * 100.0),
            ts2.alternates * 100.0,
            format!("{:.2}%", ts1.alternates_sfs * 100.0),
            ts2.alternates_sfs * 100.0,
            format!("{:.2}%", (ts1.alternates + ts1.alternates_sfs) * 100.0),
            (ts2.alternates + ts2.alternates_sfs) * 100.0,
            format!("{:.3}%", ts1.redirects * 100.0),
            ts2.redirects * 100.0,
            format!("{:.3}%", ts1.redirects_sfs * 100.0),
            ts2.redirects_sfs * 100.0,
            format!("{:.3}%", ts1.bad_redirects * 100.0),
            ts2.bad_redirects * 100.0,
            format!("{:.3}%", ts1.bad_redirects_sfs * 100.0),
            ts2.bad_redirects_sfs * 100.0,
            format!(
                "{:.3}%",
                (ts1.redirects + ts1.redirects_sfs + ts1.bad_redirects + ts1.bad_redirects_sfs)
                    * 100.0
            ),
            (ts2.redirects + ts2.redirects_sfs + ts2.bad_redirects + ts2.bad_redirects_sfs) * 100.0,
            format!("{:.3}%", ts1.bad_sfbs * 100.0),
            ts2.bad_sfbs * 100.0,
            format!("{:.3}%", ts1.sfts * 100.0),
            ts2.sfts * 100.0,
            format!("{:.3}", fmt_score(self.layout_gen.score(&l1))),
            fmt_score(self.layout_gen.score(&l2))
        );

        Ok(())
    }

    fn swap(&self, name: &str, swaps: &[String]) -> Result<()> {
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

        Ok(())
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
        freq: impl Fn(&LayoutGeneration, &FastLayout, &BigramPair) -> i64,
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

        self.bigram_stat(&pairs, LayoutGeneration::pair_sfb, layout, count, true);
    }

    fn sfbs(&self, name: &str, top_n: Option<usize>) -> Result<()> {
        let layout = self.layout(name)?;
        let count = top_n.unwrap_or(10).min(layout.fspeed_indices.all.len());

        println!("top {} sfbs for {name}:", count);

        self.bigram_stat(
            &layout.fspeed_indices.all,
            LayoutGeneration::pair_sfb,
            &layout,
            count,
            true,
        );

        Ok(())
    }

    fn pinky_ring(&self, name: &str, top_n: Option<usize>) -> Result<()> {
        let layout = self.layout(name)?;
        let count = top_n
            .unwrap_or(10)
            .min(layout.pinky_ring_indices.pairs.len());

        println!("top {} pinky-ring bigrams for {name}:", count);

        self.percent_stat(&layout, count, &layout.pinky_ring_indices.pairs);

        Ok(())
    }

    fn fspeed(&self, name: &str, top_n: Option<usize>) -> Result<()> {
        let layout = self.layout(name)?;
        let count = top_n.unwrap_or(10).min(layout.fspeed_indices.all.len());

        println!("top {} fspeed pairs for {name}:", count);

        self.bigram_stat(
            &layout.fspeed_indices.all,
            LayoutGeneration::pair_fspeed,
            &layout,
            count,
            false,
        );

        Ok(())
    }

    fn stretches(&self, name: &str, top_n: Option<usize>) -> Result<()> {
        let layout = self.layout(name)?;
        let count = top_n
            .unwrap_or(10)
            .min(layout.stretch_indices.all_pairs.len());

        println!("top {} stretch pairs for {name}:", count);

        self.bigram_stat(
            &layout.stretch_indices.all_pairs,
            LayoutGeneration::pair_stretch,
            &layout,
            count,
            false,
        );

        Ok(())
    }

    fn scissors(&self, name: &str, top_n: Option<usize>) -> Result<()> {
        let layout = self.layout(name)?;
        let count = top_n.unwrap_or(10).min(layout.scissor_indices.pairs.len());

        println!("top {} scissor pairs for {name}:", count);

        self.percent_stat(&layout, count, &layout.scissor_indices.pairs);

        Ok(())
    }

    fn lsbs(&self, name: &str, top_n: Option<usize>) -> Result<()> {
        let layout = self.layout(name)?;
        let count = top_n.unwrap_or(10).min(layout.lsb_indices.pairs.len());

        println!("top {} lsbs for {name}:", count);

        self.percent_stat(&layout, count, &layout.lsb_indices.pairs);

        Ok(())
    }

    fn language<P: AsRef<Path>>(&mut self, language: Option<P>) -> Result<()> {
        let language = match language {
            Some(l) => l,
            None => {
                println!("Current language: {}", self.language);
                return Ok(());
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

        Ok(())
    }

    pub fn include<P: AsRef<Path>>(&mut self, languages: &[P]) -> Result<()> {
        let layouts_base_path = PathBuf::from("./static/layouts");

        for language in languages {
            let language = language.as_ref().display().to_string();

            load_layouts(layouts_base_path.join(language))?
                .into_iter()
                .for_each(|(name, l)| {
                    self.saved.insert(name, l);
                });
        }

        Ok(())
    }

    pub fn languages(&self) -> Result<()> {
        let path = "static/language_data";
        std::fs::read_dir(path)
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

        Ok(())
    }

    fn load_one_with_config<P: AsRef<Path>>(
        &mut self,
        language: &str,
        cleaner: CorpusCleaner,
        corpus_paths: &[P],
    ) -> Result<()> {
        match Data::from_paths(corpus_paths, language, &cleaner) {
            Ok(data) => match data.save("./static/language_data") {
                Ok(_) => println!("Saved data for {language}!"),
                Err(e) => println!("Failed to save data for {language}: {e}"),
            },
            Err(e) => println!("Couldn't convert language: {e}"),
        };

        Ok(())
    }

    pub fn load<P: AsRef<Path>>(
        &mut self,
        language: String,
        corpus_paths: &[P],
        all: bool,
        raw: bool,
    ) -> Result<()> {
        let base_path = "./static/text/";

        let corpus_paths = match corpus_paths {
            &[] => vec![PathBuf::from(base_path).join(&language)],
            p @ &[_, ..] => p.iter().map(AsRef::as_ref).map(PathBuf::from).collect(),
        };

        match (all, raw) {
            (true, true) => {
                for dir_entry in std::fs::read_dir(&base_path)
                    .path_context(base_path)?
                    .flatten()
                    .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
                {
                    let language = dir_entry.file_name().display().to_string();
                    let corpus_path = PathBuf::from(base_path).join(&language);
                    let cleaner = CorpusCleaner::raw();

                    println!("loading raw data for language: {language}...");

                    self.load_one_with_config(&language, cleaner, &[corpus_path])?;
                }
            }
            (true, false) => {
                for dir_entry in std::fs::read_dir(&base_path)
                    .path_context(base_path)?
                    .flatten()
                    .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
                {
                    let language = dir_entry.file_name().display().to_string();
                    let corpus_path = PathBuf::from(base_path).join(&language);
                    let cleaner = CorpusConfig::new_translator(&language, None);

                    println!("loading data for language: {language}...");

                    self.load_one_with_config(&language, cleaner, &[corpus_path])?;
                }
            }
            (false, true) => {
                let cleaner = CorpusCleaner::raw();

                println!("loading raw data for language: {language}...");

                self.load_one_with_config(&language, cleaner, &corpus_paths)?;
            }
            (false, false) => {
                let cleaner = CorpusConfig::new_translator(&language, None);

                println!("loading data for {language}...");

                self.load_one_with_config(&language, cleaner, &corpus_paths)?;
                self.language(Some(language))?;
            }
        };

        Ok(())
    }

    pub fn ngram(&self, ngram: &str) -> Result<()> {
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

        Ok(())
    }

    fn reset_with_language(&mut self, language: &str) -> Result<()> {
        let config = Config::with_loaded_weights("config.toml");
        let layouts_path = PathBuf::from("./static/layouts").join(language);

        let generator = LayoutGeneration::new(language, "static", Some(config))?;
        let saved = load_layouts(layouts_path)?;

        self.layout_gen = generator;
        self.language = language.to_string();
        self.saved = saved;

        Ok(())
    }

    pub fn reload(&mut self) -> Result<()> {
        self.reset_with_language(&self.language.clone())
    }

    fn respond(&mut self, line: &str) -> Result<ReplStatus> {
        use crate::flags::{Repl, ReplCmd::*};

        let args = shlex::split(line)
            .ok_or(ReplError::ShlexError)?
            .into_iter()
            .map(std::ffi::OsString::from)
            .collect::<Vec<_>>();

        let flags = Repl::from_vec(args)?;

        match flags.subcommand {
            Analyze(a) => self.analyze(&a.name_or_nr)?,
            Compare(c) => self.compare(&c.name1, &c.name2)?,
            Swap(s) => self.swap(&s.name, &s.swaps)?,
            Rank(_) => self.rank(),
            Generate(i) => self.generate(&i.name, i.count, i.pins)?,
            Save(s) => self.save(s.n, s.name)?,
            Sfbs(s) => self.sfbs(&s.name, s.count)?,
            Fspeed(s) => self.fspeed(&s.name, s.count)?,
            Stretches(s) => self.stretches(&s.name, s.count)?,
            Scissors(s) => self.scissors(&s.name, s.count)?,
            Lsbs(s) => self.lsbs(&s.name, s.count)?,
            Pinkyring(s) => self.pinky_ring(&s.name, s.count)?,
            Language(l) => self.language(l.language)?,
            Include(l) => self.include(&l.languages)?,
            Languages(_) => self.languages()?,
            Load(l) => self.load(l.language, &l.corpus_paths, l.raw, l.all)?,
            Ngram(n) => self.ngram(&n.ngram)?,
            Reload(_) => self.reload()?,
            Quit(_) => return Ok(ReplStatus::Quit),
        };

        Ok(ReplStatus::Continue)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_layouts<P: AsRef<Path>>(path: P) -> Result<HashMap<String, Layout>> {
    if let Ok(readdir) = std::fs::read_dir(&path) {
        let map = readdir
            .flatten()
            .flat_map(|p| {
                Layout::load(p.path()).inspect_err(|e| {
                    println!("Error loading layout from '{}': {e}", p.path().display())
                })
            })
            .map(|l| (l.name.to_lowercase(), l))
            .collect();

        Ok(map)
    // } else if !path.exists() {
    //     fs::create_dir_all(path)?;
    //     Ok(HashMap::default())
    } else {
        Err(ReplError::NotADirectory(path.as_ref().into()))
    }
}

#[cfg(test)]
mod tests {
    use once_cell::sync::Lazy;

    use super::*;

    static REPL: Lazy<Repl> = Lazy::new(|| {
        let path = concat!(std::env!("CARGO_MANIFEST_DIR"), "/../static");

        Repl::new(path).unwrap()
    });

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
