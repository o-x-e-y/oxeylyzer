use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use itertools::Itertools;
use oxeylyzer_core::corpus_cleaner::CorpusCleaner;
use oxeylyzer_core::data::Data;
use oxeylyzer_core::{cached_layout::*, generate::LayoutGeneration, rayon, weights::Config};
use rustyline::DefaultEditor;
use rustyline::config::Configurer;
use rustyline::error::ReadlineError;
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

    #[error(transparent)]
    XflagsError(#[from] xflags::Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    // #[error("{0}")]
    // OxeylyzerDataError(#[from] OxeylyzerError),
    // #[error(transparent)]
    // DofError(#[from] libdof::DofError),
    // #[error(transparent)]
    // TomlSerializeError(#[from] toml::ser::Error),
    // #[error(transparent)]
    // TomlDeserializeError(#[from] toml::de::Error),
    #[error(transparent)]
    AnyhowError(#[from] anyhow::Error),
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
    saved: IndexMap<String, FastLayout>,
    temp_generated: Vec<FastLayout>,
    thread_pool: rayon::ThreadPool,
}

// TODO: move everything out to its own function
impl Repl {
    pub fn new<P>(generator_base_path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let config = Config::with_loaded_weights("config.toml");
        let language = config.language.clone();

        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(config.max_cores)
            .build()
            .unwrap();

        let mut layout_gen = LayoutGeneration::new(
            config.language.clone().as_str(),
            generator_base_path.as_ref(),
            Some(config),
        )?;

        Ok(Self {
            saved: layout_gen.load_layouts(
                generator_base_path.as_ref().join("layouts"),
                language.as_str(),
            )?,
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

    pub fn layout(&self, name: &str) -> Result<&FastLayout> {
        self.saved
            .get(&name.to_lowercase())
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
            Err(_) => self.layout(name_or_nr)?,
        };

        println!("{}", name_or_nr);
        self.analyze_layout(layout);

        Ok(())
    }

    pub fn rank(&self) {
        for (name, layout) in self.saved.iter() {
            let score = (layout.score as f64) / (self.layout_gen.data.char_total as f64) / 100.0;
            println!("{:10}{}", format!("{:.3}:", score), name);
        }
    }

    pub fn pin_positions(&self, layout: &FastLayout, pin_chars: String) -> Vec<usize> {
        let m = HashSet::<char>::from_iter(pin_chars.chars());

        layout
            .matrix
            .iter()
            .map(|u| self.layout_gen.mapping.get_c(*u))
            .enumerate()
            .filter_map(|(i, k)| m.contains(&k).then_some(i))
            .collect()
    }

    pub fn generate(&mut self, count: Option<usize>) -> Result<()> {
        let count = count.unwrap_or(2500);

        println!("generating {} layouts...", count);
        self.thread_pool.install(|| {
            // TODO: figure out how to use ctrl+c to cancel during generation
            self.temp_generated = generate_n(&self.layout_gen, count);
        });

        Ok(())
    }

    fn improve(
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
                .matrix
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

        let name_path = new_name.replace(' ', "_").to_lowercase();

        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(format!("static/layouts/{}/{}.kb", self.language, name_path))?;

        let layout_formatted = layout.formatted_string(&self.layout_gen.data.mapping);
        println!("saved {}\n{}", new_name, layout_formatted);
        f.write_all(layout_formatted.as_bytes()).unwrap();

        layout.score = self.layout_gen.score(&layout);
        self.saved.insert(new_name, layout);
        self.saved
            .sort_by(|_, a, _, b| a.score.partial_cmp(&b.score).unwrap());

        Ok(())
    }

    pub fn analyze_layout(&self, layout: &FastLayout) {
        let fmt_score = |base| (base as f64) / (self.layout_gen.data.char_total as f64) / 100.0;

        let stats = self.layout_gen.get_layout_stats(layout);
        let score = self.layout_gen.score(layout);

        let layout_str = heatmap_string(&self.layout_gen.data, layout);

        println!("{}\n{}\nScore: {:.3}", layout_str, stats, fmt_score(score));
    }

    pub fn compare(&self, name1: &str, name2: &str) -> Result<()> {
        let l1 = self.layout(name1)?;
        let l2 = self.layout(name2)?;

        println!("\n{:31}{}", name1, name2);
        for y in 0..3 {
            for (n, layout) in [l1, l2].into_iter().enumerate() {
                for x in 0..10 {
                    print!(
                        "{} ",
                        heatmap_heat(&self.layout_gen.data, layout.char(x + 10 * y).unwrap())
                    );
                    if x == 4 {
                        print!(" ");
                    }
                }
                if n == 0 {
                    print!("          ");
                }
            }
            println!();
        }
        let s1 = self.layout_gen.get_layout_stats(l1);
        let s2 = self.layout_gen.get_layout_stats(l2);
        let ts1 = s1.trigram_stats;
        let ts2 = s2.trigram_stats;

        let fmt_score = |base| (base as f64) / (self.layout_gen.data.char_total as f64) / 100.0;

        println!(
            concat!(
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
            format!("{:.3}", fmt_score(l1.score)),
            fmt_score(l2.score)
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
                    .matrix
                    .iter()
                    .position(|&k| k == self.layout_gen.mapping.get_u(c1));
                let p2 = layout
                    .matrix
                    .iter()
                    .position(|&k| k == self.layout_gen.mapping.get_u(c2));

                match (p1, p2) {
                    (Some(p1), Some(p2)) => assert!(layout.swap(p1, p2).is_some()),
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
    ) {
        let fmt_freq = |v| v as f64 / self.layout_gen.data.bigram_total as f64;

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

                let freq = freq(&self.layout_gen, layout, pair);

                Some((fmt, fmt_freq(freq)))
            })
            .sorted_by(|(_, a), (_, b)| a.total_cmp(b))
            .take(count)
            .for_each(|(bigram, freq)| println!("{bigram}: {:.3}", freq));
    }

    fn sfbs(&self, name: &str, top_n: Option<usize>) -> Result<()> {
        let layout = self.layout(name)?;
        let count = top_n.unwrap_or(10);

        println!("top {} sfbs for {name}:", count);

        self.bigram_stat(
            &layout.fspeed_indices.all,
            LayoutGeneration::pair_sfb,
            layout,
            count,
        );

        Ok(())
    }

    fn fspeed(&self, name: &str, top_n: Option<usize>) -> Result<()> {
        let layout = self.layout(name)?;
        let count = top_n.unwrap_or(10);

        println!("top {} fspeed pairs for {name}:", count);

        self.bigram_stat(
            &layout.fspeed_indices.all,
            LayoutGeneration::pair_fspeed,
            layout,
            count,
        );

        Ok(())
    }

    fn stretches(&self, name: &str, top_n: Option<usize>) -> Result<()> {
        let layout = self.layout(name)?;
        let count = top_n.unwrap_or(10);

        println!("top {} stretch pairs for {name}:", count);

        self.bigram_stat(
            &layout.stretch_indices.all_pairs,
            LayoutGeneration::pair_stretch,
            layout,
            count,
        );

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
        for language in languages {
            let language = language.as_ref().display().to_string();

            self.layout_gen
                .load_layouts("static/layouts", &language)?
                .into_iter()
                .for_each(|(name, layout)| {
                    self.saved.insert(name, layout);
                });
        }

        self.saved
            .sort_by(|_, a, _, b| a.score.partial_cmp(&b.score).unwrap());

        Ok(())
    }

    pub fn languages(&self) -> Result<()> {
        std::fs::read_dir("static/language_data")?
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
                for dir_entry in std::fs::read_dir(base_path)?
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
                for dir_entry in std::fs::read_dir(base_path)?
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

        let mut generator = LayoutGeneration::new(language, "static", Some(config))?;
        let saved = generator.load_layouts("static/layouts", language)?;

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
            Generate(g) => self.generate(g.count)?,
            Improve(i) => self.improve(&i.name, i.count, i.pins)?,
            Save(s) => self.save(s.n, s.name)?,
            Sfbs(s) => self.sfbs(&s.name, s.count)?,
            Fspeed(s) => self.fspeed(&s.name, s.count)?,
            Stretches(s) => self.stretches(&s.name, s.count)?,
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
