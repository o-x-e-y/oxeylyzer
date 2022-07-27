#![feature(let_chains)]
#![feature(exclusive_range_pattern)]

mod language_data;
mod load_text;
mod layout;
mod trigram_patterns;
mod utility;
mod weights;
mod analyze;
mod generate;
mod translation;
mod languages_cfg;

pub use language_data::*;
pub use load_text::*;
pub use layout::*;
pub use trigram_patterns::*;
pub use utility::*;
pub use weights::*;
pub use analyze::*;
pub use generate::*;
pub use translation::*;
pub use languages_cfg::*;
