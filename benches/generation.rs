#![allow(dead_code)]

use std::hint::black_box;

use diol::prelude::*;
use oxeylyzer_core::{
    generate::*,
    utility::{PosPair, POSSIBLE_SWAPS},
};

fn main() -> std::io::Result<()> {
    let mut g = LayoutGeneration::new("english", "./static/", None).unwrap();
    let saved = g.load_layouts("./static/layouts", "english").unwrap();

    let layout_names = saved.keys().take(5).cloned().collect::<Vec<_>>();
    let swaps = POSSIBLE_SWAPS
        .into_iter()
        .enumerate()
        .filter_map(|(i, swap)| ((i + 17) % 50 == 0).then_some(swap))
        .collect::<Vec<_>>();

    let mut bench = Bench::new(BenchConfig::from_args()?);

    bench.register(score_swap, swaps);
    bench.register(score_layout, layout_names.clone());
    bench.register(generate, [0]);
    bench.register(best_swap_cached, layout_names);

    bench.run()?;
    Ok(())
}

fn score_swap(bencher: Bencher, swap: PosPair) {
    let mut g = LayoutGeneration::new("english", "./static/", None).unwrap();
    let saved = g.load_layouts("./static/layouts", "english").unwrap();

    let (_name, mut layout) = saved.into_iter().next().unwrap();
    let cache = g.initialize_cache(&layout);

    bencher.bench(|| g.score_swap_cached(&mut layout, &swap, &cache))
}

fn score_layout(bencher: Bencher, name: String) {
    let mut g = LayoutGeneration::new("english", "./static/", None).unwrap();
    let saved = g.load_layouts("./static/layouts", "english").unwrap();

    let layout = black_box(saved.get(&name).unwrap());

    bencher.bench(|| {
        g.score(layout);
    })
}

fn best_swap_cached(bencher: Bencher, name: String) {
    let mut g = black_box(LayoutGeneration::new("english", "./static/", None).unwrap());
    let saved = g.load_layouts("./static/layouts", "english").unwrap();
    let mut layout = saved.get(&name).cloned().unwrap();

    let cache = black_box(g.initialize_cache(&layout));

    bencher.bench(|| {
        black_box(g.best_swap_cached(&mut layout, &cache, None, &POSSIBLE_SWAPS));
    })
}

fn generate(bencher: Bencher, _: usize) {
    let g = black_box(LayoutGeneration::new("english", "./static/", None).unwrap());

    bencher.bench(|| {
        g.generate();
    })
}
