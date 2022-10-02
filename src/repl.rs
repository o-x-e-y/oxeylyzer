use std::io::Write;
use std::path::Path;

use clap::{arg, command, Command};
use indexmap::IndexMap;
use oxeylyzer::{
    generate::LayoutGeneration,
    layout::*,
    weights::Config,
    load_text
};

use crate::tui::*;

pub struct Repl {
    language: String,
    gen: LayoutGeneration,
    saved: IndexMap<String, FastLayout>,
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

            match env.respond(line) {
                Ok(quit) => {
                    if quit {
                        break;
                    }
                }
                Err(err) => {
                    write!(std::io::stdout(), "{}", err).map_err(|e| e.to_string())?;
                    std::io::stdout().flush().map_err(|e| e.to_string())?;
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

	// pub fn analyze_str(&mut self, layout_str: &str) {
	// 	let layout_str = Self::format_layout_str(layout_str.to_string());
	// 	let layout = FastLayout::try_from(layout_str.as_str()).unwrap();
	// 	self.analyze(&layout);
	// }

	pub fn save(
        &mut self, mut layout: FastLayout, name: Option<String>
    ) -> Result<(), String> {
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
        if let Some(temp_list) = &self.gen.temp_generated {
            if nr < temp_list.len() {
                let l = temp_list[nr].clone();
                Some(l)
            } else {
                println!("That's not a valid index!");
                None
            }
        } else {
            println!("You haven't generated any layouts yet!");
            None
        }
    }

    fn save_match(&mut self, save_m: &clap::ArgMatches) {
        let n_str = save_m.value_of("NR").unwrap();
        if let Ok(nr) = usize::from_str_radix(n_str, 10) {
            if let Some(layout) = self.get_nth(nr) {
                if let Some(name) = save_m.value_of("NAME") {
                    self.save(layout, Some(name.to_string())).unwrap();
                } else {
                    self.save(layout, None).unwrap();
                }
            }
        }
    }

    fn respond(&mut self, line: &str) -> Result<bool, String> {
        let args = shlex::split(line).ok_or("error: Invalid quoting")?;
        let matches = self.cli()
            .try_get_matches_from(&args)
            .map_err(|e| e.to_string())?;
        match matches.subcommand() {
            Some(("generate", new_m)) => {
                let count_str = new_m.value_of("COUNT").unwrap();
                println!("generating {} layouts...", count_str);
                let count = usize::from_str_radix(count_str, 10).map_err(|e| e.to_string())?;
                generate_n(&self.gen, count);
            }
            Some(("improve", comp_m)) => {
                let name = comp_m.value_of("LAYOUT_NAME").unwrap();
                let amount_str = comp_m.value_of("AMOUNT").unwrap();
                if let Ok(amount) = usize::from_str_radix(amount_str, 10) {
                    if let Some(l) = self.layout_by_name(name) {
                        generate_n_with_pins(&self.gen, amount, l.clone(), &self.pins);
                    }
                }
            }
            Some(("rank", _)) => {
                self.rank();
            }
            Some(("analyze", new_m)) => {
                let name_or_nr = new_m.value_of("LAYOUT_NAME_OR_NR").unwrap();
                if let Ok(nr) = usize::from_str_radix(name_or_nr, 10) {
                    if let Some(layout) = self.get_nth(nr) {
                        self.analyze(&layout);
                    }
                } else {
                    self.analyze_name(name_or_nr);
                }
            }
            Some(("ngram", occ_m)) => {
                let ngram = occ_m.value_of("NGRAM").unwrap();
                println!("{}", get_ngram_info(&self.gen.data, ngram));
            }
            Some(("compare", new_m)) => {
                let layout1 = new_m.value_of("LAYOUT_1").unwrap();
                let layout2 = new_m.value_of("LAYOUT_2").unwrap();
                self.compare_name(layout1, layout2);
            }
            Some(("language", lang_m)) => {
                let config = Config::new();

                match lang_m.value_of("LANGUAGE") {
                    Some(language) => {
                        if let Ok(generator) = LayoutGeneration::new(
                            language,
                            "static",
                            config.trigram_precision(),
                            Some(config.weights)
                        ) {
                            self.language = language.to_string();
                            self.gen = generator;
                            
                            let double_freq = self.gen.data.bigrams
                                .iter()
                                .filter(|(bg, _)| bg[0] == bg[1] )
                                .map(|(_, &f)| f)
                                .sum::<f64>();
                            println!(
                                "Set language to {}. Sfr: {:.2}%",
                                language, double_freq * 100.0
                            );
                        } else {
                            println!("Could not load {}", language);
                        }
                    },
                    None => println!("Current language: {}", self.language)
                }
            }
            Some(("languages", _)) => {
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
            Some(("reload", _)) => {
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
            Some(("save", save_m)) => {
                self.save_match(save_m);
            }
            Some(("load", load_m)) => {
                if let Some(language) = load_m.value_of("LANGUAGE") {
                    load_text::load_default(language);
                }
            }
            Some(("quit", _)) => {
                println!("Exiting anlyzer...");
                return Ok(true);
            }
            Some((name, _new_m)) => println!("{name} is not a valid command!"),
            None => unreachable!("subcommand required"),
        }

        Ok(false)
    }

    fn cli(&self) -> Command<'static> {
        // strip out usage
        const REPL_TEMPLATE: &str = "\
            {all-args}
        ";
        // strip out name/version
        const COMMAND_TEMPLATE: &str = "\
            {about-with-newline}\n\
            {usage-heading}\n    {usage}\n\
            \n\
            {all-args}{after-help}\
        ";

        command!("repl")
            .multicall(true)
            .arg_required_else_help(true)
            .subcommand_required(true)
            .subcommand_value_name("APPLET")
            .subcommand_help_heading("APPLETS")
            .help_template(REPL_TEMPLATE)
            .subcommand(
                command!("rank")
                    .alias("sort")
                    .about("(sort) Rank all layouts in set language by score using values set from 'config.toml'")
                    .help_template(COMMAND_TEMPLATE),
            )
            .subcommand(
                command!("analyze")
                    .aliases(&["a", "layout"])
                    .arg(
                        arg!(<LAYOUT_NAME_OR_NR>)
                    )
                    .about("(a, layout) Show details of layout")
                    .help_template(COMMAND_TEMPLATE)
            )
            .subcommand(
                command!("compare")
                    .aliases(&["c", "comp", "cmopare", "comprae"])
                    .arg(
                        arg!(<LAYOUT_1>)
                    )
                    .arg(
                        arg!(<LAYOUT_2>)
                    )
                    .about("(c, comp) Compare 2 layouts")
                    .help_template(COMMAND_TEMPLATE)
            )
            .subcommand(
                command!("language")
                    .aliases(&["l", "lang", "lanugage", "langauge"])
                    .arg(
                        arg!([LANGUAGE])
                    )
                    .help_template(COMMAND_TEMPLATE)
                    .about("(l, lang) Set a language to be used for analysis. Loads corpus when not present")
            )
            .subcommand(
                command!("languages")
                .aliases(&["langs", "lanugages", "langauges"])
                .help_template(COMMAND_TEMPLATE)
                .about("(langs) Show available languages")
            )
            .subcommand(
                command!("ngram")
                .aliases(&["n","occ"])
                .help_template(COMMAND_TEMPLATE) 
                .arg(
                        arg!(<NGRAM>)
                )
                .about("(occ) Gives information about a certain ngram. for 2 letter ones, skipgram info will be provided as well.")
            )
            .subcommand(
                command!("reload")
                .alias("r")
                .help_template(COMMAND_TEMPLATE)
                .about("(r) Reloads all data with the current language. Loses temporary layouts. ")
            )
            .subcommand(
                command!("generate")
                    .aliases(&["g", "gen"])
                    .arg(
                        arg!(<COUNT>)
                    )
                    .help_template(COMMAND_TEMPLATE)
                    .about("(g, gen) Generate a number of layouts and shows the best 10, All layouts generated are accessible until reloading or quiting. ")
            )
            .subcommand(
                command!("improve")
                    .aliases(&["i", "optimize"])
                    .arg(
                        arg!(<LAYOUT_NAME>)
                    )
                    .arg(
                        arg!(<AMOUNT>)
                    )
                    .help_template(COMMAND_TEMPLATE)
                    .about("(i, optimize) Save the top <NR> result that was generated. Starts from 1, Takes negative values")
            )
            .subcommand(
                command!("save")
                .alias("s")
                .arg(
                    arg!(<NR>)
                )
                .arg(
                    arg!([NAME])
                )
                .help_template(COMMAND_TEMPLATE)
                .about("(s) Save the top <NR> result that was generated. Starts from 1 up to the number generated, Takes negative values")
            )
            .subcommand(
                command!("load")
                .arg(
                    arg!(<LANGUAGE>)
                )
                .help_template(COMMAND_TEMPLATE)
                .about("Generates corpus for <language>. Will be exclude spaces from source if the language isn't known")
            )
            // .subcommand(
            //     command!("passthrough")
            //     .alias("pass")
            //     .arg(
            //         arg!(<LANGUAGE>)
            //     )
            //     .help_template(COMMAND_TEMPLATE)
            //     .about("Loads corpus as passthrough for <language> in static/language_data_pass")
            // )
            .subcommand(
                command!("quit")
                    .aliases(&["exit","q"])
                    .about("(q) Quit the repl")
                    .help_template(COMMAND_TEMPLATE),
            )
    }
}

fn readline() -> Result<String, String> {
    write!(std::io::stdout(), "> ").map_err(|e| e.to_string())?;
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let mut buf = String::new();
    std::io::stdin()
        .read_line(&mut buf)
        .map_err(|e| e.to_string())?;
    Ok(buf)
}
