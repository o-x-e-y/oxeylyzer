#![feature(exclusive_range_pattern)]

mod repl;
use oxeylyzer::load_text;

fn main() -> Result<(), String> {
	repl::Repl::run()
}

// fn pause() -> Result<(), std::io::Error> {
//     println!("\nPress any key to continue...");
//     let mut _s = String::new();
//     std::io::stdin().read_line(&mut _s)?;
// 	Ok(())
// }

// fn main() -> Result<(), std::io::Error> {
// 	load_text::load_default("toki_pona");
// 	// load_text::load_all_default().expect("sussy impostor lol impostor");

// 	pause()
// }













