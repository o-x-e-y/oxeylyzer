pub mod char_mapping;
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

pub const REPLACEMENT_CHAR: char = char::REPLACEMENT_CHARACTER;
pub const SPACE_CHAR: char = '␣';
pub const SHIFT_CHAR: char = '⇑';
pub const REPEAT_KEY: char = '@';
