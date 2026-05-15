use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use crate::{ResourceError, download::DownloadProgress, seed};

/// Manages platform-appropriate paths for oxeylyzer configuration and data.
///
/// Follows XDG conventions on Linux — configuration lives in `$XDG_CONFIG_HOME`
/// and downloaded/generated data lives in `$XDG_DATA_HOME`:
///
/// | Location            | Linux                              | Windows/macOS          |
/// |---------------------|------------------------------------|------------------------|
/// | `config_root`       | `$XDG_CONFIG_HOME/oxeylyzer`       | `%APPDATA%\oxeylyzer`  |
/// | `data_root`         | `$XDG_DATA_HOME/oxeylyzer`         | same as config_root    |
///
/// **Config root** holds: `config.toml`, `session.json`, `history.txt`, `weight-presets/`
/// **Data root** holds:   `static/layouts/`, `static/language_data/`, `static/corpus_configs/`
///
/// For development set `OXEYLYZER_DATA_DIR=.` (workspace root); both roots collapse to
/// the same directory so the existing `./static/` tree is used directly.
#[derive(Clone)]
pub struct OxeylyzerDirs {
    config_root: PathBuf,
    data_root: PathBuf,
    first_run: bool,
}

impl OxeylyzerDirs {
    /// Resolves platform directories and creates them if needed.
    ///
    /// `is_first_run()` returns `true` when data has never been downloaded
    /// (the `.data-version` sentinel is absent from the data root).
    pub fn resolve() -> Result<Self, ResourceError> {
        let proj = ProjectDirs::from("", "", "oxeylyzer").ok_or(ResourceError::NoDirFound)?;

        let config_root = proj.config_dir().to_path_buf();
        let data_root = proj.data_dir().to_path_buf();

        std::fs::create_dir_all(&config_root)?;
        std::fs::create_dir_all(&data_root)?;

        let first_run = !data_root.join(".data-version").exists();

        Ok(Self {
            config_root,
            data_root,
            first_run,
        })
    }

    /// Creates an `OxeylyzerDirs` pointing at a single override root.
    ///
    /// Both config and data resolve under the same directory — intended for
    /// development (`OXEYLYZER_DATA_DIR=.`) and tests. `is_first_run()` is
    /// always `false` so no download is triggered.
    pub fn with_override(root: PathBuf) -> Self {
        Self {
            config_root: root.clone(),
            data_root: root,
            first_run: false,
        }
    }

    /// `true` if data files have never been downloaded (fresh install).
    pub fn is_first_run(&self) -> bool {
        self.first_run
    }

    // ── Data root (XDG_DATA_HOME) ────────────────────────────────────────────

    /// The data root directory — used to resolve relative paths from `config.toml`.
    pub fn data_dir(&self) -> &Path {
        &self.data_root
    }

    /// `data_root/static/layouts`
    pub fn layouts_dir(&self) -> PathBuf {
        self.data_root.join("static/layouts")
    }

    /// `data_root/static/language_data`
    pub fn language_data_dir(&self) -> PathBuf {
        self.data_root.join("static/language_data")
    }

    /// `data_root/static/corpus_configs`
    pub fn corpus_configs_dir(&self) -> PathBuf {
        self.data_root.join("static/corpus_configs")
    }

    /// `data_root/static/text`
    pub fn text_dir(&self) -> PathBuf {
        self.data_root.join("static/text")
    }

    // ── Config root (XDG_CONFIG_HOME) ────────────────────────────────────────

    /// `config_root/config.toml`
    pub fn config_file(&self) -> PathBuf {
        self.config_root.join("config.toml")
    }

    /// `config_root/session.json`
    pub fn session_file(&self) -> PathBuf {
        self.config_root.join("session.json")
    }

    /// `config_root/history.txt`
    pub fn history_file(&self) -> PathBuf {
        self.config_root.join("history.txt")
    }

    /// `config_root/weight-presets`
    pub fn weight_presets_dir(&self) -> PathBuf {
        self.config_root.join("weight-presets")
    }

    // ── Bootstrap ────────────────────────────────────────────────────────────

    /// Writes a default `config.toml` to the config root if one does not exist.
    ///
    /// Idempotent — safe to call on every startup before loading config.
    pub fn ensure_config(&self) -> Result<(), ResourceError> {
        seed::ensure_config(self)
    }

    /// Downloads and extracts data files if not already present, then ensures
    /// a default `config.toml` exists in the config root.
    ///
    /// Calls `progress` with download/extraction events. Blocks the current
    /// thread — wrap in `spawn_blocking` for async contexts.
    pub fn ensure_data<F>(&self, progress: F) -> Result<(), ResourceError>
    where
        F: Fn(DownloadProgress) + Send + 'static,
    {
        seed::ensure_data(self, progress)
    }
}
