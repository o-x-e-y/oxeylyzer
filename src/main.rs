#![feature(iter_advance_by)]
#![feature(const_eval_limit)]
#![const_eval_limit = "100000000"]
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

fn main() -> Result<(), String> {
	repl::Repl::run()
}













