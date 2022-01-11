#![feature(const_eval_limit)]
#[const_eval_limit = "100000000"]

#[allow(unused)]

mod language_data;
mod trigram_patterns;
mod analysis;
mod analyze;
mod generate;

use analyze::*;
use crate::generate::LayoutGeneration;

fn main() {
	let lang = "dutch";
	let mut l = LayoutAnalysis::new(lang);
	l.rank();
	l.analyze_name("au_pinky");
	let mut gen = LayoutGeneration::new(lang);
	let dh = l.layout_by_name("colemak_dh").unwrap();
	println!("{}", gen.optimize(dh));
	gen.generate_n(100);
}