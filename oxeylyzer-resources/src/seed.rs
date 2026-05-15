use oxeylyzer_core::weights::Config;

use crate::{
    OxeylyzerDirs, ResourceError,
    download::{DownloadProgress, download_and_extract},
};

pub fn ensure_data<F>(dirs: &OxeylyzerDirs, progress: F) -> Result<(), ResourceError>
where
    F: Fn(DownloadProgress) + Send + 'static,
{
    // Download data files into $XDG_DATA_HOME/oxeylyzer if not already present.
    if !dirs.data_dir().join(".data-version").exists() {
        download_and_extract(dirs.data_dir(), progress)?;
    }

    ensure_config(dirs)
}

/// Writes a default `config.toml` to the config root if one does not exist.
///
/// Idempotent — safe to call on every startup.
pub fn ensure_config(dirs: &OxeylyzerDirs) -> Result<(), ResourceError> {
    let config_file = dirs.config_file();
    if !config_file.exists() {
        if let Some(parent) = config_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&config_file, default_config_toml(dirs)?)?;
    }
    Ok(())
}

/// Builds a default `config.toml` string from `Config::default()` with paths
/// pointing into the data root for this installation.
fn default_config_toml(dirs: &OxeylyzerDirs) -> Result<String, ResourceError> {
    let mut config = Config::default();
    config.corpus = dirs.language_data_dir().join("english.json");
    config.layouts = vec![dirs.layouts_dir().join("english").join("*.dof")];
    config.corpus_configs = dirs.corpus_configs_dir().join("**").join("*.toml");

    toml::to_string_pretty(&config).map_err(|e| ResourceError::Config(e.to_string()))
}
