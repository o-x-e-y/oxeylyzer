#![feature(let_chains)]

pub mod language_data;
pub mod load_text;
pub mod trigram_patterns;
pub mod analysis;
pub mod analyze;
pub mod generate;
pub mod translation;
pub mod repl;
pub mod generate_cached;

fn main() -> Result<(), String> {
	repl::Repl::run()
}

fn pause() -> Result<(), std::io::Error> {
    println!("\nPress any key to continue...");
    let mut _s = String::new();
    std::io::stdin().read_line(&mut _s)?;
	Ok(())
}

// fn main() -> Result<(), std::io::Error> {
// 	let x = GenerateCached::new("english", 1000).unwrap();
	

// 	pause()
// }













