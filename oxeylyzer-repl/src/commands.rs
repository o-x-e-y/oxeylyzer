#[derive(Debug)]
pub(crate) enum ArgumentType<'a> {
    R(&'a str),
    O(&'a str),
    A(&'a str),
}

impl<'a> ArgumentType<'a> {
    pub(crate) fn is_required(&self) -> bool {
        match *self {
            Self::R(_) => true,
            _ => false,
        }
    }

    pub(crate) fn parse(&self) -> String {
        match *self {
            Self::R(s) => format!("<{s}>"),
            Self::O(s) => format!("[{s}]"),
            Self::A(s) => {
                let first = s.chars().next().unwrap();
                format!("[-{first}/--{s}]")
            }
        }
    }
}

fn usage(command_name: &str, args: &[ArgumentType]) -> String {
    let args_left_right = args
        .into_iter()
        .map(ArgumentType::parse)
        .collect::<Vec<_>>()
        .join(" ");

    format!("USAGE:\n    {command_name} {args_left_right}")
}

pub(crate) fn print_help(command_name: &str, about: &str, args: &[ArgumentType]) {
    println!("{about}\n\n{}\n", usage(command_name, args));
}

pub(crate) fn print_error(command_name: &str, args: &[ArgumentType]) {
    let plural = if args.len() > 1 { "s were" } else { " was" };

    let args_top_down = args
        .into_iter()
        .filter(|a| a.is_required())
        .map(ArgumentType::parse)
        .collect::<Vec<_>>()
        .join("\n    ");

    println!(
        concat!(
            "error: The following required argument{} not provided:\n    {}\n\n{}",
            "\n\nFor more information try 'help'"
        ),
        plural,
        args_top_down,
        usage(command_name, args)
    );
}

// #[derive(Debug)]
// pub(crate) struct ReplCommand<'a> {
//     names: &'a [&'a str],
//     arguments: &'a [ArgumentType<'a>],
//     about: &'a str
// }

// impl<'a> ReplCommand<'a> {
//     pub const fn new(
//         names: &'a [&'a str], arguments: &'a [ArgumentType<'a>], about: &'a str
//     ) -> Self {
//         ReplCommand { names, arguments, about }
//     }

//     pub fn name(&self) -> &'a str {
//         self.names[0]
//     }

//     pub fn about(&self) -> &'a str {
//         self.about
//     }

//     pub fn arguments(&self) -> &'a [ArgumentType<'a>] {
//         self.arguments
//     }

//     pub fn aliases(&self) -> &'a [&'a str] {
//         self.names
//     }

//     pub fn r#match(&self, opt: Option<&str>) -> bool {
//         if let Some(m) = opt {
//             self.names.contains(&m)
//         } else {
//             false
//         }
//     }
// }

// use ArgumentType::*;

// macro_rules! create_command_helper {
//     () => {
//         Some("a") | Some("b")
//     }
// }

// // fn something() {
// //     let x = "a";
// //     match x {
// //         Some("c")
// //     }
// // }

// const COMPARE_COMMAND: ReplCommand = ReplCommand::new(
//     &["compare", "c", "comp", "cmopare", "comprae"],
//     &[R("layout 1"), R("layout 2")],
//     "(c, cmp) Compare 2 layouts."
// );

//         Some("generate") | Some("gen") | Some("g") => {
//             if let Some(count_str) = opts.next_positional()
//             && let Ok(count) = usize::from_str_radix(count_str, 10) {
//                 println!("generating {} layouts...", count_str);
//                 self.temp_generated = generate_n(&self.gen, count);
//             } else {
//                 print_error("generate", &[R("amount")]);
//             }
//         }
//         Some("improve") | Some("i") => {
//             if let Some(name) = opts.next_positional()
//             && let Some(amount_str) = opts.next_positional()
//             && let Ok(amount) = usize::from_str_radix(amount_str, 10) {
//                 if let Some(l) = self.layout_by_name(name) {
//                     generate_n_with_pins(&self.gen, amount, l.clone(), &self.pins);
//                 } else {
//                     println!("'{name}' does not exist!")
//                 }
//             } else {
//                 print_error("improve", &[R("name"), R("amount")]);
//             }
//         }
//         Some("rank") => self.rank(),
//         Some("analyze") | Some("layout") | Some("a") => {
//             if let Some(name_or_nr) = opts.next_positional() {
//                 if let Ok(nr) = usize::from_str_radix(name_or_nr, 10) {
//                     if let Some(layout) = self.get_nth(nr) {
//                         self.analyze(&layout);
//                     }
//                 } else {
//                     self.analyze_name(name_or_nr);
//                 }
//             } else {
//                 print_error("analyze", &[R("name or number")]);
//             }
//         }
//         Some("compare") | Some("c") | Some("comp") | Some("cmopare") | Some("comprae") => {
//             if let Some(layout1) = opts.next_positional()
//             && let Some(layout2) = opts.next_positional() {
//                 self.compare_name(layout1, layout2);
//             } else {
//                 print_error("compare", &[R("layout 1"), R("layout 2")]);
//             }
//         }
//         Some("ngram") | Some("occ") | Some("n") => {
//             if let Some(ngram) = opts.next_positional() {
//                 println!("{}", get_ngram_info(&self.gen.data, ngram));
//             } else {
//                 print_error("ngram", &[R("ngram")]);
//             }
//         }
//         Some("load") => {
//             if let Some(language) = opts.next_positional() {
//                 load_text::load_default(language);
//             }
//         }
//         Some("language") | Some("lanugage") | Some("langauge") | Some("lang") | Some("l") => {
//             let config = Config::new();
//             match opts.next_positional() {
//                 Some(language) => {
//                     if let Ok(generator) = LayoutGeneration::new(
//                         language,
//                         "static",
//                         config.trigram_precision(),
//                         Some(config.weights)
//                     ) {
//                         self.language = language.to_string();
//                         self.gen = generator;

//                         println!(
//                             "Set language to {}. Sfr: {:.2}%",
//                             language, self.double_freq() * 100.0
//                         );
//                     } else {
//                         println!("Could not load {}", language);
//                     }
//                 }
//                 None => println!("Current language: {}", self.language)
//             }
//         }
//         Some("languages") | Some("langs") => {
//             for entry in std::fs::read_dir("static/language_data").unwrap() {
//                 if let Ok(p) = entry {
//                     let name = p
//                         .file_name()
//                         .to_string_lossy()
//                         .replace("_", " ")
//                         .replace(".json", "");
//                     if name != "test" {
//                         println!("{}", name);
//                     }
//                 }
//             }
//         }
//         Some("reload") | Some("r") => {
//             let config = Config::new();

//             if let Ok(generator) = LayoutGeneration::new(
//                 self.language.as_str(),
//                 "static",
//                 config.trigram_precision(),
//                 Some(config.weights)
//             ) {
//                 self.gen = generator;
//                 self.pins = config.pins;
//             } else {
//                 println!("Could not load {}", self.language);
//             }
//         }
//         Some("save") | Some("s") => {
//             if let Some(n_str) = opts.next_positional()
//             && let Ok(nr) = usize::from_str_radix(n_str, 10) {
//                 if let Some(layout) = self.get_nth(nr) {
//                     let name = opts.next_positional().map(str::to_string);
//                     self.save(layout, name).unwrap();
//                 }
//             } else {
//                 print_error("save", &[R("index"), O("name")])
//             }
//         }
//         Some("quit") | Some("exit") | Some("q") => {
//             println!("Exiting analyzer...");
//             return Ok(true)
//         }
//         Some("help") | Some("--help") | Some("h") | Some("-h") => {
//             match opts.next_positional() {
//                 Some("generate") | Some("gen") | Some("g") => {
//                     print_help(
//                         "generate",
//                         "(g, gen) Generate a number of layouts and shows the best 10, All layouts generated are accessible until reloading or quiting.",
//                         &[R("amount")]
//                     )
//                 }
//                 Some("improve") | Some("i") => {
//                     print_help(
//                         "improve",
//                         "(i) Save the top <number> result that was generated.",
//                         &[R("name"), R("amount")]
//                     )
//                 }
//                 Some("rank") => {
//                     print_help(
//                         "rank",
//                         "(sort) Rank all layouts in set language by score using values set from 'config.toml'",
//                         &[]
//                     )
//                 }
//                 Some("analyze") | Some("layout") | Some("a") => {
//                     print_help(
//                         "analyze",
//                         "(a, layout) Show details of layout.",
//                         &[R("name or number")]
//                     )
//                 }
//                 Some("compare") | Some("c") | Some("cmp") | Some("cmopare") | Some("comprae") => {
//                     print_help(
//                         "compare",
//                         "(c, cmp) Compare 2 layouts.",
//                         &[R("layout 1"), R("layout 2")]
//                     )
//                 }
//                 Some("ngram") | Some("occ") | Some("n") => {
//                     print_help(
//                         "ngram",
//                         "(n, occ) Gives information about a certain ngram. for 2 letter ones, skipgram info will be provided as well.",
//                         &[R("ngram")]
//                     )
//                 }
//                 Some("load") => {
//                     print_help(
//                         "load",
//                         "Generates corpus for <language>. Will be include everything but spaces if the language is not known.",
//                         &[R("language")]
//                     )
//                 }
//                 Some("language") | Some("lanugage") | Some("langauge") | Some("lang") | Some("l") => {
//                     print_help(
//                         "language",
//                         "(l, lang) Sets a language to be used for analysis.",
//                         &[R("language")]
//                     )
//                 }
//                 Some("languages") | Some("langs") => {
//                     print_help(
//                         "languages",
//                         "(langs) Shows available languages.",
//                         &[]
//                     )
//                 }
//                 Some("reload") | Some("r") => {
//                     print_help(
//                         "reload",
//                         "(r) Reloads all data with the current language. Loses temporary layouts.",
//                         &[]
//                     )
//                 }
//                 Some("save") | Some("s") => {
//                     print_help(
//                         "save",
//                         "(s) Saves the top <number> result that was generated. Starts from 0 up to the number generated.",
//                         &[R("index"), O("name")]
//                     )
//                 }
//                 Some("quit") | Some("exit") | Some("q") => {
//                     print_help(
//                         "quit",
//                         "(q) Quit the repl",
//                         &[]
//                     )
//                 }
//                 Some("help") | Some("--help") | Some("h") | Some("-h") => {
//                     print_help(
//                         "help",
//                         "Print this message or the help of the given subcommand(s)",
//                         &[O("subcommand")]
//                     )
//                 }
//                 Some(c) => println!("error: the subcommand '{c}' wasn't recognized"),
//                 None => {
//                     println!(concat!(
//                         "commands:",
//                         "    analyze      (a, layout) Show details of layout\n",
//                         "    compare      (c, comp) Compare 2 layouts\n",
//                         "    generate     (g, gen) Generate a number of layouts and shows the best 10, All layouts\n",
//                         "                     generated are accessible until reloading or quiting.\n",
//                         "    help         Print this message or the help of the given subcommand(s)\n",
//                         "    improve      (i, optimize) Save the top <NR> result that was generated. Starts from 1, Takes\n",
//                         "                     negative values\n",
//                         "    language     (l, lang) Set a language to be used for analysis. Loads corpus when not present\n",
//                         "    languages    (langs) Show available languages\n",
//                         "    load         Generates corpus for <language>. Will be exclude spaces from source if the\n",
//                         "                     language isn't known\n",
//                         "    ngram        (occ) Gives information about a certain ngram. for 2 letter ones, skipgram info\n",
//                         "                     will be provided as well.\n",
//                         "    quit         (q) Quit the repl\n",
//                         "    rank         (sort) Rank all layouts in set language by score using values set from\n",
//                         "                     'config.toml'\n",
//                         "    reload       (r) Reloads all data with the current language. Loses temporary layouts.\n",
//                         "    save         (s) Save the top <NR> result that was generated. Starts from 1 up to the number\n",
//                         "                     generated, Takes negative values\n"
//                     ));
//                 }
//             }
//         }
//         Some(c) => println!("error: the command '{c}' wasn't recognized"),
//         None => {}
//     }

//     Ok(false)
// }
