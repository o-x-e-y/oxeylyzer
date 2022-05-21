use std::io::Write;
use clap::{arg, command, Command};
use crate::generate::LayoutGeneration;

pub struct Repl {
    language: String,
    gen: LayoutGeneration
}

impl Repl {
    pub fn run() -> Result<(), String> {
        let mut env = Self {
            language: "english".to_string(),
            gen: LayoutGeneration::new("english")
        };

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

    fn respond(&mut self, line: &str) -> Result<bool, String> {
        let args = shlex::split(line).ok_or("error: Invalid quoting")?;
        let matches = self.cli()
            .try_get_matches_from(&args)
            .map_err(|e| e.to_string())?;
        match matches.subcommand() {
            Some(("generate", new_m)) => {
                println!("generating {} layouts...", new_m.value_of("COUNT").unwrap());
                let count_str = new_m.value_of("COUNT").unwrap();
                let count = usize::from_str_radix(count_str, 10).map_err(|e| e.to_string())?;
                self.gen.generate_n(count);
            }
            Some(("rank", _new_m)) => {
                self.gen.analysis.rank();
            }
            Some(("layout", new_m)) => {
                let name = new_m.value_of("LAYOUT_NAME").unwrap();
                self.gen.analysis.analyze_name(name);
            }
            Some(("compare", new_m)) => {
                let layout1 = new_m.value_of("LAYOUT_1").unwrap();
                let layout2 = new_m.value_of("LAYOUT_2").unwrap();
                self.gen.analysis.compare_name(layout1, layout2);
            }
            Some(("language", lang_m)) => {
                match lang_m.value_of("LANGUAGE") {
                    Some(language) => {
                        self.language = language.to_string();
                        self.gen = LayoutGeneration::new(language);
                        println!("Set language to {}", language);
                    },
                    None => println!("Current language: {}", self.language)
                }
            }
            Some(("quit", _new_m)) => {
                println!("Exiting anlyzer...");
                return Ok(true);
            }
            Some((name, _new_m)) => unimplemented!("{}", name),
            None => unreachable!("subcommand required"),
        }

        Ok(false)
    }

    fn cli(&self) -> Command<'static> {
        // strip out usage
        const PARSER_TEMPLATE: &str = "\
            {all-args}
        ";
        // strip out name/version
        const APPLET_TEMPLATE: &str = "\
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
            .help_template(PARSER_TEMPLATE)
            .subcommand(
                command!("quit")
                    .alias("exit")
                    .about("Quit the repl")
                    .help_template(APPLET_TEMPLATE),
            )
            .subcommand(
                command!("rank")
                    .alias("r")
                    .about("Rank all layouts in set language by score")
                    .help_template(APPLET_TEMPLATE),
            )
            .subcommand(
                command!("layout")
                    .alias("l")
                    .alias("analyze")
                    .alias("a")
                    .arg(
                        arg!(<LAYOUT_NAME>)
                    )
                    .about("Show details of layout")
                    .help_template(APPLET_TEMPLATE)
            )
            .subcommand(
                command!("compare")
                    .alias("c")
                    .arg(
                        arg!(<LAYOUT_1>)
                    )
                    .arg(
                        arg!(<LAYOUT_2>)
                    )
                    .about("Compare 2 layouts")
                    .help_template(APPLET_TEMPLATE)
            )
            .subcommand(
                command!("language")
                    .alias("lang")
                    .arg(   
                        arg!([LANGUAGE])
                    )
                    .help_template(APPLET_TEMPLATE)
                    .about("Set a language to be used for analysis. Loads corpus when not present")
            )
            .subcommand(
                command!("generate")
                    .alias("gen")
                    .alias("g")
                    .arg(
                        arg!([COUNT])
                        .default_value("100")
                    )
                    .help_template(APPLET_TEMPLATE)
                    .about("Generate a number of layouts and take the best 10")
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