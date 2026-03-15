pub mod analyzer_data;
pub mod cached_layout;
pub mod char_mapping;
pub mod corpus_cleaner;
pub mod data;
pub mod generate;
pub mod layout;
pub mod trigram_patterns;
pub mod utility;
pub mod weights;

use std::path::{Path, PathBuf};

pub use rayon;
pub use serde;

use thiserror::Error;

pub const REPLACEMENT_CHAR: char = char::REPLACEMENT_CHARACTER;
pub const SPACE_CHAR: char = '␣';
pub const SHIFT_CHAR: char = '⇑';
pub const REPEAT_KEY: char = '↻';

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
    #[error("Path must be either a directory or a file, '{}' is neither", .0.display())]
    NotAFile(PathBuf),
    #[error("Specifying a name for the corpus is required")]
    MissingDataName,
    #[error("Failed to serialize data for language '{0}'")]
    CouldNotSerializeData(String),

    #[error(transparent)]
    AnyhowError(#[from] anyhow::Error),
    // #[error("{0}")]
    // IoError(#[from] std::io::Error),
    // #[error("{0}")]
    // JsonError(#[from] serde_json::Error),
    // #[error("{0}")]
    // UTF8Error(#[from] std::str::Utf8Error),
    // #[error("{0}")]
    // DofError(#[from] DofError),
    // #[error("{0}")]
    // TomlDeserializationError(#[from] toml::de::Error),
    #[cfg(target_arch = "wasm32")]
    #[error("{0}")]
    GlooError(#[from] gloo_net::Error),
}

pub type Result<T> = std::result::Result<T, OxeylyzerError>;

pub trait OxeylyzerResultExt<T> {
    fn path_context<P: AsRef<Path>>(self, path: P) -> Result<T>;

    fn str_context<S: ToString>(self, s: S) -> Result<T>;
}

impl<T, E> OxeylyzerResultExt<T> for std::result::Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn path_context<P: AsRef<Path>>(self, path: P) -> Result<T> {
        use anyhow::Context;

        self.context(path.as_ref().display().to_string())
            .map_err(OxeylyzerError::AnyhowError)
    }

    fn str_context<S: ToString>(self, s: S) -> Result<T> {
        use anyhow::Context;

        self.context(s.to_string())
            .map_err(OxeylyzerError::AnyhowError)
    }
}
