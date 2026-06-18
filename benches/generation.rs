#![allow(dead_code)]

mod data;
mod languages;
mod util;

use std::hint::black_box;

use diol::prelude::*;
use oxeylyzer_core::{corpus_cleaner::CorpusCleaner, data::Data, layout::PosPair};

use crate::util::oxeylyzer;

fn main() -> diol::Result<()> {
    let dirs =
        oxeylyzer_resources::OxeylyzerDirs::resolve().expect("Failed to resolve oxeylyzer dirs");
    let g = oxeylyzer("english");
    let saved = oxeylyzer_repl::repl::load_layouts(dirs.layouts_dir().join("english")).unwrap();

    let layout_names = saved.keys().take(5).cloned().collect::<Vec<_>>();
    let swaps = g
        .fast_layout(saved.values().next().unwrap(), &[])
        .possible_swaps
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(i, swap)| ((i + 17) % 50 == 0).then_some(swap))
        .collect::<Vec<_>>();

    let languages = ["english", "bokmal"];
    let corpora = ["bokmal", "finnish", "shai"];

    let bench = Bench::from_args()?;

    bench.register("score swap", score_swap, swaps);
    bench.register("score layout", score_layout, layout_names.clone());
    bench.register("generate", generate, languages);
    bench.register("best swap cached", best_swap_cached, layout_names.clone());
    bench.register("best swap", best_swap, layout_names);
    bench.register("create language data", create_data, corpora);
    bench.register("shuffle pins", shuffle_pins, (0..40).step_by(5));
    bench.register("load data", load_data, corpora);

    bench.run()?;

    Ok(())
}

fn score_swap(bencher: Bencher, swap: PosPair) {
    let dirs =
        oxeylyzer_resources::OxeylyzerDirs::resolve().expect("Failed to resolve oxeylyzer dirs");
    let g = oxeylyzer("english");
    let saved = oxeylyzer_repl::repl::load_layouts(dirs.layouts_dir().join("english")).unwrap();

    let (_name, mut layout) = saved
        .into_iter()
        .next()
        .map(|(name, l)| (name, g.fast_layout(&l, &[])))
        .unwrap();

    let cache = g.initialize_cache(&layout);

    bencher.bench(|| g.score_swap_cached(&mut layout, &swap, &cache))
}

fn score_layout(bencher: Bencher, name: String) {
    let dirs =
        oxeylyzer_resources::OxeylyzerDirs::resolve().expect("Failed to resolve oxeylyzer dirs");
    let g = oxeylyzer("english");
    let saved = oxeylyzer_repl::repl::load_layouts(dirs.layouts_dir().join("english")).unwrap();

    let layout = black_box(g.fast_layout(saved.get(&name).unwrap(), &[]));

    bencher.bench(|| {
        g.score(&layout);
    })
}

fn best_swap(bencher: Bencher, name: String) {
    let dirs =
        oxeylyzer_resources::OxeylyzerDirs::resolve().expect("Failed to resolve oxeylyzer dirs");
    let g = black_box(oxeylyzer("english"));
    let saved = oxeylyzer_repl::repl::load_layouts(dirs.layouts_dir().join("english")).unwrap();
    let mut layout = black_box(g.fast_layout(saved.get(&name).unwrap(), &[]));
    let possible_swaps = std::mem::take(&mut layout.possible_swaps);

    bencher.bench(|| {
        black_box(g.best_swap(&mut layout, None, &possible_swaps));
    })
}

fn best_swap_cached(bencher: Bencher, name: String) {
    let dirs =
        oxeylyzer_resources::OxeylyzerDirs::resolve().expect("Failed to resolve oxeylyzer dirs");
    let g = black_box(oxeylyzer("english"));
    let saved = oxeylyzer_repl::repl::load_layouts(dirs.layouts_dir().join("english")).unwrap();
    let mut layout = black_box(g.fast_layout(saved.get(&name).unwrap(), &[]));

    let cache = black_box(g.initialize_cache(&layout));
    let possible_swaps = layout.possible_swaps.clone();

    bencher.bench(|| {
        black_box(g.best_swap_cached(&mut layout, &cache, &possible_swaps, None));
    })
}

fn generate(bencher: Bencher, language: &str) {
    let dirs =
        oxeylyzer_resources::OxeylyzerDirs::resolve().expect("Failed to resolve oxeylyzer dirs");
    let g = black_box(oxeylyzer(language));
    let saved = oxeylyzer_repl::repl::load_layouts(dirs.layouts_dir().join("english")).unwrap();
    let basis = black_box(g.fast_layout(saved.get("sturdy").unwrap(), &[]));

    bencher.bench(|| {
        g.generate(&basis);
    })
}

fn create_data(bencher: Bencher, language: &str) {
    let dirs =
        oxeylyzer_resources::OxeylyzerDirs::resolve().expect("Failed to resolve oxeylyzer dirs");
    let cleaner = CorpusCleaner::raw();
    let text_path = dirs.text_dir().join(language);

    bencher.bench(|| {
        Data::from_paths(&[&text_path], language, &cleaner).expect("couldn't create data:");
    })
}

fn shuffle_pins(bencher: Bencher, pin_count: usize) {
    let step = 40f64 / pin_count as f64;
    let pins = black_box(
        (0..pin_count)
            .map(|v| (v as f64 * step) as usize)
            .collect::<Vec<_>>(),
    );

    let mut arr = black_box((0..40i32).collect::<Vec<_>>());

    bencher.bench(|| {
        oxeylyzer_core::utility::shuffle_pins::<i32>(&mut arr, &pins);
    })
}

fn load_data(bencher: Bencher, language: &str) {
    let dirs =
        oxeylyzer_resources::OxeylyzerDirs::resolve().expect("Failed to resolve oxeylyzer dirs");
    let path = dirs.language_data_dir().join(format!("{language}.json"));

    bencher.bench(|| Data::load(&path).unwrap().name)
}
