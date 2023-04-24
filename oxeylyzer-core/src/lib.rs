#![feature(fs_try_exists)]
#![feature(exclusive_range_pattern)]
#![feature(let_chains)]
#![feature(const_slice_index)]
#![feature(iterator_try_collect)]

pub mod generate;
pub mod language_data;
pub mod languages_cfg;
pub mod layout;
pub mod load_text;
pub mod translation;
pub mod trigram_patterns;
pub mod utility;
pub mod weights;

pub use rayon;
pub use serde;
