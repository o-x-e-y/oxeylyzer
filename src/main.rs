#![feature(const_eval_limit)]
#![feature(iter_advance_by)]
#![const_eval_limit = "100000000"]

pub mod language_data;
pub mod load_text;
pub mod trigram_patterns;
pub mod analysis;
pub mod analyze;
pub mod generate;
pub mod translation;
pub mod ngrams;
pub mod repl;

use analyze::*;
use anyhow::Result;
use translation::Translator;
use generate::LayoutGeneration;
use load_text::*;


fn pause() -> Result<()> {
	println!("\nPress any key to continue...");
	let mut _s = String::new();
	std::io::stdin().read_line(&mut _s)?;
	Ok(())
}

fn main() -> Result<(), String> {
	// let translator = Translator::new()
	// 	.language("czech")?
	// 	.letters("áíě")
	// 	.build();
	// load_data("czech", translator)?;
	repl::Repl::run()
	
	// let lang = "english";
	// // load_default(lang);

	// let l = LayoutAnalysis::new(lang);
	// let gen = LayoutGeneration::new(lang);
	// l.rank();
	// l.compare_name("wtf", "whorf");
	// l.compare_name("whorf_rsnt", "whorf_nsrt");
	// gen.generate_n(000);

	// pause()
}













