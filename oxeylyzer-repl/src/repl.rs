use std::io::Write;
use std::path::Path;

use getargs::Options;
use indexmap::IndexMap;
use itertools::Itertools;
use oxeylyzer_core::{generate::LayoutGeneration, layout::*, load_text, weights::Config};

use crate::commands::*;
use crate::corpus_transposition::CorpusConfig;
use crate::tui::*;
use ArgumentType::*;

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
        let config = Config::new();
        let language = config.defaults.language.clone();
        let pins = config.pins.clone();

        let mut gen = LayoutGeneration::new(
            config.defaults.language.clone().as_str(),
            generator_base_path.as_ref(),
            Some(config),
        )
        .expect(format!("Could not read language data for {}", language).as_str());

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
            let line = readline()?;
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
        self.analyze(&l);
    }

    fn placeholder_name(&self, layout: &FastLayout) -> Result<String, String> {
        for i in 1..1000usize {
            let new_name_bytes = layout.matrix[10..14]
                .into_iter()
                .map(|b| *b)
                .collect::<Vec<u8>>();
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
            n.replace(" ", "_")
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
        f.write(layout_formatted.as_bytes()).unwrap();

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
                "Sfb:               {: <11} Sfb:               {:.3}%\n",
                "Dsfb:              {: <11} Dsfb:              {:.3}%\n",
                "Finger Speed:      {: <11} Finger Speed:      {:.3}\n",
                "Scissors           {: <11} Scissors:          {:.3}%\n",
                "Lsbs               {: <11} Lsbs:              {:.3}%\n\n",
                "Inrolls:           {: <11} Inrolls:           {:.2}%\n",
                "Outrolls:          {: <11} Outrolls:          {:.2}%\n",
                "Total Rolls:       {: <11} Total Rolls:       {:.2}%\n",
                "Onehands:          {: <11} Onehands:          {:.3}%\n\n",
                "Alternates:        {: <11} Alternates:        {:.2}%\n",
                "Alternates Sfs:    {: <11} Alternates Sfs:    {:.2}%\n",
                "Total Alternates:  {: <11} Total Alternates:  {:.2}%\n\n",
                "Redirects:         {: <11} Redirects:         {:.3}%\n",
                "Redirects Sfs:     {: <11} Redirects Sfs:     {:.3}%\n",
                "Bad Redirects:     {: <11} Bad Redirects:     {:.3}%\n",
                "Bad Redirects Sfs: {: <11} Bad Redirects Sfs: {:.3}%\n",
                "Total Redirects:   {: <11} Total Redirects:   {:.3}%\n\n",
                "Bad Sfbs:          {: <11} Bad Sfbs:          {:.3}%\n",
                "Sft:               {: <11} Sft:               {:.3}%\n\n",
                "Score:             {: <11} Score:             {:.3}\n"
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

    fn get_nth(&self, nr: usize) -> Option<FastLayout> {
        if nr < self.temp_generated.len() {
            let l = self.temp_generated[nr].clone();
            Some(l)
        } else {
            if self.temp_generated.len() == 0 {
                println!("You haven't generated any layouts yet!");
            } else {
                println!("That's not a valid index!");
            }
            None
        }
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
        let args = shlex::split(line).ok_or("error: Invalid quoting")?;
        let mut args = Options::new(args.iter().map(String::as_str));

        match args.next_positional() {
            Some("generate") | Some("gen") | Some("g") => {
                if let Some(count_str) = args.next_positional() {
                    if let Ok(count) = usize::from_str_radix(count_str, 10) {
                        println!("generating {} layouts...", count_str);
                        self.temp_generated = generate_n(&self.gen, count);
                    } else {
                        print_error("generate", &[R("amount")]);
                    }
                }
            }
            Some("improve") | Some("i") => {
                if let Some(name) = args.next_positional() {
                    if let Some(amount_str) = args.next_positional() {
                        if let Ok(amount) = usize::from_str_radix(amount_str, 10) {
                            if let Some(l) = self.layout_by_name(name) {
                                self.temp_generated = generate_n_with_pins(&self.gen, amount, l.clone(), &self.pins);
                            } else {
                                println!("'{name}' does not exist!")
                            }
                        } else {
                            print_error("improve", &[R("name"), R("amount")]);
                        }
                    }
                }
            }
            Some("rank") => self.rank(),
            Some("analyze") | Some("layout") | Some("a") => {
                if let Some(name_or_nr) = args.next_positional() {
                    if let Ok(nr) = usize::from_str_radix(name_or_nr, 10) {
                        if let Some(layout) = self.get_nth(nr) {
                            self.analyze(&layout);
                        }
                    } else {
                        self.analyze_name(name_or_nr);
                    }
                } else {
                    print_error("analyze", &[R("name or number")]);
                }
            }
            Some("compare") | Some("c") | Some("comp") | Some("cmopare") | Some("comprae") => {
                if let Some(layout1) = args.next_positional() {
                    if let Some(layout2) = args.next_positional() {
                        self.compare_name(layout1, layout2);
                    } else {
                        print_error("compare", &[R("layout 1"), R("layout 2")]);
                    }
                }
            }
            Some("sfbs") | Some("sfb") => {
                if let Some(name) = args.next_positional() {
                    if let Some(top_n_str) = args.next_positional() {
                        if let Ok(top_n) = usize::from_str_radix(top_n_str, 10) {
                            self.sfbs(name, top_n)
                        } else {
                            print_error("ngram", &[R("name"), O("top n")]);
                        }
                    } else {
                        self.sfbs(name, 10);
                    }
                } else {
                    print_error("ngram", &[R("name"), O("top n")]);
                }
            }
            Some("ngram") | Some("occ") | Some("n") => {
                if let Some(ngram) = args.next_positional() {
                    println!("{}", get_ngram_info(&mut self.gen.data, ngram));
                } else {
                    print_error("ngram", &[R("ngram")]);
                }
            }
            Some("load") => {
                use getargs::Opt::*;
                let opt1 = args.next_opt();

                if matches!(opt1, Ok(Some(Short('a'))) | Ok(Some(Long("all")))) {
                    for (language, config) in CorpusConfig::all() {
                        println!("loading data for language: {language}...");
                        load_text::load_data(language.as_str(), config.translator())
                            .map_err(|e| e.to_string())?;
                    }
                } else if let Some(language) = args.next_positional() {
                    let opt2 = args.next_opt();
                    if matches!(opt1, Ok(Some(Short('r'))) | Ok(Some(Long("raw"))))
                    || matches!(opt2, Ok(Some(Short('r'))) | Ok(Some(Long("raw")))) {
                        println!("loading raw data for language: {language}...");
                        load_text::load_raw(language);
                    } else {
                        let preferred_folder = args.next_positional();
                        let translator = CorpusConfig::new_translator(language, preferred_folder);
                        let is_raw_translator = translator.is_raw;

                        println!("loading data for {language}...");
                        load_text::load_data(language, translator)
                            .map_err(|e| e.to_string())?;

                        if !is_raw_translator {
                            let config = Config::new();
                            if let Ok(generator) = LayoutGeneration::new(
                                language,
                                "static",
                                Some(config)
                            ) {
                                self.language = language.to_string();
                                self.gen = generator;
                                self.saved = self.gen.load_layouts(
                                    "static/layouts",
                                    language
                                ).expect("couldn't load layouts lol");

                                println!(
                                    "Set language to {}. Sfr: {:.2}%",
                                    language, self.sfr_freq() * 100.0
                                );
                            } else {
                                println!("Could not load data for {language}");
                            }
                        }
                    }
                } else {
                    print_error(
                        "load",
                        &[R("language"), O("preferred_config_folder"), A("raw")]
                    );
                }
            }
            Some("language") | Some("lanugage") | Some("langauge") | Some("lang") | Some("l") => {
                match args.next_positional() {
                    Some(language) => {
                        let config = Config::new();
                        if let Ok(generator) = LayoutGeneration::new(
                            language,
                            "static",
                            Some(config)
                        ) {
                            self.language = language.to_string();
                            self.gen = generator;
                            self.saved = self.gen.load_layouts(
                                "static/layouts",
                                language
                            ).expect("couldn't load layouts lol");

                            println!(
                                "Set language to {}. Sfr: {:.2}%",
                                language, self.sfr_freq() * 100.0
                            );
                        } else {
                            println!("Could not load data for {language}");
                        }
                    }
                    None => println!("Current language: {}", self.language)
                }
            }
            Some("languages") | Some("langs") => {
                std::fs::read_dir("static/language_data")
                    .unwrap()
                    .flatten()
                    .map(|p| p
                        .file_name()
                        .to_string_lossy()
                        .replace("_", " ")
                        .replace(".json", "")
                    )
                    .filter(|n| n != "test")
                    .for_each(|n| println!("{n}"))
            }
            Some("reload") | Some("r") => {
                let config = Config::new();
                self.pins = config.pins.clone();

                if let Ok(generator) = LayoutGeneration::new(
                    self.language.as_str(),
                    "static",
                    Some(config)
                ) {
                    self.gen = generator;
                    self.saved = self.gen.load_layouts(
                        "static/layouts",
                        self.language.as_str()
                    ).expect("couldn't load layouts lol");
                } else {
                    println!("Could not load {}", self.language);
                }
            }
            Some("save") | Some("s") => {
                if let Some(n_str) = args.next_positional() {
                    if let Ok(nr) = usize::from_str_radix(n_str, 10) {
                        if let Some(layout) = self.get_nth(nr) {
                            let name = args.next_positional().map(str::to_string);
                            self.save(layout, name).unwrap();
                        }
                    } else {
                        print_error("save", &[R("index"), O("name")])
                    }
                }
            }
            Some("quit") | Some("exit") | Some("q") => {
                println!("Exiting analyzer...");
                return Ok(true)
            }
            Some("help") | Some("--help") | Some("h") | Some("-h") => {
                match args.next_positional() {
                    Some("generate") | Some("gen") | Some("g") => {
                        print_help(
                            "generate", 
                            "(g, gen) Generate a number of layouts and shows the best 10, All layouts generated are accessible until reloading or quiting.",
                            &[R("amount")]
                        )
                    }
                    Some("improve") | Some("i") => {
                        print_help(
                            "improve",
                            "(i) Save the top <number> result that was generated.",
                            &[R("name"), R("amount")]
                        )
                    }
                    Some("rank") => {
                        print_help(
                            "rank",
                            "(sort) Rank all layouts in set language by score using values set from 'config.toml'",
                            &[]
                        )
                    }
                    Some("analyze") | Some("layout") | Some("a") => {
                        print_help(
                            "analyze",
                            "(a, layout) Show details of layout.",
                            &[R("name or number")]
                        )
                    }
                    Some("compare") | Some("c") | Some("cmp") | Some("cmopare") | Some("comprae") => {
                        print_help(
                            "compare",
                            "(c, cmp) Compare 2 layouts.",
                            &[R("layout 1"), R("layout 2")]
                        )
                    }
                    Some("sfbs") | Some("sfb") => {
                        print_help(
                            "sfbs",
                            "(sfbs, sfb) Shows the top n sfbs for a certain layout.",
                            &[R("name"), O("top n")]
                        )
                    }
                    Some("ngram") | Some("occ") | Some("n") => {
                        print_help(
                            "ngram",
                            "(n, occ) Gives information about a certain ngram. for 2 letter ones, skipgram info will be provided as well.",
                            &[R("ngram")]
                        )
                    }
                    Some("load") => {
                        print_help(
                            "load",
                            "Generates corpus for <language>. Will be include everything but spaces if the language is not known.",
                            &[R("language"), O("preferred_config_folder"), A("raw")]
                        )
                    }
                    Some("language") | Some("lanugage") | Some("langauge") | Some("lang") | Some("l") => {
                        print_help(
                            "language",
                            "(l, lang) Sets a language to be used for analysis.",
                            &[R("language")]
                        )
                    }
                    Some("languages") | Some("langs") => {
                        print_help(
                            "languages",
                            "(langs) Shows available languages.",
                            &[]
                        )
                    }
                    Some("reload") | Some("r") => {
                        print_help(
                            "reload",
                            "(r) Reloads all data with the current language. Loses temporary layouts.",
                            &[]
                        )
                    }
                    Some("save") | Some("s") => {
                        print_help(
                            "save",
                            "(s) Saves the top <number> result that was generated. Starts from 0 up to the number generated.",
                            &[R("index"), O("name")]
                        )
                    }
                    Some("quit") | Some("exit") | Some("q") => {
                        print_help(
                            "quit",
                            "(q) Quit the repl",
                            &[]
                        )
                    }
                    Some("help") | Some("--help") | Some("h") | Some("-h") => {
                        print_help(
                            "help",
                            "Print this message or the help of the given subcommand(s)",
                            &[O("subcommand")]
                        )
                    }
                    Some(c) => println!("error: the subcommand '{c}' wasn't recognized"),
                    None => {
                        println!(concat!(
                            "commands:\n",
                            "    analyze      (a, layout) Show details of layout\n",
                            "    compare      (c, comp) Compare 2 layouts\n",
                            "    generate     (g, gen) Generate a number of layouts and shows the best 10, All layouts\n",
                            "                     generated are accessible until reloading or quiting.\n",
                            "    help         Print this message or the help of the given subcommand(s)\n",
                            "    improve      (i, optimize) Save the top <NR> result that was generated. Starts from 1, Takes\n",
                            "                     negative values\n",
                            "    language     (l, lang) Set a language to be used for analysis. Loads corpus when not present\n",
                            "    languages    (langs) Show available languages\n",
                            "    load         Generates corpus for <language>. Will be exclude spaces from source if the\n",
                            "                     language isn't known\n",
                            "    ngram        (occ) Gives information about a certain ngram. for 2 letter ones, skipgram info\n",
                            "                     will be provided as well.\n",
                            "    quit         (q) Quit the repl\n",
                            "    rank         (sort) Rank all layouts in set language by score using values set from\n",
                            "                     'config.toml'\n",
                            "    reload       (r) Reloads all data with the current language. Loses temporary layouts.\n",
                            "    save         (s) Save the top <NR> result that was generated. Starts from 1 up to the number\n",
                            "                     generated, Takes negative values\n"
                        ));
                    }
                }
            }
            Some(c) => println!("error: the command '{c}' wasn't recognized"),
            None => {}
        }

        Ok(false)
    }
}
