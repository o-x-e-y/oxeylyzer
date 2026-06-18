//! Shared test fixtures for layout search algorithm tests.

use std::path::PathBuf;
use std::sync::OnceLock;

use crate::data::Data;
use crate::fast_layout::FastLayout;
use crate::generate::Oxeylyzer;
use crate::layout::Layout;
use crate::weights::Config;

/// Returns a reference to the singleton analyzer instance used in tests.
pub(crate) fn analyzer() -> &'static Oxeylyzer {
    static GEN: OnceLock<Oxeylyzer> = OnceLock::new();
    GEN.get_or_init(|| {
        let base = PathBuf::from(concat!(std::env!("CARGO_MANIFEST_DIR"), "/.."));
        let config = Config::with_loaded_weights(base.join("config.toml")).unwrap();
        let data = Data::load(base.join(&config.corpus)).unwrap();

        Oxeylyzer::new(data, config)
    })
}

/// Returns a reference to the singleton QWERTY layout used in tests.
pub(crate) fn qwerty() -> &'static FastLayout {
    static QWERTY: OnceLock<FastLayout> = OnceLock::new();
    QWERTY.get_or_init(|| {
        let dof_str = r#"
            {
                "name": "Qwerty",
                "board": "ansi",
                "layers": {
                    "main": [
                        "q w e r t  y u i o p",
                        "a s d f g  h j k l ;",
                        "z x c v b  n m , . /"
                    ]
                },
                "fingering": "traditional"
            }
        "#;

        let layout = serde_json::from_str::<Layout>(dof_str).unwrap();

        analyzer().fast_layout(&layout, &[])
    })
}

/// Scores the given layout using the analyzer's cache-based scoring method.
pub(crate) fn score(layout: &FastLayout) -> i64 {
    analyzer().initialize_cache(layout).total_score()
}

/// Asserts the generated layout is a permutation of the basis (same key multiset).
pub(crate) fn assert_valid(basis: &FastLayout, generated: &FastLayout) {
    let mut expected = basis.keys.to_vec();
    let mut actual = generated.keys.to_vec();
    expected.sort_unstable();
    actual.sort_unstable();

    assert_eq!(
        expected, actual,
        "generated layout is not a permutation of the basis"
    );
}
