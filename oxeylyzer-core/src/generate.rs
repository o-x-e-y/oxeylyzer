use std::path::Path;
use std::sync::Arc;

use ahash::AHashMap as HashMap;
use anyhow::{Context, Result};
use indexmap::IndexMap;
use itertools::Itertools;
use libdof::Dof;
use libdof::prelude::Finger;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::REPEAT_KEY;
use crate::analyzer_data::AnalyzerData;
use crate::cached_layout::*;
use crate::char_mapping::CharMapping;
use crate::data::Data;
use crate::trigram_patterns::{TrigramPattern, get_trigram_combinations};
use crate::utility::*;
use crate::weights::{AnalyzerWeights, Config};

pub type CharacterData = Box<[f64]>;
pub type BigramData = Box<[f64]>;
pub type TrigramData = Box<[([u8; 3], i64)]>;

const SMALLEST_SCORE: i64 = i64::MIN;

#[cfg(test)]
static ANALYZED_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[derive(Clone, Default)]
pub struct TrigramAccumulator {
    alternates: i64,
    alternates_sfs: i64,
    inrolls: i64,
    outrolls: i64,
    onehands: i64,
    redirects: i64,
    redirects_sfs: i64,
    bad_redirects: i64,
    bad_redirects_sfs: i64,
    sfbs: i64,
    bad_sfbs: i64,
    sfts: i64,
    thumbs: i64,
    other: i64,
    invalid: i64,
}

impl TrigramAccumulator {
    fn to_stats(&self, trigram_total: i64) -> TrigramStats {
        let div_total = |stat| (stat as f64) / (trigram_total as f64);

        TrigramStats {
            alternates: div_total(self.alternates),
            alternates_sfs: div_total(self.alternates_sfs),
            inrolls: div_total(self.inrolls),
            outrolls: div_total(self.outrolls),
            onehands: div_total(self.onehands),
            redirects: div_total(self.redirects),
            redirects_sfs: div_total(self.redirects_sfs),
            bad_redirects: div_total(self.bad_redirects),
            bad_redirects_sfs: div_total(self.bad_redirects_sfs),
            sfbs: div_total(self.sfbs),
            bad_sfbs: div_total(self.bad_sfbs),
            sfts: div_total(self.sfts),
            thumbs: div_total(self.thumbs),
            other: div_total(self.other),
            invalid: div_total(self.invalid),
        }
    }
}

#[derive(Clone, Default)]
pub struct TrigramStats {
    pub alternates: f64,
    pub alternates_sfs: f64,
    pub inrolls: f64,
    pub outrolls: f64,
    pub onehands: f64,
    pub redirects: f64,
    pub redirects_sfs: f64,
    pub bad_redirects: f64,
    pub bad_redirects_sfs: f64,
    pub sfbs: f64,
    pub bad_sfbs: f64,
    pub sfts: f64,
    pub thumbs: f64,
    pub other: f64,
    pub invalid: f64,
}

impl std::fmt::Display for TrigramStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
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
			Sft: {:.3}%\n",
            self.inrolls * 100.0,
            self.outrolls * 100.0,
            (self.inrolls + self.outrolls) * 100.0,
            self.onehands * 100.0,
            self.alternates * 100.0,
            self.alternates_sfs * 100.0,
            (self.alternates + self.alternates_sfs) * 100.0,
            self.redirects * 100.0,
            self.redirects_sfs * 100.0,
            self.bad_redirects * 100.0,
            self.bad_redirects_sfs * 100.0,
            (self.redirects + self.redirects_sfs + self.bad_redirects + self.bad_redirects_sfs)
                * 100.0,
            self.bad_sfbs * 100.0,
            self.sfts * 100.0
        )
    }
}

pub fn format_fspeed(finger_speed: &[f64]) -> String {
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

#[derive(Clone)]
pub struct LayoutStats {
    pub sfb: f64,
    pub dsfb: f64,
    pub dsfb2: f64,
    pub dsfb3: f64,
    pub scissors: f64,
    pub lsbs: f64,
    pub stretches: f64,
    pub pinky_ring: f64,
    pub trigram_stats: TrigramStats,
    pub fspeed: f64,
    pub finger_speed: [f64; 10],
}

impl std::fmt::Display for LayoutStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            concat!(
                "Sfb:  {:.3}%\nDsfb: {:.3}%\n\nFinger Speed: {:.3}\n",
                "{}\nStretches: {:.3}%\nScissors: {:.3}%\nLsbs: {:.3}%\n",
                "Pinky Ring Bigrams: {:.3}%\n\n{}"
            ),
            self.sfb * 100.0,
            self.dsfb * 100.0,
            self.fspeed * 10.0,
            format_fspeed(&self.finger_speed),
            self.stretches * 10.0,
            self.scissors * 100.0,
            self.lsbs * 100.0,
            self.pinky_ring * 100.0,
            self.trigram_stats
        )
    }
}

#[derive(Default, Debug)]
pub struct LayoutCache {
    // effort: [f64; 30],
    // effort_total: f64,
    scissors: i64,
    lsbs: i64,
    pinky_ring: i64,

    usage: [i64; 10],
    usage_total: i64,

    fspeed: [i64; 10],
    fspeed_total: i64,

    // trigrams: HashMap<(char, Option<char>), f64>,
    stretch_total: i64,
    trigrams_total: i64,

    total_score: i64,
}

impl LayoutCache {
    pub fn total_score(&self) -> i64 {
        self.trigrams_total
            + self.scissors
            + self.lsbs
            + self.pinky_ring
            + self.stretch_total
            + self.usage_total
            + self.fspeed_total
    }
}

type PerCharTrigrams = HashMap<[u8; 2], TrigramData>;

static COLS: [usize; 6] = [0, 1, 2, 7, 8, 9];

pub(crate) fn pinned_swaps(pins: &[usize]) -> Vec<PosPair> {
    let mut map = [true; 30];
    for (i, m) in map.iter_mut().enumerate() {
        if pins.contains(&i) {
            *m = false;
        }
    }
    let mut res = Vec::new();
    for ps in POSSIBLE_SWAPS {
        if map[ps.0] && map[ps.1] {
            res.push(ps);
        }
    }
    res
}

pub struct LayoutGeneration {
    pub language: String,
    pub data: AnalyzerData,
    pub mapping: Arc<CharMapping>,
    pub repeat_key: usize,
    pub chars_for_generation: [u8; 30],
    pub trigram_precision: usize,
    pub trigram_patterns: Box<[TrigramPattern]>,

    scissor_indices: [PosPair; 17],
    lsb_indices: [PosPair; 16],
    pinky_ring_indices: [PosPair; 18],

    per_char_trigrams: PerCharTrigrams,

    pub weights: AnalyzerWeights,
    pub layouts: IndexMap<String, FastLayout>,
}

impl LayoutGeneration {
    pub fn new<P>(language: &str, base_path: P, config: Option<Config>) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let config = config.unwrap_or_else(|| {
            Config::with_loaded_weights(concat!(std::env!("CARGO_MANIFEST_DIR"), "/../config.toml"))
        });
        let data_path = base_path
            .as_ref()
            .join("language_data")
            .join(language)
            .with_extension("json");

        let data = Data::load(&data_path).with_context(|| data_path.display().to_string())?;

        let data = AnalyzerData::new(data, &config.weights);

        let mut chars_for_generation: [u8; 30] = chars_for_generation(language)
            .into_iter()
            .map(|c| data.mapping.get_u(c))
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to convert Vec<u8> to [u8; 30]"))?;

        chars_for_generation.sort_by(|&a, &b| {
            let a = data.get_char_u(a);
            let b = data.get_char_u(b);
            b.cmp(&a)
        });

        Ok(Self {
            language: language.to_string(),
            chars_for_generation, // TODO: remove
            per_char_trigrams: Self::per_char_trigrams(
                data.gen_trigrams(),
                data.len() as u8,
                config.trigram_precision(),
            ),
            mapping: data.mapping.clone(),
            repeat_key: data.mapping.get_u(REPEAT_KEY) as usize, // TODO: remove
            trigram_precision: config.trigram_precision(),
            trigram_patterns: get_trigram_combinations(),
            data,

            scissor_indices: get_scissor_indices(), // TODO: remove for generation
            lsb_indices: get_lsb_indices(),         // TODO: remove for generation
            pinky_ring_indices: get_pinky_ring_indices(),

            weights: config.weights.into(),
            layouts: IndexMap::default(),
        })
    }

    // TODO: move layouts out of LayoutGeneration to split concerns
    pub fn load_layouts<P>(
        &mut self,
        base_directory: P,
        language: &str,
    ) -> Result<IndexMap<String, FastLayout>>
    where
        P: AsRef<Path>,
    {
        let mut res: IndexMap<String, FastLayout> = IndexMap::new();
        let language_dir_path = base_directory.as_ref().join(language);

        if let Ok(read_dir) = std::fs::read_dir(&language_dir_path) {
            let paths = read_dir
                .flatten()
                .filter_map(|d| {
                    let path = d.path();
                    path.is_file().then_some(path)
                })
                .collect::<Vec<_>>();
            let kb_paths = paths.iter().filter(is_kb_file).collect::<Vec<_>>();
            let dof_paths = paths.iter().filter(is_dof_file).collect::<Vec<_>>();

            // let stats_dir = base_directory.as_ref().join("stats").join(language);
            // if let Ok(false) = std::fs::try_exists(stats_dir) {
            // 	std::fs::create_dir_all(&stats_dir)?;
            // }

            for path in kb_paths {
                if let Some(name) = layout_name(path) {
                    let content = std::fs::read_to_string(path)?;
                    let layout_str = format_layout_str(&content);
                    let layout_bytes = self.mapping.map_cs(&layout_str).collect::<Vec<_>>();

                    if let Ok(mut layout) = FastLayout::try_from(layout_bytes.as_slice()) {
                        layout.score = self.score(&layout);
                        res.insert(name, layout);

                    // self.get_layout_stats(&layout);
                    } else {
                        println!("layout {} is not formatted correctly", name);
                    }
                }
            }

            for path in dof_paths {
                let s = std::fs::read_to_string(path)?;
                let dof =
                    serde_json::from_str::<Dof>(&s).with_context(|| path.display().to_string())?;
                let name = dof.name().to_string();

                match FastLayout::from_dof(dof, &self.mapping, &self.weights) {
                    Ok(mut layout) => {
                        layout.score = self.score(&layout);
                        res.insert(name.to_lowercase(), layout);
                    }
                    Err(e) => println!(".dof layout {name} formatted incorrectly: {e}"),
                }
            }

            res.sort_by(|_, a, _, b| a.score.partial_cmp(&b.score).unwrap());
        } else {
            std::fs::create_dir(language_dir_path)?;
        }

        Ok(res)
    }

    pub fn get_layout_stats(&self, layout: &FastLayout) -> LayoutStats {
        let sfb = self.bigram_percent(layout, self.data.bigrams(), self.data.bigram_total);
        let dsfb = self.bigram_percent(layout, self.data.skipgrams(), self.data.skipgram_total);
        let dsfb2 = self.bigram_percent(layout, self.data.skipgrams2(), self.data.skipgram2_total);
        let dsfb3 = self.bigram_percent(layout, self.data.skipgrams3(), self.data.skipgram3_total);

        // TODO: rework conversion to f64
        let cache = self.initialize_cache(layout);
        let fspeed = cache.fspeed_total as f64 / self.data.bigram_total as f64 / 100.0;
        let finger_speed = cache
            .fspeed
            .map(|v| v as f64 / self.data.bigram_total as f64 / 100.0);

        let stretches = self.stretch_score(layout) as f64 / self.data.bigram_total as f64;
        let scissors = (self.scissor_score(layout) as f64 / self.weights.scissors as f64)
            / self.data.bigram_total as f64;
        let lsbs = (self.lsb_score(layout) as f64 / self.weights.lsbs as f64)
            / self.data.bigram_total as f64;
        let pinky_ring = (self.pinky_ring_score(layout) as f64
            / self.weights.pinky_ring_bigrams as f64)
            / self.data.bigram_total as f64;
        let trigram_stats = self
            .trigram_stats(layout, usize::MAX)
            .to_stats(self.data.trigram_total);

        LayoutStats {
            sfb,
            dsfb,
            dsfb2,
            dsfb3,
            fspeed,
            finger_speed,
            stretches,
            scissors,
            lsbs,
            pinky_ring,
            trigram_stats,
        }
    }

    pub fn bigram_percent(&self, layout: &FastLayout, data: &[i64], total: i64) -> f64 {
        let mut res = 0;
        let len = self.data.len();

        for BigramPair { pair, .. } in &layout.fspeed_indices.all {
            let c1 = layout.char(pair.0).unwrap() as usize;
            let c2 = layout.char(pair.1).unwrap() as usize;

            res += data.get(c1 * len + c2).copied().unwrap_or_default();
            res += data.get(c2 * len + c1).copied().unwrap_or_default();
        }

        res as f64 / (total as f64)
    }

    pub fn get_trigram_pattern(
        &self,
        layout: &FastLayout,
        &[t1, t2, t3]: &[u8; 3],
    ) -> TrigramPattern {
        let a = match layout.char_to_finger.get(t1 as usize) {
            Some(&Some(v)) => v as usize,
            _ => return TrigramPattern::Invalid,
        };
        let b = match layout.char_to_finger.get(t2 as usize) {
            Some(&Some(v)) => v as usize,
            _ => return TrigramPattern::Invalid,
        };
        let c = match layout.char_to_finger.get(t3 as usize) {
            Some(&Some(v)) => v as usize,
            _ => return TrigramPattern::Invalid,
        };

        let index = a * 100 + b * 10 + c;
        self.trigram_patterns[index] // TODO: handle out of bounds
    }

    pub fn trigram_stats(
        &self,
        layout: &FastLayout,
        trigram_precision: usize,
    ) -> TrigramAccumulator {
        use TrigramPattern::*;

        let mut freqs = TrigramAccumulator::default();

        for (trigram, freq) in self.data.gen_trigrams().iter().take(trigram_precision) {
            match self.get_trigram_pattern(layout, trigram) {
                Alternate => freqs.alternates += freq,
                AlternateSfs => freqs.alternates_sfs += freq,
                Inroll => freqs.inrolls += freq,
                Outroll => freqs.outrolls += freq,
                Onehand => freqs.onehands += freq,
                Redirect => freqs.redirects += freq,
                RedirectSfs => freqs.redirects_sfs += freq,
                BadRedirect => freqs.bad_redirects += freq,
                BadRedirectSfs => freqs.bad_redirects_sfs += freq,
                Sfb => freqs.sfbs += freq,
                BadSfb => freqs.bad_sfbs += freq,
                Sft => freqs.sfts += freq,
                Thumb => freqs.thumbs += freq,
                Other => freqs.other += freq,
                Invalid => freqs.invalid += freq,
            }
        }
        freqs
    }

    pub fn score(&self, layout: &FastLayout) -> i64 {
        #[cfg(test)]
        ANALYZED_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // let effort = (0..layout.matrix.len())
        //     .map(|i| self.char_effort(layout, i))
        //     .sum::<f64>();

        let fspeed_usage = Finger::FINGERS
            .into_iter()
            .map(|f| self.finger_usage(layout, f) + self.finger_fspeed(layout, f))
            .sum::<i64>();

        let scissors = self.scissor_score(layout);
        let lsbs = self.lsb_score(layout);
        let pinky_ring = self.pinky_ring_score(layout);
        let trigram_score = self.trigram_score_iter(layout, self.data.gen_trigrams());

        trigram_score /* - effort */ + fspeed_usage + scissors + lsbs + pinky_ring
    }

    // fn weighted_bigrams(data: &LanguageData, weights: &Weights) -> BigramData {
    //     let len = data.characters.len();
    //     let chars = 0..len;

    //     chars
    //         .clone()
    //         .cartesian_product(chars)
    //         .map(|(c1, c2)| {
    //             let bigram = c1 * len + c2;
    //             let sfb = data.bigrams.get(bigram).unwrap_or(&0.0);
    //             let dsfb = data.skipgrams.get(bigram).unwrap_or(&0.0) * weights.dsfb_ratio;
    //             let dsfb2 = data.skipgrams2.get(bigram).unwrap_or(&0.0) * weights.dsfb_ratio2;
    //             let dsfb3 = data.skipgrams3.get(bigram).unwrap_or(&0.0) * weights.dsfb_ratio3;
    //             (sfb + dsfb + dsfb2 + dsfb3) * weights.fspeed
    //         })
    //         .collect()
    // }

    // fn stretch_weighted_bigrams(data: &LanguageData, weights: &Weights) -> BigramData {
    //     data.bigrams
    //         .iter()
    //         .zip(&data.skipgrams)
    //         .zip(&data.skipgrams2)
    //         .zip(&data.skipgrams3)
    //         .map(|(((&b, s), s2), s3)| {
    //             let sfb = b;
    //             let sfs = s * weights.dsfb_ratio;
    //             let sfs2 = s2 * weights.dsfb_ratio2;
    //             let sfs3 = s3 * weights.dsfb_ratio3;
    //             (sfb + sfs + sfs2 + sfs3) * weights.stretches
    //         })
    //         .collect::<Box<_>>()
    // }

    fn per_char_trigrams(
        trigrams: &[([u8; 3], i64)],
        highest: u8,
        trigram_precision: usize,
    ) -> HashMap<[u8; 2], TrigramData> {
        let mut n_trigrams = trigrams.to_vec();
        n_trigrams.truncate(trigram_precision);

        (0..highest)
            .cartesian_product(0..highest)
            .map(|(c1, c2)| {
                let v1 = n_trigrams
                    .iter()
                    .map(|(t, f)| (*t, *f))
                    .filter(|(t, _)| t.contains(&c1))
                    .collect::<Vec<_>>();

                let v2 = n_trigrams
                    .iter()
                    .map(|(t, f)| (*t, *f))
                    .filter(|(t, _)| t.contains(&c2))
                    .collect::<Vec<_>>();

                let (big, small, c) = if v1.len() >= v2.len() {
                    (v1, v2, &c1)
                } else {
                    (v2, v1, &c2)
                };

                let per_char = big
                    .into_iter()
                    .chain(small.into_iter().filter(|(t, _)| !t.contains(c)))
                    .collect::<Box<_>>();
                ([c1, c2], per_char)
            })
            .collect()
    }

    #[inline]
    fn trigram_score_iter<'a, T>(&self, layout: &FastLayout, trigrams: T) -> i64
    where
        T: IntoIterator<Item = &'a ([u8; 3], i64)>,
    {
        use TrigramPattern::*;

        let mut freqs = TrigramAccumulator::default();

        for (trigram, freq) in trigrams {
            match self.get_trigram_pattern(layout, trigram) {
                Alternate => freqs.alternates += freq,
                AlternateSfs => freqs.alternates_sfs += freq,
                Inroll => freqs.inrolls += freq,
                Outroll => freqs.outrolls += freq,
                Onehand => freqs.onehands += freq,
                Redirect => freqs.redirects += freq,
                RedirectSfs => freqs.redirects += freq,
                BadRedirect => freqs.bad_redirects += freq,
                BadRedirectSfs => freqs.bad_redirects += freq,
                Sfb => {}
                BadSfb => {}
                Sft => {}
                Thumb => {}
                Other => {}
                Invalid => {}
            }
        }

        let mut score = 0;
        score += self.weights.inrolls * freqs.inrolls;
        score += self.weights.outrolls * freqs.outrolls;
        score += self.weights.onehands * freqs.onehands;
        score += self.weights.alternates * freqs.alternates;
        score += self.weights.alternates_sfs * freqs.alternates_sfs;
        score += self.weights.redirects * freqs.redirects;
        score += self.weights.redirects_sfs * freqs.redirects_sfs;
        score += self.weights.bad_redirects * freqs.bad_redirects;
        score += self.weights.bad_redirects_sfs * freqs.bad_redirects_sfs;
        score
    }

    fn trigram_char_score(&self, layout: &FastLayout, &PosPair(p1, p2): &PosPair) -> i64 {
        if let Some(c1) = layout.char(p1)
            && let Some(c2) = layout.char(p2)
            && let Some(t_vec) = self.per_char_trigrams.get(&[c1, c2])
        {
            self.trigram_score_iter(layout, t_vec)
        } else {
            0
        }
    }

    #[inline]
    fn scissor_score(&self, layout: &FastLayout) -> i64 {
        let mut res = 0;

        for PosPair(p1, p2) in self.scissor_indices {
            if let Some(c1) = layout.char(p1)
                && let Some(c2) = layout.char(p2)
            {
                res += self.data.get_bigram_u([c1, c2]);
                res += self.data.get_bigram_u([c2, c1]);
            }
        }

        res * self.weights.scissors
    }

    #[inline]
    fn lsb_score(&self, layout: &FastLayout) -> i64 {
        let mut res = 0;

        for PosPair(p1, p2) in self.lsb_indices {
            if let Some(c1) = layout.char(p1)
                && let Some(c2) = layout.char(p2)
            {
                res += self.data.get_bigram_u([c1, c2]);
                res += self.data.get_bigram_u([c2, c1]);
            }
        }

        res * self.weights.lsbs
    }

    fn pinky_ring_score(&self, layout: &FastLayout) -> i64 {
        let mut res = 0;

        for PosPair(p1, p2) in self.pinky_ring_indices {
            if let Some(c1) = layout.char(p1)
                && let Some(c2) = layout.char(p2)
            {
                res += self.data.get_bigram_u([c1, c2]);
                res += self.data.get_bigram_u([c2, c1]);
            }
        }

        res * self.weights.pinky_ring_bigrams
    }

    fn stretch_score(&self, layout: &FastLayout) -> i64 {
        layout
            .stretch_indices
            .all_pairs
            .iter()
            .map(
                |BigramPair {
                     dist,
                     pair: PosPair(a, b),
                 }| {
                    let u1 = layout.matrix[*a];
                    let u2 = layout.matrix[*b];

                    // TODO: rework with duplicated bigram logic
                    (self.data.get_stretch_weighted_bigram_u([u1, u2])
                        + self.data.get_stretch_weighted_bigram_u([u2, u1]))
                        * dist
                },
            )
            .sum()
    }

    fn finger_usage(&self, layout: &FastLayout, finger: Finger) -> i64 {
        let mut res = 0;
        match finger {
            Finger::LP | Finger::LR | Finger::LM => {
                let col = finger as usize;
                for c in [
                    layout.char(col).unwrap(),
                    layout.char(col + 10).unwrap(),
                    layout.char(col + 20).unwrap(),
                ] {
                    res += self.data.get_char_u(c);
                }
            }
            Finger::LI | Finger::RI => {
                let col = if finger == Finger::LI {
                    (finger as usize - 3) * 2 + 3
                } else {
                    (finger as usize - 5) * 2 + 3
                };
                // let x = layout.get_index(index)
                for c in [
                    layout.char(col).unwrap(),
                    layout.char(col + 10).unwrap(),
                    layout.char(col + 20).unwrap(),
                    layout.char(col + 1).unwrap(),
                    layout.char(col + 11).unwrap(),
                    layout.char(col + 21).unwrap(),
                ] {
                    res += self.data.get_char_u(c);
                }
            }
            Finger::LT | Finger::RT => { /* TODO: fix for thumbs */ }
            Finger::RM | Finger::RR | Finger::RP => {
                let col = finger as usize;
                for c in [
                    layout.char(col).unwrap(),
                    layout.char(col + 10).unwrap(),
                    layout.char(col + 20).unwrap(),
                ] {
                    res += self.data.get_char_u(c)
                }
            }
        };

        self.weights.max_finger_use.penalty
            * match finger {
                // TODO: fix calculation for int
                Finger::LP | Finger::RP => res - self.weights.max_finger_use.pinky,
                Finger::LR | Finger::RR => res - self.weights.max_finger_use.ring,
                Finger::LM | Finger::RM => res - self.weights.max_finger_use.middle,
                Finger::LI | Finger::RI => res - self.weights.max_finger_use.index,
                Finger::LT | Finger::RT => 0, // TODO: fix for thumb
            }
    }

    pub fn pair_sfb(&self, layout: &FastLayout, bigram_pair: &BigramPair) -> i64 {
        let BigramPair {
            pair: PosPair(p1, p2),
            ..
        } = bigram_pair;

        if let Some(c1) = layout.char(*p1)
            && let Some(c2) = layout.char(*p2)
        {
            self.data.get_bigram_u([c1, c2]) + self.data.get_bigram_u([c2, c1])
        } else {
            0
        }
    }

    #[inline]
    pub fn pair_fspeed(&self, layout: &FastLayout, bigram_pair: &BigramPair) -> i64 {
        let BigramPair {
            pair: PosPair(p1, p2),
            dist,
        } = bigram_pair;

        if let Some(c1) = layout.char(*p1)
            && let Some(c2) = layout.char(*p2)
        {
            // TODO: rework with duplicate bigram logic
            let freq = self.data.get_same_finger_weighted_bigram_u([c1, c2])
                + self.data.get_same_finger_weighted_bigram_u([c2, c1]);

            freq * dist
        } else {
            0
        }
    }

    #[inline]
    fn finger_fspeed(&self, layout: &FastLayout, finger: Finger) -> i64 {
        if let Some(indices) = layout.fspeed_indices.fingers.get(finger as usize) {
            indices
                .iter()
                .map(|pair| self.pair_fspeed(layout, pair))
                .sum()
        } else {
            0
        }
    }

    pub fn stretches_including_pair(&self, layout: &FastLayout, pair: &PosPair) -> i64 {
        layout
            .stretch_indices
            .per_key_pair
            .get(pair)
            .map(|pairs| {
                pairs
                    .iter()
                    .map(|pair| self.pair_stretch(layout, pair))
                    .sum()
            })
            .unwrap_or_default()
    }

    #[inline]
    pub fn pair_stretch(&self, layout: &FastLayout, pair: &BigramPair) -> i64 {
        let BigramPair {
            pair: PosPair(a, b),
            dist,
        } = pair;

        let u1 = layout.matrix[*b];
        let u2 = layout.matrix[*a];

        (self.data.get_stretch_weighted_bigram_u([u1, u2])
            + self.data.get_stretch_weighted_bigram_u([u2, u1]))
            * dist
    }

    pub fn initialize_cache(&self, layout: &FastLayout) -> LayoutCache {
        #[cfg(test)]
        ANALYZED_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let mut res = LayoutCache::default();

        for finger in Finger::FINGERS {
            res.usage[finger as usize] = self.finger_usage(layout, finger);
            res.fspeed[finger as usize] = self.finger_fspeed(layout, finger)
        }
        res.usage_total = res.usage.iter().sum();
        res.fspeed_total = res.fspeed.iter().sum();

        res.scissors = self.scissor_score(layout);

        res.lsbs = self.lsb_score(layout);

        res.pinky_ring = self.pinky_ring_score(layout);

        res.stretch_total = self.stretch_score(layout);

        res.trigrams_total = self.trigram_score_iter(
            layout,
            self.data.gen_trigrams().iter().take(self.trigram_precision),
        );

        res.total_score = res.total_score();

        res
    }

    pub fn score_swap_cached(
        &self,
        layout: &mut FastLayout,
        swap: &PosPair,
        cache: &LayoutCache,
    ) -> Option<i64> {
        #[cfg(test)]
        ANALYZED_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let PosPair(i1, i2) = *swap;

        if i1 == i2
            || layout.char(i1) == layout.char(i2)
            || (self.data.get_char_u(i1 as u8) == 0 && self.data.get_char_u(i2 as u8) == 0)
        {
            return None;
        }

        layout.swap_pair(swap);

        let f1 = layout.matrix_fingers[i1];
        let f2 = layout.matrix_fingers[i2];

        let fspeed_score = if f1 == f2 {
            let fspeed = self.finger_fspeed(layout, f1);

            cache.fspeed_total - cache.fspeed[f1 as usize] + fspeed
        } else {
            let fspeed1 = self.finger_fspeed(layout, f1);
            let fspeed2 = self.finger_fspeed(layout, f2);

            cache.fspeed_total - cache.fspeed[f1 as usize] - cache.fspeed[f2 as usize]
                + fspeed1
                + fspeed2
        };

        let usage_score = if f1 == f2 {
            let usage = self.finger_usage(layout, f1);
            cache.usage_total - cache.usage[f1 as usize] + usage
        } else {
            let usage1 = self.finger_usage(layout, f1);
            let usage2 = self.finger_usage(layout, f2);
            cache.usage_total - cache.usage[f1 as usize] - cache.usage[f2 as usize]
                + usage1
                + usage2
        };

        let scissors_score = if swap.affects_scissor() {
            self.scissor_score(layout)
        } else {
            cache.scissors
        };

        let lsbs_score = if swap.affects_lsb() {
            self.lsb_score(layout)
        } else {
            cache.lsbs
        };

        let pinky_ring_score = if swap.affects_pinky_ring() {
            self.pinky_ring_score(layout)
        } else {
            cache.pinky_ring
        };

        let (stretch_score, trigrams_score) = {
            let stretch_new = self.stretches_including_pair(layout, swap);
            let trigrams_end = self.trigram_char_score(layout, swap);

            layout.swap_pair(swap);

            let stretch_old = self.stretches_including_pair(layout, swap);
            let trigrams_start = self.trigram_char_score(layout, swap);

            let stretch_score = cache.stretch_total - stretch_old + stretch_new;
            let trigrams_score = cache.trigrams_total - trigrams_start + trigrams_end;

            (stretch_score, trigrams_score)
        };

        Some(
            trigrams_score
                + scissors_score
                + lsbs_score
                + pinky_ring_score
                + stretch_score
                + usage_score
                + fspeed_score,
        )
    }

    pub fn accept_swap(&self, layout: &mut FastLayout, swap: &PosPair, cache: &mut LayoutCache) {
        let PosPair(i1, i2) = *swap;

        if i1 == i2
            || layout.char(i1) == layout.char(i2)
            || (self.data.get_char_u(i1 as u8) == 0 && self.data.get_char_u(i2 as u8) == 0)
        {
            return;
        }

        let stretch_start = self.stretches_including_pair(layout, swap);
        let trigrams_start = self.trigram_char_score(layout, swap);

        layout.swap_pair(swap).unwrap();

        let f1 = layout.matrix_fingers[i1];
        let f2 = layout.matrix_fingers[i2];

        cache.fspeed_total = if f1 == f2 {
            let fspeed = self.finger_fspeed(layout, f1);
            let total = cache.fspeed_total - cache.fspeed[f1 as usize] + fspeed;

            cache.fspeed[f1 as usize] = fspeed;

            total
        } else {
            let fspeed1 = self.finger_fspeed(layout, f1);
            let fspeed2 = self.finger_fspeed(layout, f2);
            let total = cache.fspeed_total - cache.fspeed[f1 as usize] - cache.fspeed[f2 as usize]
                + fspeed1
                + fspeed2;

            cache.fspeed[f1 as usize] = fspeed1;
            cache.fspeed[f2 as usize] = fspeed2;

            total
        };

        cache.usage_total = if f1 == f2 {
            let usage = self.finger_usage(layout, f1);
            let total = cache.usage_total - cache.usage[f1 as usize] + usage;

            cache.usage[f1 as usize] = usage;

            total
        } else {
            let usage1 = self.finger_usage(layout, f1);
            let usage2 = self.finger_usage(layout, f2);
            let total = cache.usage_total - cache.usage[f1 as usize] - cache.usage[f2 as usize]
                + usage1
                + usage2;

            cache.usage[f1 as usize] = usage1;
            cache.usage[f2 as usize] = usage2;

            total
        };

        let stretch_end = self.stretches_including_pair(layout, swap);
        let trigrams_end = self.trigram_char_score(layout, swap);

        cache.stretch_total = cache.stretch_total - stretch_start + stretch_end;
        cache.trigrams_total = cache.trigrams_total - trigrams_start + trigrams_end;

        if swap.affects_scissor() {
            cache.scissors = self.scissor_score(layout);
        }

        if swap.affects_lsb() {
            cache.lsbs = self.lsb_score(layout);
        }

        if swap.affects_pinky_ring() {
            cache.pinky_ring = self.pinky_ring_score(layout);
        }

        cache.total_score = cache.total_score();
    }

    pub fn best_swap_cached(
        &self,
        layout: &mut FastLayout,
        cache: &LayoutCache,
        current_best_score: Option<i64>,
        possible_swaps: &[PosPair],
    ) -> (Option<PosPair>, i64) {
        let mut best_score = current_best_score.unwrap_or(SMALLEST_SCORE);
        let mut best_swap: Option<PosPair> = None;

        for swap in possible_swaps {
            if let Some(score) = self.score_swap_cached(layout, swap, cache)
                && score > best_score
            {
                best_score = score;
                best_swap = Some(*swap);
            }
        }

        (best_swap, best_score)
    }

    fn optimize_cached(
        &self,
        layout: &mut FastLayout,
        cache: &mut LayoutCache,
        possible_swaps: &[PosPair],
    ) -> i64 {
        let mut max_swaps = 200; // too high, but makes the system cut off after a while
        let mut current_best_score = SMALLEST_SCORE;

        while let (Some(best_swap), new_score) =
            self.best_swap_cached(layout, cache, Some(current_best_score), possible_swaps)
        {
            current_best_score = new_score;
            self.accept_swap(layout, &best_swap, cache);
            max_swaps -= 1;
            if max_swaps == 0 {
                return current_best_score;
            }
        }
        current_best_score
    }

    fn optimize_cols(&self, layout: &mut FastLayout, cache: &mut LayoutCache, score: Option<i64>) {
        let mut best_score = score.unwrap_or(cache.total_score);

        let mut best = layout.clone();
        self.col_perms(layout, &mut best, cache, &mut best_score, 6);
        layout.swap_indexes();

        self.col_perms(layout, &mut best, cache, &mut best_score, 6);
        *layout = best;
        layout.score = best_score;
    }

    fn col_perms(
        &self,
        layout: &mut FastLayout,
        best: &mut FastLayout,
        cache: &mut LayoutCache,
        best_score: &mut i64,
        k: usize,
    ) {
        if k == 1 {
            let new_score = cache.total_score;
            if new_score > *best_score {
                *best_score = new_score;
                *best = layout.clone();
            }
            return;
        }
        (0..k).for_each(|i| {
            self.col_perms(layout, best, cache, best_score, k - 1);
            if k.is_multiple_of(2) {
                self.accept_swap(layout, &PosPair(COLS[i], COLS[k - 1]), cache);
            } else {
                self.accept_swap(layout, &PosPair(COLS[0], COLS[k - 1]), cache);
            }
        });
    }

    pub fn generate(&self) -> FastLayout {
        let layout = FastLayout::random(&mut self.chars_for_generation.clone());
        let mut cache = self.initialize_cache(&layout);

        let mut layout = self.optimize(layout, &mut cache, &POSSIBLE_SWAPS);
        layout.score = self.score(&layout);
        layout
    }

    pub fn optimize(
        &self,
        mut layout: FastLayout,
        cache: &mut LayoutCache,
        possible_swaps: &[PosPair],
    ) -> FastLayout {
        let mut with_col_score = SMALLEST_SCORE;
        let mut optimized_score = SMALLEST_SCORE + 1;

        while with_col_score < optimized_score {
            optimized_score = self.optimize_cached(&mut layout, cache, possible_swaps);
            self.optimize_cols(&mut layout, cache, Some(optimized_score));
            with_col_score = layout.score;
        }

        layout.score = optimized_score;
        layout
    }

    pub fn generate_n_iter(&self, amount: usize) -> impl ParallelIterator<Item = FastLayout> + '_ {
        (0..amount).into_par_iter().map(|_| self.generate())
    }

    pub fn generate_n_with_pins_iter<'a>(
        &'a self,
        amount: usize,
        based_on: FastLayout,
        pins: &'a [usize],
    ) -> impl ParallelIterator<Item = FastLayout> + 'a {
        let possible_swaps = pinned_swaps(pins);

        (0..amount)
            .into_par_iter()
            .map(move |_| self.generate_with_pins(&based_on, pins, Some(&possible_swaps)))
    }

    pub fn generate_with_pins(
        &self,
        based_on: &FastLayout,
        pins: &[usize],
        possible_swaps: Option<&[PosPair]>,
    ) -> FastLayout {
        let mut layout = FastLayout::random_pins(&mut based_on.matrix.clone(), pins);
        let mut cache = self.initialize_cache(&layout);

        if let Some(ps) = possible_swaps {
            self.optimize_cached(&mut layout, &mut cache, ps)
        } else {
            self.optimize_cached(&mut layout, &mut cache, &pinned_swaps(pins))
        };

        layout.score = self.score(&layout);
        layout
    }
}

mod obsolete;
// mod iterative;

#[cfg(test)]
mod tests {
    use super::*;
    // use crate::utility::ApproxEq;
    use nanorand::Rng;
    use once_cell::sync::Lazy;
    use rayon::iter::ParallelIterator;
    use std::sync::atomic::Ordering;

    static GEN: Lazy<LayoutGeneration> =
        Lazy::new(|| LayoutGeneration::new("english", "static", None).unwrap());

    #[test]
    fn generate() {
        time_this::time!(GEN.generate_n_iter(250).collect::<Vec<_>>());

        println!("{}", ANALYZED_COUNT.load(Ordering::Relaxed));
    }

    #[allow(dead_code)]
    fn fspeed_per_pair() {
        let qwerty_bytes = GEN
            .mapping
            .map_cs("qwertyuiopasdfghjkl;zxcvbnm,./")
            .collect::<Vec<_>>();
        let qwerty = FastLayout::try_from(qwerty_bytes.as_slice()).unwrap();

        for BigramPair { pair, dist } in qwerty.fspeed_indices.all {
            println!(
                "({}, {}) <-> ({}, {}): {dist}",
                pair.0 % 10,
                pair.0 / 10,
                pair.1 % 10,
                pair.1 / 10
            );
        }
    }

    #[test]
    fn cached_totals() {
        let qwerty_bytes = GEN
            .mapping
            .map_cs("qwertyuiopasdfghjkl;zxcvbnm,./")
            .collect::<Vec<_>>();
        let mut qwerty = FastLayout::try_from(qwerty_bytes.as_slice()).unwrap();
        let mut cache = GEN.initialize_cache(&qwerty);
        let mut rng = nanorand::tls_rng();

        for swap in (0..10000).map(|_| &POSSIBLE_SWAPS[rng.generate_range(0..435)]) {
            GEN.accept_swap(&mut qwerty, swap, &mut cache);

            assert_eq!(cache.scissors, GEN.scissor_score(&qwerty));
            assert_eq!(cache.usage_total, GEN.usage_score(&qwerty));
            assert_eq!(cache.fspeed_total, GEN.fspeed_score(&qwerty));
            assert_eq!(
                cache.trigrams_total,
                GEN.trigram_score_iter(
                    &qwerty,
                    GEN.data.gen_trigrams().iter().take(GEN.trigram_precision)
                )
            );
            assert_eq!(cache.lsbs, GEN.lsb_score(&qwerty));
            assert_eq!(cache.pinky_ring, GEN.pinky_ring_score(&qwerty));
            assert_eq!(
                cache.total_score,
                GEN.score_with_precision(&qwerty, GEN.trigram_precision)
            );
        }
    }

    #[test]
    fn best_found_swap() {
        let qwerty_bytes = GEN
            .mapping
            .map_cs("qwertyuiopasdfghjkl;zxcvbnm,./")
            .collect::<Vec<_>>();
        let mut qwerty = FastLayout::try_from(qwerty_bytes.as_slice()).unwrap();
        let cache = GEN.initialize_cache(&qwerty);

        if let (Some(best_swap_normal), best_score_normal) =
            GEN.best_swap(&mut qwerty, None, &POSSIBLE_SWAPS)
        {
            if let (Some(best_swap_cached), best_score_cached) =
                GEN.best_swap_cached(&mut qwerty, &cache, None, &POSSIBLE_SWAPS)
            {
                if best_score_normal == best_score_cached {
                    assert_eq!(best_swap_normal, best_swap_cached);
                } else {
                    println!("scores not the same")
                }
            }
        }
    }

    #[test]
    fn score_swaps_no_accept() {
        let qwerty_bytes = GEN
            .mapping
            .map_cs("qwertyuiopasdfghjkl;zxcvbnm,./")
            .collect::<Vec<_>>();
        let base = FastLayout::try_from(qwerty_bytes.as_slice()).unwrap();
        let mut qwerty = base.clone();
        let cache = GEN.initialize_cache(&qwerty);

        for (i, swap) in POSSIBLE_SWAPS.iter().enumerate() {
            let score_normal = GEN.score_swap(&mut qwerty, swap);
            let maybe_score_cached = GEN.score_swap_cached(&mut qwerty, swap, &cache);

            assert_eq!(base, qwerty);

            if let Some(score_cached) = maybe_score_cached {
                assert!(
                    score_cached == SMALLEST_SCORE + 1000 || score_normal == score_cached,
                    "failed on iteration {i} for {}",
                    POSSIBLE_SWAPS[i]
                );
            }
        }
    }

    #[test]
    fn optimize_qwerty() {
        let qwerty_bytes = GEN
            .mapping
            .map_cs("qwertyuiopasdfghjkl;zxcvbnm,./")
            .collect::<Vec<_>>();
        let qwerty = FastLayout::try_from(qwerty_bytes.as_slice()).unwrap();

        let optimized_normal = GEN.optimize_normal_no_cols(qwerty.clone(), &POSSIBLE_SWAPS);
        let normal_score = GEN.score_with_precision(&optimized_normal, GEN.trigram_precision);

        let mut qwerty_for_cached = FastLayout::try_from(qwerty_bytes.as_slice()).unwrap();
        let mut cache = GEN.initialize_cache(&qwerty_for_cached);

        let best_cached_score =
            GEN.optimize_cached(&mut qwerty_for_cached, &mut cache, &POSSIBLE_SWAPS);

        assert_eq!(normal_score, best_cached_score);
        assert_eq!(
            qwerty_for_cached.layout_str(&GEN.mapping),
            optimized_normal.layout_str(&GEN.mapping)
        );
        // println!("{qwerty_for_cached}");
    }

    #[test]
    fn optimize_random_layouts() {
        for i in 0..5 {
            let layout = FastLayout::random(&mut GEN.chars_for_generation.clone());
            let mut layout_for_cached = layout.clone();

            let optimized_normal = GEN.optimize_normal_no_cols(layout, &POSSIBLE_SWAPS);
            let normal_score = GEN.score_with_precision(&optimized_normal, GEN.trigram_precision);

            let mut cache = GEN.initialize_cache(&layout_for_cached);
            let best_cached_score =
                GEN.optimize_cached(&mut layout_for_cached, &mut cache, &POSSIBLE_SWAPS);

            if normal_score != best_cached_score {
                println!(
                    "{}\n\n{}",
                    optimized_normal.formatted_string(&GEN.data.mapping),
                    layout_for_cached.formatted_string(&GEN.data.mapping)
                );
            }

            assert_eq!(normal_score, best_cached_score, "{i}: i");
            assert_eq!(
                layout_for_cached.layout_str(&GEN.mapping),
                optimized_normal.layout_str(&GEN.mapping),
                "i: {i}"
            );
            assert_eq!(normal_score, best_cached_score, "i: {i}");
        }
    }
}
