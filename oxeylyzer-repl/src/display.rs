use oxeylyzer_core::fast_layout::*;
use oxeylyzer_core::generate::LayoutStats;
use oxeylyzer_core::rayon::iter::ParallelIterator;
use oxeylyzer_core::{analyzer_data::AnalyzerData, generate::LayoutGeneration};

use ansi_rgb::{Colorable, rgb};
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};

pub fn heatmap_heat(c: char, data: &AnalyzerData) -> String {
    let complement = 225.0 - (data.get_char(c) as f64 / data.char_total as f64) * 1720.0;
    let complement = complement.max(0.0) as u8;
    let heat = rgb(225, complement, complement);

    format!("{}", c.to_string().fg(heat))
}

pub fn heatmap_string(layout: &FastLayout, data: &AnalyzerData) -> String {
    layout
        .formatted_string()
        .chars()
        .map(|c| match c {
            ' ' => ' '.to_string(),
            '\n' => '\n'.to_string(),
            c => heatmap_heat(c, data),
        })
        .collect()
}

pub fn generate_n_with_pins(
    layout_gen: &LayoutGeneration,
    amount: usize,
    based_on: FastLayout,
    pins: &[usize],
) -> Vec<FastLayout> {
    if amount == 0 {
        println!("Optimizing 0 variants took: 0 seconds");
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
        .generate_n_with_pins_iter(amount, &based_on, pins)
        .map(|l| (layout_gen.score(&l), l))
        .progress_with(pb)
        .collect::<Vec<_>>();

    println!(
        "Optimizing {} variants took: {} seconds",
        amount,
        start.elapsed().as_secs()
    );

    layouts.sort_by_key(|(score, _)| std::cmp::Reverse(*score));

    for (i, (score, layout)) in layouts.iter().enumerate().take(10) {
        let printable = heatmap_string(layout, &layout_gen.data);
        println!("#{}, score: {:.5}\n{}", i, fmt_score(*score), printable);
    }

    layouts.into_iter().map(|(_, layout)| layout).collect()
}

fn format_fspeed(finger_speed: &[f64]) -> String {
    let f = |v| format!("{:.3}", v * 10.0);

    let mut left_hand = Vec::new();
    for v in finger_speed.iter().take(5) {
        left_hand.push(f(v))
    }

    let mut right_hand = Vec::new();
    for v in finger_speed.iter().rev().take(5) {
        right_hand.push(f(v))
    }

    let legend = "   Pinky   Ring    Middle  Index   Thumb\n";
    let left_hand = format!("L: {}\n", left_hand.join(", "));
    let right_hand = format!("R: {}\n", right_hand.join(", "));

    format!("{legend}{left_hand}{right_hand}")
}

pub fn print_layout_stats(stats: &LayoutStats) {
    println!(
        concat!(
            "Sfb:  {:.3}%\nDsfb: {:.3}%\n\nFinger Speed: {:.3}\n",
            "{}\nStretches: {:.3}%\nScissors: {:.3}%\nLsbs: {:.3}%\n",
            "Pinky Ring Bigrams: {:.3}%\n"
        ),
        stats.sfb,
        stats.dsfb,
        stats.fspeed,
        format_fspeed(&stats.finger_speed),
        stats.stretches,
        stats.scissors,
        stats.lsbs,
        stats.pinky_ring,
    );

    let t = &stats.trigram_stats;

    println!(
        "Inrolls: {:.3}%\n\
			Outrolls: {:.3}%\n\
			Total Rolls: {:.3}%\n\
			Onehands: {:.3}%\n\n\
			Alternates: {:.3}%\n\
			Alternates (sfs): {:.3}%\n\
			Total Alternates: {:.3}%\n\n\
			Redirects: {:.3}%\n\
			Redirects Sfs: {:.3}%\n\
			Bad Redirects: {:.3}%\n\
			Bad Redirects Sfs: {:.3}%\n\
			Total Redirects: {:.3}%\n\n\
			Bad Sfbs: {:.3}%\n\
			Sft: {:.3}%",
        t.inrolls,
        t.outrolls,
        (t.inrolls + t.outrolls),
        t.onehands,
        t.alternates,
        t.alternates_sfs,
        (t.alternates + t.alternates_sfs),
        t.redirects,
        t.redirects_sfs,
        t.bad_redirects,
        t.bad_redirects_sfs,
        (t.redirects + t.redirects_sfs + t.bad_redirects + t.bad_redirects_sfs),
        t.bad_sfbs,
        t.sfts
    )
}

pub fn print_compare_stats(s1: &LayoutStats, s2: &LayoutStats, score1: f64, score2: f64) {
    let ts1 = &s1.trigram_stats;
    let ts2 = &s2.trigram_stats;

    println!(
        concat!(
            "\n",
            "Sfb:                {: <11} Sfb:                {:.3}%\n",
            "Dsfb:               {: <11} Dsfb:               {:.3}%\n",
            "Finger Speed:       {: <11} Finger Speed:       {:.3}\n",
            "Stretches:          {: <11} Stretches:          {:.3}\n",
            "Scissors:           {: <11} Scissors:           {:.3}%\n",
            "Lsbs:               {: <11} Lsbs:               {:.3}%\n",
            "Pinky Ring Bigrams: {: <11} Pinky Ring Bigrams: {:.3}%\n\n",
            "Inrolls:            {: <11} Inrolls:            {:.2}%\n",
            "Outrolls:           {: <11} Outrolls:           {:.2}%\n",
            "Total Rolls:        {: <11} Total Rolls:        {:.2}%\n",
            "Onehands:           {: <11} Onehands:           {:.3}%\n\n",
            "Alternates:         {: <11} Alternates:         {:.2}%\n",
            "Alternates Sfs:     {: <11} Alternates Sfs:     {:.2}%\n",
            "Total Alternates:   {: <11} Total Alternates:   {:.2}%\n\n",
            "Redirects:          {: <11} Redirects:          {:.3}%\n",
            "Redirects Sfs:      {: <11} Redirects Sfs:      {:.3}%\n",
            "Bad Redirects:      {: <11} Bad Redirects:      {:.3}%\n",
            "Bad Redirects Sfs:  {: <11} Bad Redirects Sfs:  {:.3}%\n",
            "Total Redirects:    {: <11} Total Redirects:    {:.3}%\n\n",
            "Bad Sfbs:           {: <11} Bad Sfbs:           {:.3}%\n",
            "Sft:                {: <11} Sft:                {:.3}%\n\n",
            "Score:              {: <11} Score:              {:.3}\n"
        ),
        format!("{:.3}%", s1.sfb * 100.0),
        s2.sfb * 100.0,
        format!("{:.3}%", s1.dsfb * 100.0),
        s2.dsfb * 100.0,
        format!("{:.3}", s1.fspeed * 10.0),
        s2.fspeed * 10.0,
        format!("{:.3}", s1.stretches * 10.0),
        s2.stretches * 10.0,
        format!("{:.3}%", s1.scissors * 100.0),
        s2.scissors * 100.0,
        format!("{:.3}%", s1.lsbs * 100.0),
        s2.lsbs * 100.0,
        format!("{:.3}%", s1.pinky_ring * 100.0),
        s2.pinky_ring * 100.0,
        format!("{:.2}%", ts1.inrolls * 100.0),
        ts2.inrolls * 100.0,
        format!("{:.2}%", ts1.outrolls * 100.0),
        ts2.outrolls * 100.0,
        format!("{:.2}%", (ts1.inrolls + ts1.outrolls) * 100.0),
        (ts2.inrolls + ts2.outrolls) * 100.0,
        format!("{:.3}%", ts1.onehands * 100.0),
        ts2.onehands * 100.0,
        format!("{:.2}%", ts1.alternates * 100.0),
        ts2.alternates * 100.0,
        format!("{:.2}%", ts1.alternates_sfs * 100.0),
        ts2.alternates_sfs * 100.0,
        format!("{:.2}%", (ts1.alternates + ts1.alternates_sfs) * 100.0),
        (ts2.alternates + ts2.alternates_sfs) * 100.0,
        format!("{:.3}%", ts1.redirects * 100.0),
        ts2.redirects * 100.0,
        format!("{:.3}%", ts1.redirects_sfs * 100.0),
        ts2.redirects_sfs * 100.0,
        format!("{:.3}%", ts1.bad_redirects * 100.0),
        ts2.bad_redirects * 100.0,
        format!("{:.3}%", ts1.bad_redirects_sfs * 100.0),
        ts2.bad_redirects_sfs * 100.0,
        format!(
            "{:.3}%",
            (ts1.redirects + ts1.redirects_sfs + ts1.bad_redirects + ts1.bad_redirects_sfs) * 100.0
        ),
        (ts2.redirects + ts2.redirects_sfs + ts2.bad_redirects + ts2.bad_redirects_sfs) * 100.0,
        format!("{:.3}%", ts1.bad_sfbs * 100.0),
        ts2.bad_sfbs * 100.0,
        format!("{:.3}%", ts1.sfts * 100.0),
        ts2.sfts * 100.0,
        format!("{:.3}", score1),
        score2,
    );
}
