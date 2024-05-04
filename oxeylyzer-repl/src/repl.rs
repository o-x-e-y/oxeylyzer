use std::io::Write;
use std::path::Path;

use indexmap::IndexMap;
use itertools::Itertools;
use oxeylyzer_core::{generate::LayoutGeneration, layout::*, load_text, weights::Config};

use crate::corpus_transposition::CorpusConfig;
use crate::tui::*;

pub struct Repl {
    language: String,
    gen: LayoutGeneration,
    saved: IndexMap<String, FastLayout>,
    temp_generated: Vec<FastLayout>,
    pins: Vec<usize>,
}

impl Repl {
    pub fn new<P>(generator_base_path: P) -> Result<Self, String>
    where
        P: AsRef<Path>,
    {
        let config = Config::with_loaded_weights();
        let language = config.defaults.language.clone();
        let pins = config.pins.clone();

        let mut gen = LayoutGeneration::new(
            config.defaults.language.clone().as_str(),
            generator_base_path.as_ref(),
            Some(config),
        )
        .unwrap_or_else(|_| panic!("Could not read language data for {}", language));

        Ok(Self {
            saved: gen
                .load_layouts(
                    generator_base_path.as_ref().join("layouts"),
                    language.as_str(),
                )
                .map_err(|e| e.to_string())?,
            language,
            gen,
            temp_generated: Vec::new(),
            pins,
        })
    }

    pub fn run() -> Result<(), String> {
        let mut env = Self::new("static")?;

        loop {
            let line = readline().map_err(|e| e.to_string())?;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            match env.respond(line) {
                Ok(true) => break,
                Ok(false) => continue,
                Err(err) => {
                    println!("{err}");
                }
            }
        }

        Ok(())
    }

    pub fn rank(&self) {
        for (name, layout) in self.saved.iter() {
            println!("{:10}{}", format!("{:.3}:", layout.score), name);
        }
    }

    pub fn layout_by_name(&self, name: &str) -> Option<&FastLayout> {
        self.saved.get(name)
    }

    pub fn analyze_name(&self, name: &str) {
        let l = match self.layout_by_name(name) {
            Some(layout) => layout,
            None => {
                println!("layout {} does not exist!", name);
                return;
            }
        };
        println!("{}", name);
        self.analyze(l);
    }

    fn placeholder_name(&self, layout: &FastLayout) -> Result<String, String> {
        for i in 1..1000usize {
            let new_name_bytes = layout.matrix[10..14].to_vec();
            let mut new_name = self.gen.data.convert_u8.as_str(new_name_bytes.as_slice());

            new_name.push_str(format!("{}", i).as_str());

            if !self.saved.contains_key(&new_name) {
                return Ok(new_name);
            }
        }
        Err("Could not find a good placeholder name for the layout.".to_string())
    }

    pub fn save(&mut self, mut layout: FastLayout, name: Option<String>) -> Result<(), String> {
        let new_name = if let Some(n) = name {
            n.replace(' ', "_")
        } else {
            self.placeholder_name(&layout).unwrap()
        };

        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(format!("static/layouts/{}/{}.kb", self.language, new_name))
            .map_err(|e| e.to_string())?;

        let layout_formatted = layout.formatted_string(&self.gen.data.convert_u8);
        println!("saved {}\n{}", new_name, layout_formatted);
        f.write_all(layout_formatted.as_bytes()).unwrap();

        layout.score = self.gen.score(&layout);
        self.saved.insert(new_name, layout);
        self.saved
            .sort_by(|_, a, _, b| a.score.partial_cmp(&b.score).unwrap());

        Ok(())
    }

    pub fn analyze(&self, layout: &FastLayout) {
        let stats = self.gen.get_layout_stats(layout);
        let score = if layout.score == 0.000 {
            self.gen.score(layout)
        } else {
            layout.score
        };

        let layout_str = heatmap_string(&self.gen.data, layout);

        println!("{}\n{}\nScore: {:.3}", layout_str, stats, score);
    }

    pub fn compare_name(&self, name1: &str, name2: &str) {
        let l1 = match self.layout_by_name(name1) {
            Some(layout) => layout,
            None => {
                println!("layout {} does not exist!", name1);
                return;
            }
        };
        let l2 = match self.layout_by_name(name2) {
            Some(layout) => layout,
            None => {
                println!("layout {} does not exist!", name2);
                return;
            }
        };
        println!("\n{:31}{}", name1, name2);
        for y in 0..3 {
            for (n, layout) in [l1, l2].into_iter().enumerate() {
                for x in 0..10 {
                    print!("{} ", heatmap_heat(&self.gen.data, layout.c(x + 10 * y)));
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
        let s1 = self.gen.get_layout_stats(l1);
        let s2 = self.gen.get_layout_stats(l2);
        let ts1 = s1.trigram_stats;
        let ts2 = s2.trigram_stats;
        println!(
            concat!(
                "Sfb:                {: <11} Sfb:                {:.3}%\n",
                "Dsfb:               {: <11} Dsfb:               {:.3}%\n",
                "Finger Speed:       {: <11} Finger Speed:       {:.3}\n",
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
            format!("{:.3}", l1.score),
            l2.score
        );
    }

    fn get_nth(&self, nr: usize) -> Option<&FastLayout> {
        self.temp_generated.get(nr)
    }

    pub fn sfr_freq(&self) -> f64 {
        let len = self.gen.data.characters.len();
        let chars = 0..len;
        chars
            .clone()
            .cartesian_product(chars)
            .filter(|(i1, i2)| i1 == i2)
            .map(|(c1, c2)| self.gen.data.bigrams.get(c1 * len + c2).unwrap_or(&0.0))
            .sum()
    }

    fn sfbs(&self, name: &str, top_n: usize) {
        if let Some(layout) = self.layout_by_name(name) {
            println!("top {} sfbs for {name}:", top_n.min(48));

            for (bigram, freq) in self.gen.sfbs(layout, top_n) {
                println!("{bigram}: {:.3}%", freq * 100.0)
            }
        } else {
            println!("layout {name} does not exist!")
        }
    }

    fn respond(&mut self, line: &str) -> Result<bool, String> {
        use crate::flags::{Repl, ReplCmd::*};

        let args = shlex::split(line)
            .ok_or("Invalid quotations")?
            .into_iter()
            .map(std::ffi::OsString::from)
            .collect::<Vec<_>>();

        let flags = Repl::from_vec(args).map_err(|e| e.to_string())?;

        match flags.subcommand {
            Analyze(a) => match a.name_or_nr.parse::<usize>() {
                Ok(nr) => match self.get_nth(nr) {
                    Some(layout) => self.analyze(layout),
                    None => {
                        return Err(format!(
                            "Index '{}' provided is out of bounds for {} generated layouts",
                            a.name_or_nr,
                            self.temp_generated.len()
                        ))
                    }
                },
                Err(_) => self.analyze_name(&a.name_or_nr),
            },
            Compare(c) => self.compare_name(&c.name1, &c.name2),
            Rank(_) => self.rank(),
            Generate(g) => {
                println!("generating {} layouts...", g.count);
                self.temp_generated = generate_n(&self.gen, g.count);
            }
            Improve(i) => match self.layout_by_name(&i.name) {
                Some(l) => {
                    self.temp_generated =
                        generate_n_with_pins(&self.gen, i.count, l.clone(), &self.pins)
                }
                None => return Err(format!("'{}' does not exist!", i.name)),
            },
            Save(s) => match (self.get_nth(s.n), s.name) {
                (Some(layout), name) => self.save(layout.clone(), name)?,
                (None, _) => {
                    return Err(format!(
                        "Index '{}' provided is out of bounds for {} generated layouts",
                        s.n,
                        self.temp_generated.len()
                    ))
                }
            },
            Sfbs(s) => match s.count {
                Some(count) => self.sfbs(&s.name, count),
                None => self.sfbs(&s.name, 10),
            },
            Language(l) => match l.language {
                Some(l) => {
                    let config = Config::with_loaded_weights();
                    let language = l.to_str()
                        .ok_or_else(|| format!("Language is invalid utf8: {:?}", l))?;
    
                    println!("{language:?}");
    
                    if let Ok(generator) = LayoutGeneration::new(language, "static", Some(config)) {
                        self.gen = generator;
                        self.saved = self
                            .gen
                            .load_layouts("static/layouts", language)
                            .expect("couldn't load layouts lol");
                        self.language = language.to_string();
    
                        println!(
                            "Set language to {}. Sfr: {:.2}%",
                            &language,
                            self.sfr_freq() * 100.0
                        );
                    } else {
                        return Err(format!("Could not load data for {}", language));
                    }
                },
                None => println!("Current language: {}", self.language)
            }
            Include(l) => {
                self
                    .gen
                    .load_layouts("static/layouts", &l.language)
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .for_each(|(name, layout)| {
                        self.saved.insert(name, layout);
                    });
                self.saved.sort_by(|_, a, _, b| a.score.partial_cmp(&b.score).unwrap());
            }
            Languages(_) => {
                std::fs::read_dir("static/language_data")
                    .map_err(|e| e.to_string())?
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
            }
            Load(l) => match (l.all, l.raw) {
                (true, true) => {
                    return Err("You can't currently generate all corpora as raw".into())
                }
                (true, _) => {
                    for (language, config) in CorpusConfig::all() {
                        println!("loading data for language: {language}...");

                        load_text::load_data(language.as_str(), config.translator())
                            .map_err(|e| e.to_string())?;
                    }
                }
                (false, true) => {
                    println!("loading raw data for language: {}...", l.language.display());
                    load_text::load_raw(&l.language.display().to_string());
                }
                (false, false) => {
                    let language = l.language.to_str()
                        .ok_or_else(|| format!("Language is invalid utf8: {:?}", l.language))?;

                    let translator = CorpusConfig::new_translator(language, None);
                    let is_raw_translator = translator.is_raw;

                    println!("loading data for {}...", &language);
                    load_text::load_data(language, translator).map_err(|e| e.to_string())?;

                    if !is_raw_translator {
                        let config = Config::with_loaded_weights();
                        match LayoutGeneration::new(language, "static", Some(config)) {
                            Ok(generator) => {
                                self.language = language.into();
                                self.gen = generator;
                                self.saved = self
                                    .gen
                                    .load_layouts("static/layouts", language)
                                    .map_err(|e| e.to_string())?;

                                println!(
                                    "Set language to {}. Sfr: {:.2}%",
                                    language,
                                    self.sfr_freq() * 100.0
                                );
                            }
                            Err(e) => return Err(e.to_string()),
                        }
                    }
                }
            },
            Ngram(n) => println!("{}", get_ngram_info(&mut self.gen.data, &n.ngram)),
            Reload(_) => {
                let config = Config::with_loaded_weights();
                self.pins.clone_from(&config.pins);

                match LayoutGeneration::new(&self.language, "static", Some(config)) {
                    Ok(generator) => {
                        self.gen = generator;
                        self.saved = self
                            .gen
                            .load_layouts("static/layouts", &self.language)
                            .map_err(|e| e.to_string())?;
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }
            Quit(_) => {
                println!("Exiting analyzer...");
                return Ok(true);
            }
        };

        Ok(false)
    }
}
