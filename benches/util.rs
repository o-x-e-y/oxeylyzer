#[cfg(not(target_arch = "wasm32"))]
use oxeylyzer_core::generate::Oxeylyzer;

#[cfg(not(target_arch = "wasm32"))]
pub fn oxeylyzer(corpus: &str) -> Oxeylyzer {
    use oxeylyzer_core::{data::Data, weights::Config};

    let config = Config::with_loaded_weights("config.toml").expect("Failed to load config");
    let data =
        Data::load(format!("./static/language_data/{corpus}.json")).expect("this should exist");

    Oxeylyzer::new(data, config)
}
