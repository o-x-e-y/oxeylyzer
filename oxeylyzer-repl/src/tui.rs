use oxeylyzer_core::layout::*;
use oxeylyzer_core::rayon::iter::ParallelIterator;
use oxeylyzer_core::{analyzer_data::AnalyzerData, generate::LayoutGeneration};

use ansi_rgb::{Colorable, rgb};
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};

pub fn heatmap_heat(data: &AnalyzerData, u: u8) -> String {
    let complement = 225.0 - (data.get_char_u(u) as f64 / data.char_total as f64) * 1720.0;
    let complement = complement.max(0.0) as u8;
    let heat = rgb(225, complement, complement);
    let c = data.mapping.get_c(u);
    format!("{}", c.to_string().fg(heat))
}

pub fn heatmap_string(data: &AnalyzerData, layout: &FastLayout) -> String {
    let mut res = String::new();

    let mut iter = layout.matrix.iter();

    for &l in layout.shape.inner().iter() {
        let mut i = 0;
        for u in iter.by_ref() {
            res.push_str(heatmap_heat(data, *u).as_str());
            res.push(' ');

            i += 1;

            if l == i {
                break;
            } else if i == 5 {
                res.push(' ');
            }
        }
        res.push('\n');
    }

    res
}

pub fn generate_n_with_pins(
    layout_gen: &LayoutGeneration,
    amount: usize,
    based_on: FastLayout,
    pins: &[usize],
) -> Vec<FastLayout> {
    if amount == 0 {
        return Vec::new();
    }

    let fmt_score = |base| (base as f64) / (layout_gen.data.char_total as f64) / 100.0;

    let start = std::time::Instant::now();

    let pb = ProgressBar::new(amount as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("[{elapsed_precise}] [{wide_bar:.white/white}] [eta: {eta:>3}] - {per_sec:>11} {pos:>6}/{len}")
        .expect("Couldn't initialize the progress bar template")
        .progress_chars("=>-"));

    let mut layouts = layout_gen
        .generate_n_with_pins_iter(amount, based_on.clone(), pins)
        .progress_with(pb)
        .collect::<Vec<_>>();

    println!(
        "Optimizing {} variants took: {} seconds",
        amount,
        start.elapsed().as_secs()
    );

    layouts.sort_by(|l1, l2| l2.score.partial_cmp(&l1.score).unwrap());

    for (i, layout) in layouts.iter().enumerate().take(10) {
        let printable = heatmap_string(&layout_gen.data, layout);
        println!(
            "#{}, score: {:.5}\n{}",
            i,
            fmt_score(layout.score),
            printable
        );
    }

    layouts
}

pub fn generate_n(layout_gen: &LayoutGeneration, amount: usize) -> Vec<FastLayout> {
    if amount == 0 {
        return Vec::new();
    }

    let fmt_score = |base| (base as f64) / (layout_gen.data.char_total as f64) / 100.0;

    let start = std::time::Instant::now();

    let pb = ProgressBar::new(amount as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("[{elapsed_precise}] [{wide_bar:.white/white}] [eta: {eta:>3}] - {per_sec:>11} {pos:>6}/{len}")
        .expect("couldn't initialize the progress bar template")
        .progress_chars("=>-"));

    let mut layouts = layout_gen
        .generate_n_iter(amount)
        .progress_with(pb)
        .collect::<Vec<_>>();

    println!(
        "optimizing {} variants took: {} seconds",
        amount,
        start.elapsed().as_secs()
    );

    layouts.sort_by(|l1, l2| l2.score.partial_cmp(&l1.score).unwrap());

    for (i, layout) in layouts.iter().enumerate().take(10) {
        let printable = heatmap_string(&layout_gen.data, layout);
        println!(
            "#{}, score: {:.5}\n{}",
            i,
            fmt_score(layout.score),
            printable
        );
    }

    layouts
}
