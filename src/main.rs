#![feature(fs_try_exists)]
#![feature(exclusive_range_pattern)]
#![feature(let_chains)]

mod repl;
mod tui;
mod commands;
mod corpus_transposition;

fn main() -> Result<(), String> {
	repl::Repl::run()
}

// fn main() {
// 	use languages::*;

// 	let x = CorpusConfig::new("english", None);
// }

// use oxeylyzer::load_text;

// fn pause() -> Result<(), std::io::Error> {
//     println!("\nPress any key to continue...");
//     let mut _s = String::new();
//     std::io::stdin().read_line(&mut _s)?;
// 	Ok(())
// }

// fn main() -> Result<(), std::io::Error> {
// 	load_text::load_all_default().expect("sussy impostor lol sus");

// 	pause()
// }













