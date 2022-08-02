#![feature(exclusive_range_pattern)]

mod repl;

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
// 	// let x = GenerateCached::new("english", 1000).unwrap();
// 	// load_text::load_default("french");
// 	// load_text::load_default("french_qu");
// 	println!("hello world");

// 	pause()
// }













