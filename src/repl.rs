use std::io::Write;
use std::path::Path;

use getargs::Options;
use indexmap::IndexMap;
use oxeylyzer::{
    generate::LayoutGeneration,
    layout::*,
    weights::Config,
    load_text
};

use crate::tui::*;
use ArgumentType::*;

pub struct Repl {
    language: String,
    gen: LayoutGeneration,
    saved: IndexMap<String, FastLayout>,
    temp_generated: Vec<FastLayout>,
    pins: Vec<usize>
}

impl Repl {
    pub fn new<P>(generator_base_path: P) -> Result<Self, String>
        where P: AsRef<Path> {
        let config = Config::new();
        let language = config.defaults.language.clone();

        let mut gen = LayoutGeneration::new(
            config.defaults.language.as_str(),
            generator_base_path.as_ref(),
            config.trigram_precision(),
            Some(config.weights),
        ).expect(format!("Could not read language data for {}", language).as_str());

        Ok(Self {
            saved: gen.load_layouts(
                generator_base_path.as_ref().join("layouts"),
                language.as_str())
                .map_err(|e| e.to_string())?,
            language,
            gen,
            temp_generated: Vec::new(),
            pins: config.pins
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

            match env.respond2(line) {
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
    		let mut new_name = layout.matrix[10..14].iter().collect::<String>();
			
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
		
		let layout_formatted = layout.to_string();
		println!("saved {}\n{}", new_name, layout_formatted);
		f.write(layout_formatted.as_bytes()).unwrap();

		layout.score = self.gen.score(&layout);
		self.saved.insert(new_name, layout);
		self.saved.sort_by(|_, a, _, b| {
			a.score.partial_cmp(&b.score).unwrap()
		});

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
		println!("\n{:29}{}", name1, name2);
		for y in 0..3 {
			for (n, layout) in [l1, l2].into_iter().enumerate() {
				for x in 0..10 {
					print!("{} ", heatmap_heat(&self.gen.data, &layout.c(x + 10*y)));
					if x == 4 {
						print!(" ");
					}
				}
				if n == 0 {
					print!("        ");
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
			"Sfb:              {: <10} Sfb:              {:.3}%\n",
			"Dsfb:             {: <10} Dsfb:             {:.3}%\n",
			"Finger Speed:     {: <10} Finger Speed:     {:.3}\n",
			"Scissors          {: <10} Scissors:         {:.3}%\n\n",
			"Inrolls:          {: <10} Inrolls:          {:.2}%\n",
			"Outrolls:         {: <10} Outrolls:         {:.2}%\n",
			"Total Rolls:      {: <10} Total Rolls:      {:.2}%\n",
			"Onehands:         {: <10} Onehands:         {:.3}%\n\n",
			"Alternates:       {: <10} Alternates:       {:.2}%\n",
			"Alternates (sfs): {: <10} Alternates (sfs): {:.2}%\n",
			"Total Alternates: {: <10} Total Alternates: {:.2}%\n\n",
			"Redirects:        {: <10} Redirects:        {:.2}%\n",
			"Bad Redirects:    {: <10} Bad Redirects:    {:.2}%\n",
			"Total Redirects:  {: <10} Total Redirects:  {:.2}%\n\n",
			"Bad Sfbs:         {: <10} Bad Sfbs:         {:.2}%\n",
			"Sft:              {: <10} Sft:              {:.3}%\n\n",
			"Score:            {: <10} Score:            {:.3}\n"
		),
			format!("{:.3}%", s1.sfb*100.0), s2.sfb*100.0,
			format!("{:.3}%", s1.dsfb*100.0), s2.dsfb*100.0,
			format!("{:.3}%", s1.fspeed * 100.0), s2.fspeed * 100.0,
			format!("{:.3}", s1.scissors*100.0), s2.scissors*100.0,
			format!("{:.2}%", ts1.inrolls*100.0), ts2.inrolls*100.0,
			format!("{:.2}%", ts1.outrolls*100.0), ts2.outrolls*100.0,
			format!("{:.2}%", (ts1.inrolls + ts1.outrolls)*100.0), (ts2.inrolls + ts2.outrolls)*100.0,
			format!("{:.3}%", ts1.onehands*100.0), ts2.onehands*100.0,
			format!("{:.2}%", ts1.alternates*100.0), ts2.alternates*100.0,
			format!("{:.2}%", ts1.alternates_sfs*100.0), ts2.alternates_sfs*100.0,
			format!("{:.2}%", (ts1.alternates + ts1.alternates_sfs)*100.0), (ts2.alternates + ts2.alternates_sfs)*100.0,
			format!("{:.3}%", ts1.redirects*100.0), ts2.redirects*100.0,
			format!("{:.3}%", ts1.bad_redirects*100.0), ts2.bad_redirects*100.0,
			format!("{:.3}%", (ts1.redirects + ts1.bad_redirects)*100.0), (ts2.redirects + ts2.bad_redirects)*100.0,
			format!("{:.3}%", ts1.bad_sfbs*100.0), ts2.bad_sfbs*100.0,
			format!("{:.3}%", ts1.sfts*100.0), ts2.sfts*100.0,
			format!("{:.3}", l1.score), l2.score
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

    pub fn double_freq(&self) -> f64 {
        self.gen.data.bigrams
            .iter()
            .filter(|(bg, _)| bg[0] == bg[1] )
            .map(|(_, &f)| f)
            .sum::<f64>()
    }

    fn respond2(&mut self, line: &str) -> Result<bool, String> {
        let args = shlex::split(line).ok_or("error: Invalid quoting")?;
        let mut opts = Options::new(args.iter().map(String::as_str));
        match opts.next_positional() {
            Some("generate") | Some("gen") | Some("g") => {
                if let Some(count_str) = opts.next_positional()
                && let Ok(count) = usize::from_str_radix(count_str, 10) {
                    println!("generating {} layouts...", count_str);
                    self.temp_generated = generate_n(&self.gen, count);
                } else {
                    print_error("generate", &[R("amount")]);
                }
            }
            Some("improve") | Some("i") => {
                if let Some(name) = opts.next_positional()
                && let Some(amount_str) = opts.next_positional()
                && let Ok(amount) = usize::from_str_radix(amount_str, 10) {
                    if let Some(l) = self.layout_by_name(name) {
                        generate_n_with_pins(&self.gen, amount, l.clone(), &self.pins);
                    } else {
                        println!("'{name}' does not exist!")
                    }
                } else {
                    print_error("improve", &[R("name"), R("amount")]);
                }
            }
            Some("rank") => self.rank(),
            Some("analyze") | Some("layout") | Some("a") => {
                if let Some(name_or_nr) = opts.next_positional() {
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
                if let Some(layout1) = opts.next_positional()
                && let Some(layout2) = opts.next_positional() {
                    self.compare_name(layout1, layout2);
                } else {
                    print_error("compare", &[R("layout 1"), R("layout 2")]);
                }
            }
            Some("ngram") | Some("occ") | Some("n") => {
                if let Some(ngram) = opts.next_positional() {
                    println!("{}", get_ngram_info(&self.gen.data, ngram));
                } else {
                    print_error("ngram", &[R("ngram")]);
                }
            }
            Some("load") => {
                if let Some(language) = opts.next_positional() {
                    load_text::load_default(language);
                }
            }
            Some("language") | Some("lanugage") | Some("langauge") | Some("lang") | Some("l") => {
                let config = Config::new();
                match opts.next_positional() {
                    Some(language) => {
                        if let Ok(generator) = LayoutGeneration::new(
                            language,
                            "static",
                            config.trigram_precision(),
                            Some(config.weights)
                        ) {
                            self.language = language.to_string();
                            self.gen = generator;
                            
                            println!(
                                "Set language to {}. Sfr: {:.2}%",
                                language, self.double_freq() * 100.0
                            );
                        } else {
                            println!("Could not load {}", language);
                        }
                    }
                    None => println!("Current language: {}", self.language)
                }
            }
            Some("languages") | Some("langs") => {
                for entry in std::fs::read_dir("static/language_data").unwrap() {
                    if let Ok(p) = entry {
                        let name = p
                            .file_name()
                            .to_string_lossy()
                            .replace("_", " ")
                            .replace(".json", "");
                        if name != "test" {
                            println!("{}", name);
                        }
                    }
                }
            }
            Some("reload") | Some("r") => {
                let config = Config::new();

                if let Ok(generator) = LayoutGeneration::new(
                    self.language.as_str(),
                    "static",
                    config.trigram_precision(),
                    Some(config.weights)
                ) {
                    self.gen = generator;
                    self.pins = config.pins;
                } else {
                    println!("Could not load {}", self.language);
                }
            }
            Some("save") | Some("s") => {
                if let Some(n_str) = opts.next_positional()
                && let Ok(nr) = usize::from_str_radix(n_str, 10) {
                    if let Some(layout) = self.get_nth(nr) {
                        let name = opts.next_positional().map(str::to_string);
                        self.save(layout, name).unwrap();
                    }
                } else {
                    print_error("save", &[R("index"), O("name")])
                }
            }
            Some("quit") | Some("exit") | Some("q") => {
                println!("Exiting analyzer...");
                return Ok(true)
            }
            Some("help") | Some("--help") | Some("h") | Some("-h") => {
                match opts.next_positional() {
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
                            &[R("language")]
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
                            "(s) Saves the top <number> result that was generated. Starts from 1 up to the number generated.",
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
                            "commands:",
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