#![feature(const_eval_limit)]
#![feature(iter_advance_by)]
#![feature(iter_collect_into)]
#[const_eval_limit = "100000000"]

#[allow(unused)]
#[allow(dead_code)]

pub mod language_data;
pub mod trigram_patterns;
pub mod analysis;
pub mod analyze;
pub mod generate;
pub mod translation;

use analyze::*;
use crate::generate::{Layout, LayoutGeneration, PerCharStats};
use language_data::text;
use translation::Translator;


fn main() {
	let lang = "dutch";
	// let l = LayoutAnalysis::new(lang);
	// let gen = LayoutGeneration::new(lang);
	// l.rank();
	// l.compare_name("trash", "hrast");
	// l.compare_name("hands_right", "hands_correct");
	// gen.generate_n(100);
	let translator = Translator::new()
		.default_formatting()
		.build();
	text::generate_data(lang, translator);
}













