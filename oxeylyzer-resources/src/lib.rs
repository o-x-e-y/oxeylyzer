mod dirs;
mod download;
mod seed;

pub use dirs::OxeylyzerDirs;
pub use download::DownloadProgress;
pub use seed::default_config_toml;

#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    #[error("Could not determine config directory for this platform")]
    NoDirFound,
    #[error("Download failed: {0}")]
    Download(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Archive extraction failed: {0}")]
    Extraction(String),
}
