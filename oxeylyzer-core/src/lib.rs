pub mod analyzer_data;
pub mod char_mapping;
pub mod corpus_cleaner;
pub mod data;
pub mod generate;
pub mod language_data;
pub mod languages_cfg;
pub mod layout;
pub mod load_text;
pub mod o2_char_mapping;
pub mod translation;
pub mod trigram_patterns;
pub mod utility;
pub mod weights;

pub use rayon;
pub use serde;

use libdof::DofError;
use thiserror::Error;

pub const REPLACEMENT_CHAR: char = char::REPLACEMENT_CHARACTER;
pub const SPACE_CHAR: char = '␣';
pub const SHIFT_CHAR: char = '⇑';
pub const REPEAT_KEY: char = '@';

// TODO: reassess each error field, maybe add more context
#[derive(Debug, Error)]
pub enum OxeylyzerError {
    #[error("Bigrams should contain 2 characters, bigram with length {0} encountered.")]
    InvalidBigramLength(usize),
    #[error("Trigrams should contain 3 characters, trigram with length {0} encountered.")]
    InvalidTrigramLength(usize),
    #[error("Failed to create a file chunker")]
    ChunkerInitError,
    #[error("Failed to create appropriate chunks")]
    ChunkerChunkError,
    #[error("Path must be either a directory or a file")]
    NotAFile,
    #[error("Specifying a name for the corpus is required")]
    MissingDataName,

    #[error("{0}")]
    IoError(#[from] std::io::Error),
    #[error("{0}")]
    JsonError(#[from] serde_json::Error),
    #[error("{0}")]
    UTF8Error(#[from] std::str::Utf8Error),
    #[error("{0}")]
    DofError(#[from] DofError),
    #[error("{0}")]
    TomlDeserializationError(#[from] toml::de::Error),

    #[cfg(target_arch = "wasm32")]
    #[error("{0}")]
    GlooError(#[from] gloo_net::Error),
}

pub type Result<T> = std::result::Result<T, OxeylyzerError>;
