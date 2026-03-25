#![warn(missing_docs)]

pub mod analyzer_data;
pub mod char_mapping;
pub mod corpus_cleaner;
pub mod data;
pub mod fast_layout;
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
    #[error("Cannot load data for '{}' as it does not exist.", .0.display())]
    PathDoesNotExist(PathBuf),
    #[error("Specifying a name for the corpus is required")]
    MissingDataName,
    #[error("Failed to serialize data for language '{0}'")]
    CouldNotSerializeData(String),
    #[error("Corpus path '{}' is invalid as it does not end in a (.json) file.", .0.display())]
    InvalidCorpusPath(PathBuf),

    #[error("{0:#}")]
    AnyhowError(#[from] anyhow::Error),

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
