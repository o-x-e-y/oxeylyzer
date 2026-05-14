use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use crate::{ResourceError, download::DownloadProgress, seed};

/// Manages platform-appropriate paths for oxeylyzer data and configuration.
///
/// The `root` acts as the project root — analogous to the old `CARGO_MANIFEST_DIR/..`.
/// Static data files (layouts, language data, corpus configs) live under `root/static/`.
/// User configuration (`config.toml`) and session state live directly in `root`.
///
/// Platform roots when using `resolve()`:
/// - Linux:   `$XDG_CONFIG_HOME/oxeylyzer` (default: `~/.config/oxeylyzer`)
/// - Windows: `%APPDATA%\oxeylyzer`
/// - macOS:   `~/Library/Application Support/oxeylyzer`
///
/// For development, set the `OXEYLYZER_DATA_DIR` environment variable to `.`
/// (the workspace root) and the existing `./static/` tree is used directly.
#[derive(Clone)]
pub struct OxeylyzerDirs {
    root: PathBuf,
    first_run: bool,
}

impl OxeylyzerDirs {
    /// Resolves the platform config directory and creates it if needed.
    ///
    /// `is_first_run()` returns `true` if the data version sentinel is absent,
    /// meaning resources have never been downloaded.
    pub fn resolve() -> Result<Self, ResourceError> {
        let root = ProjectDirs::from("", "", "oxeylyzer")
            .ok_or(ResourceError::NoDirFound)?
            .config_dir()
            .to_path_buf();

        std::fs::create_dir_all(&root)?;
        let first_run = !root.join(".data-version").exists();

        Ok(Self { root, first_run })
    }

    /// Creates an `OxeylyzerDirs` pointing at an arbitrary root path.
    ///
    /// `first_run` is always `false` — no download is triggered.
    /// Use this for development (`OXEYLYZER_DATA_DIR=.`) or tests.
    pub fn with_override(root: PathBuf) -> Self {
        Self { root, first_run: false }
    }

    /// Returns `true` if resources have never been downloaded (fresh install).
    pub fn is_first_run(&self) -> bool {
        self.first_run
    }

    /// The project root directory.
    pub fn data_dir(&self) -> &Path {
        &self.root
    }

    /// `root/static/layouts`
    pub fn layouts_dir(&self) -> PathBuf {
        self.root.join("static/layouts")
    }

    /// `root/static/language_data`
    pub fn language_data_dir(&self) -> PathBuf {
        self.root.join("static/language_data")
    }

    /// `root/static/corpus_configs`
    pub fn corpus_configs_dir(&self) -> PathBuf {
        self.root.join("static/corpus_configs")
    }

    /// `root/static/text`
    pub fn text_dir(&self) -> PathBuf {
        self.root.join("static/text")
    }

    /// `root/static/weight-presets`
    pub fn weight_presets_dir(&self) -> PathBuf {
        self.root.join("static/weight-presets")
    }

    /// `root/static/history.txt`
    pub fn history_file(&self) -> PathBuf {
        self.root.join("static/history.txt")
    }

    /// `root/config.toml`
    pub fn config_file(&self) -> PathBuf {
        self.root.join("config.toml")
    }

    /// `root/session.json`
    pub fn session_file(&self) -> PathBuf {
        self.root.join("session.json")
    }

    /// Downloads and extracts resources if not already present, then writes
    /// a default `config.toml` if one does not exist.
    ///
    /// The `progress` callback is called with download/extraction events.
    /// This function blocks the current thread; wrap in `spawn_blocking` for async contexts.
    pub fn ensure_data<F>(&self, progress: F) -> Result<(), ResourceError>
    where
        F: Fn(DownloadProgress) + Send + 'static,
    {
        seed::ensure_data(self, progress)
    }
}
