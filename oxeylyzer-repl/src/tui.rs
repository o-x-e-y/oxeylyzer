use std::io::Write;

use oxeylyzer_core::generate::LayoutGeneration;
use oxeylyzer_core::language_data::LanguageData;
use oxeylyzer_core::layout::*;
use oxeylyzer_core::rayon::iter::ParallelIterator;

use ansi_rgb::{rgb, Colorable};
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};

pub fn readline() -> std::io::Result<String> {
    write!(std::io::stdout(), "> ")?;
    std::io::stdout().flush()?;
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf)?;
    Ok(buf)
}

pub fn heatmap_heat(data: &LanguageData, c: u8) -> String {
    let complement = 215.0 - *data.characters.get(c as usize).unwrap_or(&0.0) * 1720.0;
    let complement = complement.max(0.0) as u8;
    let heat = rgb(215, complement, complement);
    let c = data.convert_u8.from_single(c);
    format!("{}", c.to_string().fg(heat))
}

pub fn heatmap_string(data: &LanguageData, layout: &FastLayout) -> String {
    let mut print_str = String::new();

    for (i, c) in layout.matrix.iter().enumerate() {
        if i % 10 == 0 && i > 0 {
            print_str.push('\n');
        }
        if (i + 5) % 10 == 0 {
            print_str.push(' ');
        }
        print_str.push_str(heatmap_heat(data, *c).as_str());
        print_str.push(' ');
    }

    print_str
}

pub fn generate_n_with_pins(
    gen: &LayoutGeneration,
    amount: usize,
    based_on: FastLayout,
    pins: &[usize],
) -> Vec<FastLayout> {
    if amount == 0 {
        return Vec::new();
    }

    let start = std::time::Instant::now();

    let pb = ProgressBar::new(amount as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("[{elapsed_precise}] [{wide_bar:.white/white}] [eta: {eta:>3}] - {per_sec:>11} {pos:>6}/{len}")
        .expect("Couldn't initialize the progress bar template")
        .progress_chars("=>-"));

    let mut layouts = gen
        .generate_n_with_pins_iter(amount, based_on, pins)
        .progress_with(pb)
        .collect::<Vec<_>>();

    println!(
        "Optimizing {} variants took: {} seconds",
        amount,
        start.elapsed().as_secs()
    );

    layouts.sort_by(|l1, l2| l2.score.partial_cmp(&l1.score).unwrap());

    for (i, layout) in layouts.iter().enumerate().take(10) {
        let printable = heatmap_string(&gen.data, layout);
        println!("#{}, score: {:.5}\n{}", i, layout.score, printable);
    }

    layouts
}

pub fn generate_n(gen: &LayoutGeneration, amount: usize) -> Vec<FastLayout> {
    if amount == 0 {
        return Vec::new();
    }

    let start = std::time::Instant::now();

    let pb = ProgressBar::new(amount as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("[{elapsed_precise}] [{wide_bar:.white/white}] [eta: {eta:>3}] - {per_sec:>11} {pos:>6}/{len}")
        .expect("couldn't initialize the progress bar template")
        .progress_chars("=>-"));

    let mut layouts = gen
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
        let printable = heatmap_string(&gen.data, layout);
        println!("#{}, score: {:.5}\n{}", i, layout.score, printable);
    }

    layouts
}

pub fn get_ngram_info(data: &mut LanguageData, ngram: &str) -> String {
    match ngram.chars().count() {
        1 => {
            let c = ngram.chars().next().unwrap();
            let u = data.convert_u8.to_single(c);
            let occ = data.characters.get(u as usize).unwrap_or(&0.0) * 100.0;
            format!("{ngram}: {occ:.3}%")
        }
        2 => {
            let bigram: [char; 2] = ngram.chars().collect::<Vec<char>>().try_into().unwrap();
            let c1 = data.convert_u8.to_single(bigram[0]) as usize;
            let c2 = data.convert_u8.to_single(bigram[1]) as usize;

            let b1 = c1 * data.characters.len() + c2;
            let b2 = c2 * data.characters.len() + c1;

            let rev = bigram.into_iter().rev().collect::<String>();

            let occ_b1 = data.bigrams.get(b1).unwrap_or(&0.0) * 100.0;
            let occ_b2 = data.bigrams.get(b2).unwrap_or(&0.0) * 100.0;
            let occ_s = data.skipgrams.get(b1).unwrap_or(&0.0) * 100.0;
            let occ_s2 = data.skipgrams.get(b2).unwrap_or(&0.0) * 100.0;

            format!(
                "{ngram} + {rev}: {:.3}%,\n  {ngram}: {occ_b1:.3}%\n  {rev}: {occ_b2:.3}%\n\
                {ngram} + {rev} (skipgram): {:.3}%,\n  {ngram}: {occ_s:.3}%\n  {rev}: {occ_s2:.3}%",
                occ_b1 + occ_b2,
                occ_s + occ_s2
            )
        }
        3 => {
            let trigram: [char; 3] = ngram.chars().collect::<Vec<char>>().try_into().unwrap();
            let t = [
                data.convert_u8.to_single(trigram[0]),
                data.convert_u8.to_single(trigram[1]),
                data.convert_u8.to_single(trigram[2]),
            ];
            let &(_, occ) = data
                .trigrams
                .iter()
                .find(|&&(tf, _)| tf == t)
                .unwrap_or(&(t, 0.0));
            format!("{ngram}: {:.3}%", occ * 100.0)
        }
        _ => "Invalid ngram! It must be 1, 2 or 3 chars long.".to_string(),
    }
}
