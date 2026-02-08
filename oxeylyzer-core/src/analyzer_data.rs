use std::sync::Arc;

use crate::{data::Data, o2_char_mapping::CharMapping, weights::Weights};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnalyzerData {
    name: String,
    chars: Box<[i64]>,
    bigrams: Box<[i64]>,
    skipgrams: Box<[i64]>,
    skipgrams2: Box<[i64]>,
    skipgrams3: Box<[i64]>,
    trigrams: Box<[i64]>,
    same_finger_weighted_bigrams: Box<[i64]>,
    stretch_weighted_bigrams: Box<[i64]>,
    pub char_total: f64,
    pub bigram_total: f64,
    pub skipgram_total: f64,
    pub trigram_total: f64,
    pub mapping: Arc<CharMapping>,
}

impl AnalyzerData {
    pub fn new(data: Data, weights: &Weights) -> Self {
        let char_total = data.char_total as f64 / 100.0;
        let bigram_total = data.bigram_total as f64 / 100.0;
        let skipgram_total = data.skipgram_total as f64 / 100.0;
        let skipgram2_total = data.skipgram2_total as f64 / 100.0;
        let skipgram3_total = data.skipgram3_total as f64 / 100.0;
        let trigram_total = data.trigram_total as f64 / 100.0;

        let mut chars = vec![0; data.chars.len() + 2];
        let mut mapping = CharMapping::new();

        for (c, f) in data.chars {
            mapping.push(c);

            let i = mapping.get_u(c) as usize;
            chars[i] = (f * char_total) as i64;
        }

        debug_assert!(chars.len() >= mapping.len());

        chars.truncate(mapping.len());

        let len = chars.len();

        // let same_finger_weighted_bigrams = vec![0; len.pow(2)];

        // for ([c1, c2], f) in &data.bigrams {
        //     let u1 = mapping.get_u(c1) as usize;
        //     let u2 = mapping.get_u(c2) as usize;

        //     let i = u1 * len + u2;
        //     debug_assert_eq!(bigrams[i], 0);
        //     bigrams[i] = (f * bigram_total) as i64;
        // }

        let mut bigrams = vec![0; len.pow(2)];

        for ([c1, c2], f) in data.bigrams {
            let u1 = mapping.get_u(c1) as usize;
            let u2 = mapping.get_u(c2) as usize;

            let i = u1 * len + u2;
            debug_assert_eq!(bigrams[i], 0);
            bigrams[i] = (f * bigram_total) as i64;
        }

        let mut skipgrams = vec![0; len.pow(2)];

        for ([c1, c2], f) in data.skipgrams {
            let u1 = mapping.get_u(c1) as usize;
            let u2 = mapping.get_u(c2) as usize;

            let i = u1 * len + u2;
            debug_assert_eq!(skipgrams[i], 0);
            skipgrams[i] = (f * skipgram_total) as i64;
        }

        let mut skipgrams2 = vec![0; len.pow(2)];

        for ([c1, c2], f) in data.skipgrams2 {
            let u1 = mapping.get_u(c1) as usize;
            let u2 = mapping.get_u(c2) as usize;

            let i = u1 * len + u2;
            debug_assert_eq!(skipgrams2[i], 0);
            skipgrams2[i] = (f * skipgram2_total) as i64;
        }

        let mut skipgrams3 = vec![0; len.pow(2)];

        for ([c1, c2], f) in data.skipgrams3 {
            let u1 = mapping.get_u(c1) as usize;
            let u2 = mapping.get_u(c2) as usize;

            let i = u1 * len + u2;
            debug_assert_eq!(skipgrams3[i], 0);
            skipgrams3[i] = (f * skipgram3_total) as i64;
        }

        let mut trigrams = vec![0; len.pow(3)];

        for ([c1, c2, c3], f) in data.trigrams {
            let u1 = mapping.get_u(c1) as usize;
            let u2 = mapping.get_u(c2) as usize;
            let u3 = mapping.get_u(c3) as usize;

            let i = u1 * len.pow(2) + u2 * len + u3;
            debug_assert_eq!(trigrams[i], 0);
            trigrams[i] = (f * trigram_total) as i64;
        }

        let same_finger_weighted_bigrams = bigrams
            .iter()
            .zip(&skipgrams)
            .zip(&skipgrams2)
            .zip(&skipgrams3)
            .map(|(((&b, &s), s2), s3)| {
                let sfb = -b as f64;
                let sfs = (-s as f64) * weights.dsfb_ratio;
                let sfs2 = (-s2 as f64) * weights.dsfb_ratio2;
                let sfs3 = (-s3 as f64) * weights.dsfb_ratio3;
                ((sfb + sfs + sfs2 + sfs3) * weights.fspeed) as i64
            })
            .collect::<Box<_>>();

        let stretch_weighted_bigrams = bigrams
            .iter()
            .zip(&skipgrams)
            .zip(&skipgrams2)
            .zip(&skipgrams3)
            .map(|(((&b, &s), s2), s3)| {
                let sfb = -b as f64;
                let sfs = (-s as f64) * weights.dsfb_ratio;
                let sfs2 = (-s2 as f64) * weights.dsfb_ratio2;
                let sfs3 = (-s3 as f64) * weights.dsfb_ratio3;
                ((sfb + sfs + sfs2 + sfs3) * weights.stretches) as i64
            })
            .collect::<Box<_>>();

        let mapping = Arc::new(mapping);

        Self {
            name: data.name,
            chars: chars.into(),
            bigrams: bigrams.into(),
            skipgrams: skipgrams.into(),
            skipgrams2: skipgrams2.into(),
            skipgrams3: skipgrams3.into(),
            trigrams: trigrams.into(),
            same_finger_weighted_bigrams,
            stretch_weighted_bigrams,

            char_total,
            bigram_total,
            skipgram_total,
            trigram_total,

            mapping,
        }
    }

    pub fn len(&self) -> usize {
        self.chars.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn get_char(&self, c: char) -> i64 {
        let i = self.mapping.get_u(c) as usize;
        self.chars[i]
    }

    pub fn get_bigram(&self, [c1, c2]: [char; 2]) -> i64 {
        let u1 = self.mapping.get_u(c1) as usize;
        let u2 = self.mapping.get_u(c2) as usize;

        let i = u1 * self.len() + u2;
        self.bigrams[i]
    }

    pub fn get_skipgram(&self, [c1, c2]: [char; 2]) -> i64 {
        let u1 = self.mapping.get_u(c1) as usize;
        let u2 = self.mapping.get_u(c2) as usize;

        let i = u1 * self.len() + u2;
        self.skipgrams[i]
    }

    pub fn get_trigram(&self, [c1, c2, c3]: [char; 3]) -> i64 {
        let u1 = self.mapping.get_u(c1) as usize;
        let u2 = self.mapping.get_u(c2) as usize;
        let u3 = self.mapping.get_u(c3) as usize;

        let i = u1 * self.len().pow(2) + u2 * self.len() + u3;
        self.trigrams[i]
    }

    pub fn get_same_finger_weighted_bigram(&self, [c1, c2]: [char; 2]) -> i64 {
        let u1 = self.mapping.get_u(c1) as usize;
        let u2 = self.mapping.get_u(c2) as usize;

        let i = u1 * self.len() + u2;
        self.same_finger_weighted_bigrams[i]
    }

    pub fn get_stretch_weighted_bigram(&self, [c1, c2]: [char; 2]) -> i64 {
        let u1 = self.mapping.get_u(c1) as usize;
        let u2 = self.mapping.get_u(c2) as usize;

        let i = u1 * self.len() + u2;
        self.stretch_weighted_bigrams[i]
    }

    #[inline]
    pub fn get_char_u(&self, c: u8) -> i64 {
        self.chars[c as usize]
    }

    #[inline]
    pub fn get_bigram_u(&self, [c1, c2]: [u8; 2]) -> i64 {
        let u1 = c1 as usize;
        let u2 = c2 as usize;

        let i = u1 * self.len() + u2;
        self.bigrams[i]
    }

    #[inline]
    pub fn get_skipgram_u(&self, [c1, c2]: [u8; 2]) -> i64 {
        let u1 = c1 as usize;
        let u2 = c2 as usize;

        let i = u1 * self.len() + u2;
        self.skipgrams[i]
    }

    #[inline]
    pub fn get_trigram_u(&self, [c1, c2, c3]: [u8; 3]) -> i64 {
        let u1 = c1 as usize;
        let u2 = c2 as usize;
        let u3 = c3 as usize;

        let i = u1 * self.len().pow(2) + u2 * self.len() + u3;
        self.trigrams[i]
    }

    #[inline]
    pub fn get_same_finger_weighted_bigram_u(&self, [c1, c2]: [u8; 2]) -> i64 {
        let u1 = c1 as usize;
        let u2 = c2 as usize;

        let i = u1 * self.len() + u2;
        self.same_finger_weighted_bigrams[i]
    }

    #[inline]
    pub fn get_stretch_weighted_bigram_u(&self, [c1, c2]: [u8; 2]) -> i64 {
        let u1 = c1 as usize;
        let u2 = c2 as usize;

        let i = u1 * self.len() + u2;
        self.stretch_weighted_bigrams[i]
    }
}
