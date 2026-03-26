//! Core logic for the oxeylyzer keyboard layout analyzer.
//!
//! This crate provides the necessary tools to analyze and generate keyboard layouts,
//! including data structures for bigrams, trigrams, and layout evaluation.

#![warn(missing_docs)]

/// Data structures for analyzing layout performance.
pub mod analyzer_data;
/// Mapping between characters and internal byte representations.
pub mod char_mapping;
/// Tools for cleaning and processing corpus data.
pub mod corpus_cleaner;
/// Basic data structures for corpus information.
pub mod data;
/// Fast layout representation for optimization.
pub mod fast_layout;
/// Layout generation algorithms.
pub mod generate;
/// Layout representation and evaluation.
pub mod layout;
/// Trigram pattern analysis.
pub mod trigram_patterns;
/// Miscellaneous utility functions.
pub mod utility;
/// Weights for different layout metrics.
pub mod weights;

use std::path::{Path, PathBuf};

pub use rayon;
pub use serde;

use thiserror::Error;

/// Character used to replace unknown or invalid characters in a corpus.
pub const REPLACEMENT_CHAR: char = char::REPLACEMENT_CHARACTER;
/// Internal representation of a space character.
pub const SPACE_CHAR: char = '␣';
/// Internal representation of a shift key press.
pub const SHIFT_CHAR: char = '⇑';
/// Internal representation of a repeat key.
pub const REPEAT_KEY: char = '↻';

/// Errors that can occur within the oxeylyzer-core crate.
#[derive(Debug, Error)]
pub enum OxeylyzerError {
    /// Encountered a bigram that does not have a length of 2.
    #[error("Bigrams should contain 2 characters, bigram with length {0} encountered.")]
    InvalidBigramLength(usize),
    /// Encountered a trigram that does not have a length of 3.
    #[error("Trigrams should contain 3 characters, trigram with length {0} encountered.")]
    InvalidTrigramLength(usize),
    /// Failed to initialize the file chunker for corpus processing.
    #[error("Failed to create a file chunker")]
    ChunkerInitError,
    /// Failed to split the corpus into valid chunks.
    #[error("Failed to create appropriate chunks")]
    ChunkerChunkError,
    /// The provided path is not a file or directory.
    #[error("Path must be either a directory or a file, '{}' is neither", .0.display())]
    NotAFile(PathBuf),
    /// The specified path does not exist on the file system.
    #[error("Cannot load data for '{}' as it does not exist.", .0.display())]
    PathDoesNotExist(PathBuf),
    /// No name was provided for the corpus data.
    #[error("Specifying a name for the corpus is required")]
    MissingDataName,
    /// Could not serialize the processed data to JSON.
    #[error("Failed to serialize data for language '{0}'")]
    CouldNotSerializeData(String),
    /// The corpus output path is invalid (usually missing .json extension).
    #[error("Corpus path '{}' is invalid as it does not end in a (.json) file.", .0.display())]
    InvalidCorpusPath(PathBuf),

    /// Wrapper for general anyhow errors.
    #[error("{0:#}")]
    AnyhowError(#[from] anyhow::Error),

    /// Wrapper for errors occurring in a WASM environment.
    #[cfg(target_arch = "wasm32")]
    #[error("{0}")]
    GlooError(#[from] gloo_net::Error),
}

/// Result type used throughout the oxeylyzer-core crate.
pub type Result<T> = std::result::Result<T, OxeylyzerError>;

/// Extension trait to provide context to Result types, converting foreign errors to [`OxeylyzerError`].
pub trait OxeylyzerResultExt<T> {
    /// Adds a path as context to the error.
    fn path_context<P: AsRef<Path>>(self, path: P) -> Result<T>;

    /// Adds a string as context to the error.
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
