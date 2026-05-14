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

    // Write default config.toml into $XDG_CONFIG_HOME/oxeylyzer if not present.
    let config_file = dirs.config_file();
    if !config_file.exists() {
        if let Some(parent) = config_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&config_file, default_config_toml(dirs))?;
    }

    Ok(())
}

/// Returns a default `config.toml` whose data paths point into the data root.
///
/// When `config_root != data_root` (XDG mode) the paths are absolute.
/// When using `with_override` (dev mode) they are relative to the workspace root.
pub fn default_config_toml(dirs: &OxeylyzerDirs) -> String {
    let corpus = dirs.language_data_dir().join("english.json");
    let layout_en = dirs.layouts_dir().join("english").join("*.dof");
    let corpus_configs = dirs.corpus_configs_dir().join("**").join("*.toml");

    let corpus_str = corpus.to_string_lossy().replace('\\', "/");
    let layout_en_str = layout_en.to_string_lossy().replace('\\', "/");
    let corpus_configs_str = corpus_configs.to_string_lossy().replace('\\', "/");

    format!(
        r#"corpus = "{corpus_str}"
layouts = [
    "{layout_en_str}",
]
corpus_configs = "{corpus_configs_str}"
trigram_precision = 1000
max_cores = 32

[weights]
lateral_penalty = 1.3
sfbs = -8.0
sfs = -1.0
stretches = -0.3
pinky_ring_bigrams = 0.0
inrolls = 1.6
outrolls = 1.3
onehands = 0.8
alternates = 0.7
alternates_sfs = 0.35
redirects = -1.5
redirects_sfs = -2.75
bad_redirects = -4.0
bad_redirects_sfs = -6.0

[weights.finger_weights]
lp = 1.4
lr = 3.6
lm = 4.8
li = 5.5
lt = 3.3
rt = 3.3
ri = 5.5
rm = 4.8
rr = 3.6
rp = 1.4

[weights.max_finger_use]
penalty = 2.5
pinky = 9.0
ring = 16.0
middle = 19.5
index = 18.0
thumb = 22.0
"#
    )
}
