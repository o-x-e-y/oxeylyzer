#![feature(let_chains)]

pub mod language_data;
pub mod load_text;
pub mod trigram_patterns;
pub mod analysis;
pub mod analyze;
pub mod generate;
pub mod translation;
pub mod ngrams;
pub mod repl;
pub mod generate_cached;

fn main() -> Result<(), String> {
	repl::Repl::run()
}













