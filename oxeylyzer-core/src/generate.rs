use std::hint::unreachable_unchecked;
use std::path::Path;

use anyhow::Result;
// use fxhash::FxHashMap;
use ahash::AHashMap as HashMap;
use indexmap::IndexMap;
use itertools::Itertools;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::language_data::{BigramData, LanguageData, TrigramData};
use crate::layout::*;
use crate::trigram_patterns::TrigramPattern;
use crate::utility::*;
use crate::weights::{Config, Weights};

#[cfg(test)]
static PRUNED_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
#[cfg(test)]
static NOT_PRUNED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

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

impl std::fmt::Debug for TrigramStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Inrolls: {:.3}%\n
			Outrolls: {:.3}%\n
			Total Rolls: {:.3}%\n
			Onehands: {:.3}%\n\n\
			Alternates: {:.3}%\n
			Alternates Sfs: {:.3}%\n
			Total Alternates: {:.3}%\n\n
			Redirects: {:.3}%\n\
			Redirects Sfs: {:.3}%\n\
			Bad Redirects: {:.3}%\n
			Bad Redirects Sfs: {:.3}%\n\
			Total Redirects: {:.3}%\n\n
			Bad Sfbs: {:.3}%\n
			Sft: {:.3}%\n\n
			Other: {:.3}%\n
			Invalid: {:.3}%",
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
            self.sfts * 100.0,
            self.other * 100.0,
            self.invalid * 100.0
        )
    }
}

fn format_fspeed(finger_speed: &[f64]) -> String {
    let mut finger_speed_str: Vec<String> = Vec::new();
    for v in finger_speed {
        finger_speed_str.push(format!("{:.3}", v * 10.0))
    }
    finger_speed_str.join(", ")
}

#[derive(Clone)]
pub struct LayoutStats {
    pub sfb: f64,
    pub dsfb: f64,
    pub dsfb2: f64,
    pub dsfb3: f64,
    pub scissors: f64,
    pub lsbs: f64,
    pub pinky_ring: f64,
    pub trigram_stats: TrigramStats,
    pub fspeed: f64,
    pub finger_speed: [f64; 8],
}

impl std::fmt::Display for LayoutStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            concat!(
                "Sfb:  {:.3}%\nDsfb: {:.3}%\nFinger Speed: {:.3}\n",
                "    [{}]\nScissors: {:.3}%\nLsbs: {:.3}%\nPinky Ring Bigrams: {:.3}%\n\n{}"
            ),
            self.sfb * 100.0,
            self.dsfb * 100.0,
            self.fspeed * 10.0,
            format_fspeed(&self.finger_speed),
            self.scissors * 100.0,
            self.lsbs * 100.0,
            self.pinky_ring * 100.0,
            self.trigram_stats
        )
    }
}

#[derive(Default, Debug)]
pub struct LayoutCache {
    effort: [f64; 30],
    effort_total: f64,

    scissors: f64,
    lsbs: f64,
    pinky_ring: f64,

    usage: [f64; 8],
    usage_total: f64,

    fspeed: [f64; 8],
    fspeed_total: f64,

    // trigrams: HashMap<(char, Option<char>), f64>,
    trigrams_total: f64,

    total_score: f64,
}

impl LayoutCache {
    pub fn total_score(&self) -> f64 {
        self.trigrams_total
            - self.scissors
            - self.lsbs
            - self.pinky_ring
            - self.effort_total
            - self.usage_total
            - self.fspeed_total
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
    pub data: LanguageData,
    pub convert_u8: ConvertU8,
    pub repeat_key: usize,
    pub chars_for_generation: [u8; 30],

    fspeed_vals: [(PosPair, f64); 48],
    effort_map: [f64; 30],
    scissor_indices: [PosPair; 17],
    lsb_indices: [PosPair; 16],
    pinky_ring_indices: [PosPair; 18],

    weighted_bigrams: BigramData,
    per_char_trigrams: PerCharTrigrams,

    pub weights: Weights,
    pub layouts: IndexMap<String, FastLayout>,
}

impl LayoutGeneration {
    pub fn new<P>(language: &str, base_path: P, config: Option<Config>) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let config = config.unwrap_or_else(Config::with_loaded_weights);

        if let Ok(mut data) =
            LanguageData::from_file(base_path.as_ref().join("language_data"), language)
        {
            let chars_fg = data.convert_u8.to(chars_for_generation(language));
            let mut chars_for_generation: [u8; 30] = chars_fg.try_into().unwrap();
            chars_for_generation.sort_by(|&a, &b| {
                let a = data.characters.get(a as usize).unwrap_or(&0.0);
                let b = data.characters.get(b as usize).unwrap_or(&0.0);
                b.partial_cmp(a).unwrap()
            });

            Ok(Self {
                language: language.to_string(),
                chars_for_generation,
                weighted_bigrams: Self::weighted_bigrams(&data, &config.weights),
                per_char_trigrams: Self::per_char_trigrams(
                    &data.trigrams,
                    data.characters.len() as u8,
                    config.defaults.trigram_precision,
                ),
                convert_u8: data.convert_u8.clone(),
                repeat_key: data.convert_u8.to_single('@') as usize,
                data,

                fspeed_vals: get_fspeed(config.weights.lateral_penalty),
                effort_map: get_effort_map(config.weights.heatmap, config.defaults.keyboard_type),
                
                scissor_indices: get_scissor_indices(),
                lsb_indices: get_lsb_indices(),
                pinky_ring_indices: get_pinky_ring_indices(),

                weights: config.weights,
                layouts: IndexMap::default(),
            })
        } else {
            anyhow::bail!("Getting language data failed")
        }
    }

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

        if let Ok(paths) = std::fs::read_dir(&language_dir_path) {
            let valid = paths
                .flatten()
                .filter(is_kb_file)
                .collect::<Vec<_>>();

            // let stats_dir = base_directory.as_ref().join("stats").join(language);
            // if let Ok(false) = std::fs::try_exists(stats_dir) {
            // 	std::fs::create_dir_all(&stats_dir)?;
            // }

            for entry in valid {
                if let Some(name) = layout_name(&entry) {
                    let content = std::fs::read_to_string(entry.path())?;
                    let layout_str = format_layout_str(&content);
                    let layout_bytes = self.convert_u8.to(layout_str.chars());

                    if let Ok(mut layout) = FastLayout::try_from(layout_bytes.as_slice()) {
                        layout.score = self.score(&layout);
                        res.insert(name, layout);

                    // self.get_layout_stats(&layout);
                    } else {
                        println!("layout {} is not formatted correctly", name);
                    }
                }
            }

            res.sort_by(|_, a, _, b| a.score.partial_cmp(&b.score).unwrap());
        } else {
            std::fs::create_dir(language_dir_path)?;
        }

        Ok(res)
    }

    pub fn get_layout_stats(&self, layout: &FastLayout) -> LayoutStats {
        let sfb = self.bigram_percent(layout, "sfbs");
        let dsfb = self.bigram_percent(layout, "skipgrams");
        let dsfb2 = self.bigram_percent(layout, "skipgrams2");
        let dsfb3 = self.bigram_percent(layout, "skipgrams3");
        let cache = self.initialize_cache(layout);
        let fspeed = cache.fspeed_total;
        let finger_speed = cache.fspeed;
        let scissors = self.scissor_score(layout) / self.weights.scissors;
        let lsbs = self.lsb_score(layout) / self.weights.lsbs;
        let pinky_ring = self.pinky_ring_score(layout) / self.weights.pinky_ring_bigrams;
        let trigram_stats = self.trigram_stats(layout, usize::MAX);

        LayoutStats {
            sfb,
            dsfb,
            dsfb2,
            dsfb3,
            fspeed,
            finger_speed,
            scissors,
            lsbs,
            pinky_ring,
            trigram_stats,
        }
    }

    pub fn bigram_percent(&self, layout: &FastLayout, bigram_type: &str) -> f64 {
        let data = match bigram_type {
            "bigram" | "bigrams" | "sfb" | "sfbs" => &self.data.bigrams,
            "skipgram" | "skipgrams" | "dsfb" | "dsfbs" => &self.data.skipgrams,
            "skipgram2" | "skipgrams2" | "dsfb2" | "dsfbs2" => &self.data.skipgrams2,
            "skipgram3" | "skipgrams3" | "dsfb3" | "dsfbs3" => &self.data.skipgrams3,
            _ => panic!("bigram type {bigram_type} does not exist!"),
        };

        let mut res = 0.0;
        let len = self.data.characters.len();

        for (PosPair(i1, i2), _) in self.fspeed_vals {
            let c1 = unsafe { layout.cu(i1) } as usize;
            let c2 = unsafe { layout.cu(i2) } as usize;

            // if c1 != self.repeat_key && c2 != self.repeat_key {
            // 	res += data.get(c1 * len + c2).unwrap_or(&0.0);
            // 	res += data.get(c2 * len + c1).unwrap_or(&0.0);
            // } else {
            // 	res += data.get(c1 * len + c2).unwrap_or(&0.0);
            // 	res += data.get(c2 * len + c1).unwrap_or(&0.0);
            // }

            res += data.get(c1 * len + c2).unwrap_or(&0.0);
            res += data.get(c2 * len + c1).unwrap_or(&0.0);
        }
        res
    }

    pub fn sfbs(&self, layout: &FastLayout, top_n: usize) -> Vec<(String, f64)> {
        self.fspeed_vals
            .iter()
            .map(|(p, _)| {
                let u1 = layout.c(p.0);
                let u2 = layout.c(p.1);

                let bigram = self.convert_u8.as_str(&[u1, u2]);
                let bigram2 = self.convert_u8.as_str(&[u2, u1]);

                let i = (u1 as usize) * self.data.characters.len() + (u2 as usize);
                let i2 = (u2 as usize) * self.data.characters.len() + (u1 as usize);

                let freq = self.data.bigrams[i];
                let freq2 = self.data.bigrams[i2];

                [(bigram, freq), (bigram2, freq2)]
            })
            .flatten()
            .sorted_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap())
            .take(top_n)
            .collect::<Vec<_>>()
    }

    pub fn trigram_stats(&self, layout: &FastLayout, trigram_precision: usize) -> TrigramStats {
        use TrigramPattern::*;

        let mut freqs = TrigramStats::default();

        for (trigram, freq) in self.data.trigrams.iter().take(trigram_precision) {
            match layout.get_trigram_pattern(trigram) {
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
                Other => freqs.other += freq,
                Invalid => freqs.invalid += freq,
            }
        }
        freqs
    }

    pub fn score(&self, layout: &FastLayout) -> f64 {
        let effort = (0..layout.matrix.len())
            .map(|i| self.char_effort(layout, i))
            .sum::<f64>();

        let fspeed_usage = (0..8)
            .map(|col| self.col_usage(layout, col) + self.col_fspeed(layout, col))
            .sum::<f64>();

        let scissors = self.scissor_score(layout);
        let lsbs = self.lsb_score(layout);
        let pinky_ring = self.pinky_ring_score(layout);
        let trigram_score = self.trigram_score_iter(layout, &self.data.trigrams);

        trigram_score - effort - fspeed_usage - scissors - lsbs - pinky_ring
    }

    fn weighted_bigrams(data: &LanguageData, weights: &Weights) -> BigramData {
        let len = data.characters.len();
        let chars = 0..len;

        chars
            .clone()
            .cartesian_product(chars)
            .map(|(c1, c2)| {
                let bigram = c1 * len + c2;
                let sfb = data.bigrams.get(bigram).unwrap_or(&0.0);
                let dsfb = data.skipgrams.get(bigram).unwrap_or(&0.0) * weights.dsfb_ratio;
                let dsfb2 = data.skipgrams2.get(bigram).unwrap_or(&0.0) * weights.dsfb_ratio2;
                let dsfb3 = data.skipgrams3.get(bigram).unwrap_or(&0.0) * weights.dsfb_ratio3;
                (sfb + dsfb + dsfb2 + dsfb3) * weights.fspeed
            })
            .collect()
    }

    fn per_char_trigrams(
        trigrams: &TrigramData,
        highest: u8,
        trigram_precision: usize,
    ) -> PerCharTrigrams {
        let mut n_trigrams = trigrams.clone();
        n_trigrams.truncate(trigram_precision);

        let thingy: Vec<([u8; 2], TrigramData)> = (0..highest)
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
                    .collect::<Vec<_>>();
                ([c1, c2], per_char)
            })
            .collect();

        PerCharTrigrams::from_iter(thingy)
    }

    #[inline]
    fn trigram_score_iter<'a, T>(&self, layout: &FastLayout, trigrams: T) -> f64
    where
        T: IntoIterator<Item = &'a ([u8; 3], f64)>,
    {
        use TrigramPattern::*;

        let mut freqs = TrigramStats::default();

        for (trigram, freq) in trigrams {
            match layout.get_trigram_pattern(trigram) {
                Alternate => freqs.alternates += freq,
                AlternateSfs => freqs.alternates_sfs += freq,
                Inroll => freqs.inrolls += freq,
                Outroll => freqs.outrolls += freq,
                Onehand => freqs.onehands += freq,
                Redirect => freqs.redirects += freq,
                RedirectSfs => freqs.redirects += freq,
                BadRedirect => freqs.bad_redirects += freq,
                BadRedirectSfs => freqs.bad_redirects += freq,
                _ => {}
            }
        }

        let mut score = 0.0;
        score += self.weights.inrolls * freqs.inrolls;
        score += self.weights.outrolls * freqs.outrolls;
        score += self.weights.onehands * freqs.onehands;
        score += self.weights.alternates * freqs.alternates;
        score += self.weights.alternates_sfs * freqs.alternates_sfs;
        score -= self.weights.redirects * freqs.redirects;
        score -= self.weights.redirects_sfs * freqs.redirects_sfs;
        score -= self.weights.bad_redirects * freqs.bad_redirects;
        score -= self.weights.bad_redirects_sfs * freqs.bad_redirects_sfs;
        score
    }

    fn trigram_char_score(&self, layout: &FastLayout, pos: &PosPair) -> f64 {
        let c1 = unsafe { layout.cu(pos.0) };
        let c2 = unsafe { layout.cu(pos.1) };

        if let Some(t_vec) = self.per_char_trigrams.get(&[c1, c2]) {
            self.trigram_score_iter(layout, t_vec)
        } else {
            0.0
        }
    }

    #[inline]
    fn scissor_score(&self, layout: &FastLayout) -> f64 {
        let mut res = 0.0;
        let len = self.data.characters.len();

        for PosPair(i1, i2) in self.scissor_indices {
            let c1 = unsafe { layout.cu(i1) } as usize;
            let c2 = unsafe { layout.cu(i2) } as usize;
            res += self.data.bigrams.get(c1 * len + c2).unwrap_or(&0.0);
            res += self.data.bigrams.get(c2 * len + c1).unwrap_or(&0.0);
        }

        res * self.weights.scissors
    }

    #[inline]
    fn lsb_score(&self, layout: &FastLayout) -> f64 {
        let mut res = 0.0;
        let len = self.data.characters.len();

        for PosPair(i1, i2) in self.lsb_indices {
            let c1 = unsafe { layout.cu(i1) } as usize;
            let c2 = unsafe { layout.cu(i2) } as usize;
            res += self.data.bigrams.get(c1 * len + c2).unwrap_or(&0.0);
            res += self.data.bigrams.get(c2 * len + c1).unwrap_or(&0.0);
        }

        res * self.weights.lsbs
    }

    fn pinky_ring_score(&self, layout: &FastLayout) -> f64 {
        let mut res = 0.0;
        let len = self.data.characters.len();

        for PosPair(i1, i2) in self.pinky_ring_indices {
            let c1 = unsafe { layout.cu(i1) } as usize;
            let c2 = unsafe { layout.cu(i2) } as usize;
            res += self.data.bigrams.get(c1 * len + c2).unwrap_or(&0.0);
            res += self.data.bigrams.get(c2 * len + c1).unwrap_or(&0.0);
        }

        res * self.weights.pinky_ring_bigrams
    }

    fn col_usage(&self, layout: &FastLayout, col: usize) -> f64 {
        let mut res = 0.0;
        match col {
            0..=2 => {
                for c in unsafe { [layout.cu(col), layout.cu(col + 10), layout.cu(col + 20)] } {
                    if let Some(v) = self.data.characters.get(c as usize) {
                        res += v;
                    }
                }
            }
            3 | 4 => {
                let col = (col - 3) * 2 + 3;
                for c in unsafe {
                    [
                        layout.cu(col),
                        layout.cu(col + 10),
                        layout.cu(col + 20),
                        layout.cu(col + 1),
                        layout.cu(col + 11),
                        layout.cu(col + 21),
                    ]
                } {
                    if let Some(v) = self.data.characters.get(c as usize) {
                        res += v;
                    }
                }
            }
            5..=7 => {
                let col = col + 2;
                for c in unsafe { [layout.cu(col), layout.cu(col + 10), layout.cu(col + 20)] } {
                    if let Some(v) = self.data.characters.get(c as usize) {
                        res += v;
                    }
                }
            }
            _ => unsafe { unreachable_unchecked() },
        };

        self.weights.max_finger_use.penalty
            * match col {
                0 | 7 => (res - self.weights.max_finger_use.pinky).max(0.0),
                1 | 6 => (res - self.weights.max_finger_use.ring).max(0.0),
                2 | 5 => (res - self.weights.max_finger_use.middle).max(0.0),
                3 | 4 => (res - self.weights.max_finger_use.index).max(0.0),
                _ => unsafe { unreachable_unchecked() },
            }
    }

    #[inline]
    fn pair_fspeed(&self, layout: &FastLayout, pair: &PosPair, dist: f64) -> f64 {
        let c1 = unsafe { layout.cu(pair.0) } as usize;
        let c2 = unsafe { layout.cu(pair.1) } as usize;
        // if c1 != self.repeat_key && c1 != self.repeat_key {
        // 	let mut res = 0.0;

        // 	let len = self.data.characters.len();
        // 	res += self.weighted_bigrams.get(c1 * len + c2).unwrap_or(&0.0) * dist;
        // 	res += self.weighted_bigrams.get(c2 * len + c1).unwrap_or(&0.0) * dist;
        // 	res
        // } else {
        // 	let mut res = 0.0;

        // 	let len = self.data.characters.len();
        // 	res += self.weighted_bigrams.get(c1 * len + c2).unwrap_or(&0.0) * dist * 0.5;
        // 	res += self.weighted_bigrams.get(c2 * len + c1).unwrap_or(&0.0) * dist * 0.5;
        // 	res
        // }
        let mut res = 0.0;

        let len = self.data.characters.len();
        res += self.weighted_bigrams.get(c1 * len + c2).unwrap_or(&0.0) * dist;
        res += self.weighted_bigrams.get(c2 * len + c1).unwrap_or(&0.0) * dist;
        res
    }

    #[inline(always)]
    unsafe fn col_to_start_len(col: usize) -> (usize, usize) {
        *[
            (0, 3),
            (3, 3),
            (6, 3),
            (18, 15),
            (33, 15),
            (9, 3),
            (12, 3),
            (15, 3),
        ]
        .get_unchecked(col)
    }

    #[inline]
    fn col_fspeed(&self, layout: &FastLayout, col: usize) -> f64 {
        let (start, len) = unsafe { Self::col_to_start_len(col) };
        let mut res = 0.0;

        for i in start..(start + len) {
            let (pair, dist) = unsafe { self.fspeed_vals.get_unchecked(i) };

            res += self.pair_fspeed(layout, pair, *dist);
        }
        res
    }

    #[inline]
    fn char_effort(&self, layout: &FastLayout, i: usize) -> f64 {
        let c = unsafe { layout.cu(i) };

        match self.data.characters.get(c as usize) {
            Some(&v) => v * unsafe { self.effort_map.get_unchecked(i) },
            None => 0.0,
        }
    }

    pub fn initialize_cache(&self, layout: &FastLayout) -> LayoutCache {
        let mut res = LayoutCache::default();

        for i in 0..layout.matrix.len() {
            res.effort[i] = self.char_effort(layout, i);
        }
        res.effort_total = res.effort.iter().sum();

        for col in 0..8 {
            res.usage[col] = self.col_usage(layout, col);
            res.fspeed[col] = self.col_fspeed(layout, col)
        }
        res.usage_total = res.usage.iter().sum();
        res.fspeed_total = res.fspeed.iter().sum();

        res.scissors = self.scissor_score(layout);

        res.lsbs = self.lsb_score(layout);

        res.pinky_ring = self.pinky_ring_score(layout);

        res.trigrams_total =
            self.trigram_score_iter(layout, self.data.trigrams.iter().take(100000));

        res.total_score = res.total_score();

        res
    }

    pub fn score_swap_cached(
        &self,
        layout: &mut FastLayout,
        swap: &PosPair,
        cache: &LayoutCache,
    ) -> f64 {
        unsafe { layout.swap_no_bounds(swap) };

        let PosPair(i1, i2) = *swap;

        let col1 = I_TO_COL[i1];
        let col2 = I_TO_COL[i2];

        let fspeed_score = if col1 == col2 {
            let fspeed = self.col_fspeed(layout, col1);
            
            cache.fspeed_total - cache.fspeed[col1] + fspeed
        } else {
            let fspeed1 = self.col_fspeed(layout, col1);
            let fspeed2 = self.col_fspeed(layout, col2);
            
            cache.fspeed_total - cache.fspeed[col1] - cache.fspeed[col2] + fspeed1 + fspeed2
        };

        let usage_score = if col1 == col2 {
            let usage = self.col_usage(layout, col1);
            cache.usage_total - cache.usage[col1] + usage
        } else {
            let usage1 = self.col_usage(layout, col1);
            let usage2 = self.col_usage(layout, col2);
            cache.usage_total - cache.usage[col1] - cache.usage[col2] + usage1 + usage2
        };

        let effort1 = self.char_effort(layout, i1);
        let effort2 = self.char_effort(layout, i2);
        let effort_score =
            cache.effort_total - cache.effort[i1] - cache.effort[i2] + effort1 + effort2;

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

        // let _new_heur = cache.trigrams_total - scissors_score - effort_score - usage_score - fspeed_score;

        let trigrams_score = if cache.total_score < (f64::MAX) {
            //new_heur + new_heur.abs() * 0.0) {
            let trigrams_end = self.trigram_char_score(layout, swap);
            unsafe { layout.swap_no_bounds(swap) };
            let trigrams_start = self.trigram_char_score(layout, swap);

            #[cfg(test)]
            NOT_PRUNED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            cache.trigrams_total - trigrams_start + trigrams_end
        } else {
            #[cfg(test)]
            PRUNED_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            unsafe { layout.swap_no_bounds(swap) };
            return f64::MIN + 1000.0;
        };

        trigrams_score - scissors_score - lsbs_score - pinky_ring_score - effort_score - usage_score - fspeed_score
    }

    pub fn accept_swap(&self, layout: &mut FastLayout, swap: &PosPair, cache: &mut LayoutCache) {
        let trigrams_start = self.trigram_char_score(layout, swap);

        unsafe { layout.swap_no_bounds(swap) };

        let PosPair(i1, i2) = *swap;

        let col1 = I_TO_COL[i1];
        let col2 = I_TO_COL[i2];

        cache.fspeed_total = if col1 == col2 {
            let fspeed = self.col_fspeed(layout, col1);
            let total = cache.fspeed_total - cache.fspeed[col1] + fspeed;

            cache.fspeed[col1] = fspeed;

            total
        } else {
            let fspeed1 = self.col_fspeed(layout, col1);
            let fspeed2 = self.col_fspeed(layout, col2);
            let total =
                cache.fspeed_total - cache.fspeed[col1] - cache.fspeed[col2] + fspeed1 + fspeed2;

            cache.fspeed[col1] = fspeed1;
            cache.fspeed[col2] = fspeed2;

            total
        };

        cache.usage_total = if col1 == col2 {
            let usage = self.col_usage(layout, col1);
            let total = cache.usage_total - cache.usage[col1] + usage;

            cache.usage[col1] = usage;

            total
        } else {
            let usage1 = self.col_usage(layout, col1);
            let usage2 = self.col_usage(layout, col2);
            let total = cache.usage_total - cache.usage[col1] - cache.usage[col2] + usage1 + usage2;

            cache.usage[col1] = usage1;
            cache.usage[col2] = usage2;

            total
        };

        let effort1 = self.char_effort(layout, i1);
        let effort2 = self.char_effort(layout, i2);
        cache.effort_total =
            cache.effort_total - cache.effort[i1] - cache.effort[i2] + effort1 + effort2;

        cache.effort[i1] = effort1;
        cache.effort[i2] = effort2;

        let trigrams_end = self.trigram_char_score(layout, swap);
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
        current_best_score: Option<f64>,
        possible_swaps: &[PosPair],
    ) -> (Option<PosPair>, f64) {
        let mut best_score = current_best_score.unwrap_or(f64::MIN / 2.0);
        let mut best_swap: Option<PosPair> = None;

        for swap in possible_swaps {
            let score = self.score_swap_cached(layout, swap, cache);

            if score > best_score {
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
    ) -> f64 {
        let mut current_best_score = f64::MIN / 2.0;

        while let (Some(best_swap), new_score) =
            self.best_swap_cached(layout, cache, Some(current_best_score), possible_swaps)
        {
            current_best_score = new_score;
            self.accept_swap(layout, &best_swap, cache);
        }
        current_best_score
    }

    fn optimize_cols(&self, layout: &mut FastLayout, cache: &mut LayoutCache, score: Option<f64>) {
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
        best_score: &mut f64,
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
            if k % 2 == 0 {
                self.accept_swap(layout, &PosPair(COLS[i], COLS[k - 1]), cache);
            } else {
                self.accept_swap(layout, &PosPair(COLS[0], COLS[k - 1]), cache);
            }
        });
    }

    pub fn generate(&self) -> FastLayout {
        let layout = FastLayout::random(self.chars_for_generation);
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
        let mut with_col_score = f64::MIN;
        let mut optimized_score = f64::MIN / 2.0;

        while with_col_score < optimized_score {
            optimized_score = self.optimize_cached(&mut layout, cache, possible_swaps);
            self.optimize_cols(&mut layout, cache, Some(optimized_score));
            with_col_score = layout.score;
        }

        layout.score = optimized_score;
        layout
    }

    pub fn optimize_mut(
        &self,
        layout: &mut FastLayout,
        cache: &mut LayoutCache,
        possible_swaps: &[PosPair],
    ) {
        let mut with_col_score = f64::MIN;
        let mut optimized_score = f64::MIN / 2.0;

        while with_col_score < optimized_score {
            optimized_score = self.optimize_cached(layout, cache, possible_swaps);
            self.optimize_cols(layout, cache, Some(optimized_score));
            with_col_score = layout.score;
        }

        layout.score = optimized_score;
    }

    pub fn generate_n_iter(&self, amount: usize) -> impl ParallelIterator<Item = FastLayout> + '_ {
        (0..amount).into_par_iter().map(|_| self.generate())
    }

    pub fn generate_n_with_pins_iter<'a>(
        &'a self,
        amount: usize,
        based_on: FastLayout,
        pins: &'a [usize],
    ) -> impl ParallelIterator<Item = FastLayout> + '_ {
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
        let mut layout = FastLayout::random_pins(based_on.matrix, pins);
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
    use crate::utility::ApproxEq;
    use nanorand::Rng;
    use once_cell::sync::Lazy;
    use std::sync::atomic::Ordering;

    static GEN: Lazy<LayoutGeneration> =
        Lazy::new(|| LayoutGeneration::new("english", "static", None).unwrap());

    #[allow(dead_code)]
    fn fspeed_per_pair() {
        for (pair, dist) in GEN.fspeed_vals {
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
    fn prune_heuristic_correctness() {
        //has been tested with 10000 runs
        let runs = 200;

        for _ in 0..runs {
            let mut layout = FastLayout::random(GEN.chars_for_generation);
            let cache = GEN.initialize_cache(&layout);

            if let (Some(best_swap_normal), best_score_normal) =
                GEN.best_swap(&mut layout, None, &POSSIBLE_SWAPS)
            {
                if let (Some(best_swap_cached), best_score_cached) =
                    GEN.best_swap_cached(&mut layout, &cache, None, &POSSIBLE_SWAPS)
                {
                    if best_score_normal.approx_eq_dbg(best_score_cached, 7) {
                        assert_eq!(best_swap_normal, best_swap_cached);
                    }
                }
            }
        }
        println!(
			"pruned {} times.\nRecalculated trigrams {} times.\namount pruned: {:.2}%\n analyzed {} swaps",
			PRUNED_COUNT.load(Ordering::Relaxed),
			435 * runs - PRUNED_COUNT.load(Ordering::Relaxed),
			(PRUNED_COUNT.load(Ordering::Relaxed) as f64) / (435.0 * runs as f64) * 100.0,
			435 * runs
		);
    }

    #[test]
    fn cached_totals() {
        let qwerty_bytes = GEN
            .convert_u8
            .to_lossy("qwertyuiopasdfghjkl;zxcvbnm,./".chars());
        let mut qwerty = FastLayout::try_from(qwerty_bytes.as_slice()).unwrap();
        let mut cache = GEN.initialize_cache(&qwerty);
        let mut rng = nanorand::tls_rng();

        for swap in (0..)
            .map(|_| &POSSIBLE_SWAPS[rng.generate_range(0..435)])
            .take(10000)
        {
            GEN.accept_swap(&mut qwerty, swap, &mut cache);

            assert!(cache.scissors.approx_eq_dbg(GEN.scissor_score(&qwerty), 7));
            assert!(cache
                .effort_total
                .approx_eq_dbg(GEN.effort_score(&qwerty), 7));
            assert!(cache.usage_total.approx_eq_dbg(GEN.usage_score(&qwerty), 7));
            assert!(cache
                .fspeed_total
                .approx_eq_dbg(GEN.fspeed_score(&qwerty), 7));
            assert!(cache.trigrams_total.approx_eq_dbg(
                GEN.trigram_score_iter(&qwerty, GEN.data.trigrams.iter().take(1000)),
                7
            ));
            assert!(cache.lsbs.approx_eq_dbg(GEN.lsb_score(&qwerty), 7));
            assert!(cache.pinky_ring.approx_eq_dbg(GEN.pinky_ring_score(&qwerty), 7));
            assert!(cache
                .total_score
                .approx_eq_dbg(GEN.score_with_precision(&qwerty, 1000), 7));
        }
    }

    #[test]
    fn best_found_swap() {
        let qwerty_bytes = GEN
            .convert_u8
            .to_lossy("qwertyuiopasdfghjkl;zxcvbnm,./".chars());
        let mut qwerty = FastLayout::try_from(qwerty_bytes.as_slice()).unwrap();
        let cache = GEN.initialize_cache(&qwerty);

        if let (Some(best_swap_normal), best_score_normal) =
            GEN.best_swap(&mut qwerty, None, &POSSIBLE_SWAPS)
        {
            if let (Some(best_swap_cached), best_score_cached) =
                GEN.best_swap_cached(&mut qwerty, &cache, None, &POSSIBLE_SWAPS)
            {
                if best_score_normal.approx_eq_dbg(best_score_cached, 7) {
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
            .convert_u8
            .to_lossy("qwertyuiopasdfghjkl;zxcvbnm,./".chars());
        let base = FastLayout::try_from(qwerty_bytes.as_slice()).unwrap();
        let mut qwerty = base.clone();
        let cache = GEN.initialize_cache(&qwerty);

        for (i, swap) in POSSIBLE_SWAPS.iter().enumerate() {
            let score_normal = GEN.score_swap(&mut qwerty, swap);
            let score_cached = GEN.score_swap_cached(&mut qwerty, swap, &cache);

            assert_eq!(base, qwerty);

            assert!(
                score_cached == f64::MIN + 1000.0 || score_normal.approx_eq_dbg(score_cached, 7),
                "failed on iteration {i} for {}",
                POSSIBLE_SWAPS[i]
            );
        }
    }

    #[test]
    fn optimize_qwerty() {
        let qwerty_bytes = GEN
            .convert_u8
            .to_lossy("qwertyuiopasdfghjkl;zxcvbnm,./".chars());
        let qwerty = FastLayout::try_from(qwerty_bytes.as_slice()).unwrap();

        let optimized_normal = GEN.optimize_normal_no_cols(qwerty.clone(), &POSSIBLE_SWAPS);
        let normal_score = GEN.score_with_precision(&optimized_normal, 100000);

        let mut qwerty_for_cached = FastLayout::try_from(qwerty_bytes.as_slice()).unwrap();
        let mut cache = GEN.initialize_cache(&qwerty_for_cached);

        let best_cached_score =
            GEN.optimize_cached(&mut qwerty_for_cached, &mut cache, &POSSIBLE_SWAPS);

        assert!(normal_score.approx_eq_dbg(best_cached_score, 7));
        assert_eq!(
            qwerty_for_cached.layout_str(&GEN.convert_u8),
            optimized_normal.layout_str(&GEN.convert_u8)
        );
        // println!("{qwerty_for_cached}");
    }

    #[test]
    fn optimize_random_layouts() {
        for _ in 0..5 {
            let layout = FastLayout::random(GEN.chars_for_generation);
            let mut layout_for_cached = layout.clone();

            let optimized_normal = GEN.optimize_normal_no_cols(layout, &POSSIBLE_SWAPS);
            let normal_score = GEN.score_with_precision(&optimized_normal, 100000);

            let mut cache = GEN.initialize_cache(&layout_for_cached);
            let best_cached_score =
                GEN.optimize_cached(&mut layout_for_cached, &mut cache, &POSSIBLE_SWAPS);

            assert!(normal_score.approx_eq_dbg(best_cached_score, 7));
            assert_eq!(
                layout_for_cached.layout_str(&GEN.convert_u8),
                optimized_normal.layout_str(&GEN.convert_u8)
            );
        }
    }
}
