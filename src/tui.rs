use std::io::Write;

use oxeylyzer::generate::LayoutGeneration;
use oxeylyzer::language_data::LanguageData;
use oxeylyzer::rayon::iter::ParallelIterator;
use oxeylyzer::layout::*;

use ansi_rgb::{rgb, Colorable};
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};

pub fn readline() -> Result<String, String> {
    write!(std::io::stdout(), "> ").map_err(|e| e.to_string())?;
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let mut buf = String::new();
    std::io::stdin()
        .read_line(&mut buf)
        .map_err(|e| e.to_string())?;
    Ok(buf)
}

pub fn heatmap_heat(data: &LanguageData, c: &char) -> String {
    let complement = 215.0 - *data.characters.get(c).unwrap_or_else(|| &0.0) * 1720.0;
    let complement = complement.max(0.0) as u8;
    let heat = rgb(215, complement, complement);
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
        print_str.push_str(heatmap_heat(data, c).as_str());
        print_str.push(' ');
    }

    print_str
}

pub fn generate_n_with_pins(
    gen: &LayoutGeneration, amount: usize, based_on: FastLayout, pins: &[usize]
) -> Vec<FastLayout> {
    if amount == 0 {
        return Vec::new();
    }

    let start = std::time::Instant::now();
    
    let pb = ProgressBar::new(amount as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("[{elapsed_precise}] [{wide_bar:.white/white}] [eta: {eta}] - {per_sec:>4} {pos:>6}/{len}")
        .expect("couldn't initialize the progress bar template")
        .progress_chars("=>-"));

    let mut layouts = gen.generate_n_with_pins_iter(amount, based_on, pins)
        .progress_with(pb)
        .collect::<Vec<_>>();

    println!("optmizing {} variants took: {} seconds", amount, start.elapsed().as_secs());

    layouts.sort_by(
        |l1, l2| l2.score.partial_cmp(&l1.score).unwrap()
    );
    
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
        .template("[{elapsed_precise}] [{wide_bar:.white/white}] [eta: {eta}] - {per_sec:>4} {pos:>6}/{len}")
        .expect("couldn't initialize the progress bar template")
        .progress_chars("=>-"));

    let mut layouts = gen.generate_n_iter(amount)
        .progress_with(pb)
        .collect::<Vec<_>>();

    println!("optmizing {} variants took: {} seconds", amount, start.elapsed().as_secs());

    layouts.sort_by(
        |l1, l2| l2.score.partial_cmp(&l1.score).unwrap()
    );
    
    for (i, layout) in layouts.iter().enumerate().take(10) {
        let printable = heatmap_string(&gen.data, layout);
        println!("#{}, score: {:.5}\n{}", i, layout.score, printable);
    }
    
    layouts
}

pub fn get_ngram_info(data: &LanguageData, ngram: &str) -> String {
    match ngram.chars().count() {
        1 => {
            let c = ngram.chars().next().unwrap();
            let occ = data.characters.get(&c).unwrap_or(&0.0) * 100.0;
            format!("{ngram}: {occ:.3}%")
        },
        2 => {
            let b: [char; 2] = ngram.chars().collect::<Vec<char>>().try_into().unwrap();
            let b2 = [b[1], b[0]];
            let rev = String::from_iter(b2);
            let occ_b = data.bigrams.get(&b).unwrap_or(&0.0) * 100.0;
            let occ_b2 = data.bigrams.get(&b2).unwrap_or(&0.0) * 100.0;
            let occ_s = data.skipgrams.get(&b).unwrap_or(&0.0) * 100.0;
            let occ_s2 = data.skipgrams.get(&b2).unwrap_or(&0.0) * 100.0;
            format!(
                "{ngram} + {rev}: {:.3}%,\n  {ngram}: {occ_b:.3}%\n  {rev}: {occ_b2:.3}%\n\
                {ngram} + {rev} (skipgram): {:.3}%,\n  {ngram}: {occ_s:.3}%\n  {rev}: {occ_s2:.3}%",
                occ_b+occ_b2, occ_s+occ_s2
            )
        }
        3 => {
            let t: [char; 3] = ngram.chars().collect::<Vec<char>>().try_into().unwrap();
            let &(_, occ) = data.trigrams
                .iter()
                .find(|&&(tf, _)| tf == t)
                .unwrap_or(&(t, 0.0));
            format!("{ngram}: {:.3}%", occ*100.0)
        }
        _ => "Invalid ngram! It must be 1, 2 or 3 chars long.".to_string()
    }
}