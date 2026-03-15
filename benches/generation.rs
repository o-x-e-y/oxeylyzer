#![allow(dead_code)]

mod data;
mod languages;

use std::hint::black_box;

use diol::prelude::*;
use oxeylyzer_core::{corpus_cleaner::CorpusCleaner, data::Data, generate::*, layout::PosPair};

fn main() -> diol::Result<()> {
    let g = LayoutGeneration::new("english", "./static/", None).unwrap();
    let saved = oxeylyzer_repl::repl::load_layouts("./static/layouts/english").unwrap();

    let layout_names = saved.keys().take(5).cloned().collect::<Vec<_>>();
    let swaps = g
        .fast_layout(&saved.values().next().unwrap(), &[])
        .possible_swaps
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(i, swap)| ((i + 17) % 50 == 0).then_some(swap))
        .collect::<Vec<_>>();

    let languages = ["english", "bokmal"];
    let corpora = ["bokmal", "hebrew", "shai"];

    let bench = Bench::from_args()?;

    bench.register("score_swap", score_swap, swaps);
    bench.register("score_layout", score_layout, layout_names.clone());
    bench.register("generate", generate, languages);
    bench.register("best_swap_cached", best_swap_cached, layout_names.clone());
    bench.register("best_swap", best_swap, layout_names);
    bench.register("language_data", language_data, corpora);
    bench.register("shuffle_pins", shuffle_pins, (0..40).step_by(5));

    bench.run()?;

    Ok(())
}

fn score_swap(bencher: Bencher, swap: PosPair) {
    let g = LayoutGeneration::new("english", "./static/", None).unwrap();
    let saved = oxeylyzer_repl::repl::load_layouts("./static/layouts/english").unwrap();

    let (_name, mut layout) = saved
        .into_iter()
        .next()
        .map(|(name, l)| (name, g.fast_layout(&l, &[])))
        .unwrap();

    let cache = g.initialize_cache(&layout);

    bencher.bench(|| g.score_swap_cached(&mut layout, &swap, &cache))
}

fn score_layout(bencher: Bencher, name: String) {
    let g = LayoutGeneration::new("english", "./static/", None).unwrap();
    let saved = oxeylyzer_repl::repl::load_layouts("./static/layouts/english").unwrap();

    let layout = black_box(g.fast_layout(&saved.get(&name).unwrap(), &[]));

    bencher.bench(|| {
        g.score(&layout);
    })
}

fn best_swap(bencher: Bencher, name: String) {
    let g = black_box(LayoutGeneration::new("english", "./static/", None).unwrap());
    let saved = oxeylyzer_repl::repl::load_layouts("./static/layouts/english").unwrap();
    let mut layout = black_box(g.fast_layout(saved.get(&name).unwrap(), &[]));
    let possible_swaps = std::mem::take(&mut layout.possible_swaps);

    bencher.bench(|| {
        black_box(g.best_swap(&mut layout, None, &possible_swaps));
    })
}

fn best_swap_cached(bencher: Bencher, name: String) {
    let g = black_box(LayoutGeneration::new("english", "./static/", None).unwrap());
    let saved = oxeylyzer_repl::repl::load_layouts("./static/layouts/english").unwrap();
    let mut layout = black_box(g.fast_layout(saved.get(&name).unwrap(), &[]));

    let cache = black_box(g.initialize_cache(&layout));
    let possible_swaps = layout.possible_swaps.clone();

    bencher.bench(|| {
        black_box(g.best_swap_cached(&mut layout, &cache, &possible_swaps, None));
    })
}

fn generate(bencher: Bencher, language: &str) {
    let g = black_box(LayoutGeneration::new(language, "./static/", None).unwrap());
    let saved = oxeylyzer_repl::repl::load_layouts("./static/layouts/english").unwrap();
    let basis = black_box(g.fast_layout(saved.get("sturdy").unwrap(), &[]));

    bencher.bench(|| {
        g.generate(&basis);
    })
}

fn language_data(bencher: Bencher, language: &str) {
    let cleaner = CorpusCleaner::raw();

    bencher.bench(|| {
        Data::from_paths(&[format!("./static/text/{language}")], language, &cleaner)
            .expect("couldn't create data:");
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
