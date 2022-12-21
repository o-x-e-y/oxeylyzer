#![feature(fs_try_exists)]
#![feature(exclusive_range_pattern)]
#![feature(let_chains)]
#![feature(const_slice_index)]
#![feature(iterator_try_collect)]
#![feature(generic_const_exprs)]

pub mod language_data;
pub mod load_text;
pub mod layout;
pub mod trigram_patterns;
pub mod utility;
pub mod weights;
pub mod generate;
pub mod translation;
pub mod languages_cfg;

pub use rayon;
pub use serde;