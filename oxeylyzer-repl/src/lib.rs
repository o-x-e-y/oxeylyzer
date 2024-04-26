pub mod corpus_transposition;
pub mod flags;
pub mod repl;
pub mod tui;

#[test]
fn thing() {
    let x = std::path::PathBuf::from("this/is/a/path");

    println!("{}", x.display())
}

// fn main() {
// 	use languages::*;

// 	let x = CorpusConfig::new("english", None);
// }

// use oxeylyzer_core::load_text;

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
