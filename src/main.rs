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

fn main() -> Result<()> {
	let lang = "czech";
	let l = LayoutAnalysis::new(lang);
	let gen = LayoutGeneration::new(lang);
	l.rank();
	l.compare_name("czech1", "czech2");
	gen.generate_n(000);
	
	// let translator = Translator::new()
	// 	.language("czech")?
	// 	.build();
	// load_data("test", translator)?;

	pause()
}













