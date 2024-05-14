#![allow(dead_code)]

use std::hint::black_box;

use diol::prelude::*;
use oxeylyzer_core::{generate::*, utility::POSSIBLE_SWAPS};

fn main() -> std::io::Result<()> {
    let mut gen = LayoutGeneration::new("english", "./static/", None).unwrap();
    let saved = gen.load_layouts("./static/layouts", "english").unwrap();

    let layout_names = saved
        .keys()
        .take(10)
        .cloned()
        .collect::<Vec<_>>();

    let mut bench = Bench::new(BenchConfig::from_args()?);

    bench.register(score, layout_names.clone());
    bench.register(generate, [0]);
    bench.register(best_swap_cached, layout_names);
    
    bench.run()?;
    Ok(())
}

fn best_swap_cached(bencher: Bencher, name: String) {
    let mut gen = black_box(LayoutGeneration::new("english", "./static/", None).unwrap());
    let saved = gen.load_layouts("./static/layouts", "english").unwrap();
    let mut layout = saved.get(&name).cloned().unwrap();

    let cache = black_box(gen.initialize_cache(&layout));

    bencher.bench(|| {
        black_box(gen.best_swap_cached(&mut layout, &cache, None, &POSSIBLE_SWAPS));
    })
}

fn generate(bencher: Bencher, _: usize) {
    let gen = black_box(LayoutGeneration::new("english", "./static/", None).unwrap());

    bencher.bench(|| {
        gen.generate();
    })
}

fn score(bencher: Bencher, name: String) {
    let mut gen = LayoutGeneration::new("english", "./static/", None).unwrap();
    let saved = gen.load_layouts("./static/layouts", "english").unwrap();

    let layout = black_box(saved.get(&name).unwrap());

    bencher.bench(|| {
        gen.score(layout);
    })
}
