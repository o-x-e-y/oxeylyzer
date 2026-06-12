#[cfg(not(target_arch = "wasm32"))]
use oxeylyzer_core::generate::Oxeylyzer;

#[cfg(not(target_arch = "wasm32"))]
pub fn oxeylyzer(corpus: &str) -> Oxeylyzer {
    use oxeylyzer_core::{data::Data, weights::Config};
    use oxeylyzer_resources::OxeylyzerDirs;

    let dirs = OxeylyzerDirs::resolve().expect("Failed to resolve oxeylyzer dirs");
    let config = Config::with_loaded_weights(dirs.config_file()).expect("Failed to load config");
    let data = Data::load(dirs.language_data_dir().join(format!("{corpus}.json")))
        .expect("this should exist");

    Oxeylyzer::new(data, config)
}
