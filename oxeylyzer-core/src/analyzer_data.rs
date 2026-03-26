use std::sync::Arc;

use crate::{char_mapping::CharMapping, data::Data, weights::Weights};

/// Optimized data structure for layout analysis, containing frequency information for various n-grams.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnalyzerData {
    name: String,
    chars: Box<[i64]>,
    bigrams: Box<[i64]>,
    skipgrams: Box<[i64]>,
    skipgrams2: Box<[i64]>,
    skipgrams3: Box<[i64]>,
    trigrams: Box<[i64]>,
    gen_trigrams: Box<[([u8; 3], i64)]>,
    same_finger_weighted_bigrams: Box<[i64]>,
    stretch_weighted_bigrams: Box<[i64]>,
    /// Total number of characters in the corpus.
    pub char_total: i64,
    /// Total number of bigrams in the corpus.
    pub bigram_total: i64,
    /// Total number of skipgrams (1 distance) in the corpus.
    pub skipgram_total: i64,
    /// Total number of skipgrams (2 distance) in the corpus.
    pub skipgram2_total: i64,
    /// Total number of skipgrams (3 distance) in the corpus.
    pub skipgram3_total: i64,
    /// Total number of trigrams in the corpus.
    pub trigram_total: i64,
    /// Mapping between characters and their internal byte representations.
    pub mapping: Arc<CharMapping>,
}

impl AnalyzerData {
    /// Creates a new `AnalyzerData` from raw corpus data and layout weights.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::analyzer_data::AnalyzerData;
    /// use oxeylyzer_core::data::Data;
    /// use oxeylyzer_core::weights::Weights;
    ///
    /// let data = Data::default();
    /// let weights = Weights::default();
    /// let analyzer_data = AnalyzerData::new(data, &weights);
    ///
    /// assert_eq!(analyzer_data.char_total, 0);
    /// ```
    pub fn new(data: Data, weights: &Weights) -> Self {
        let convert_f = |f| f / 100.0;

        let char_total = data.char_total;
        let bigram_total = data.bigram_total;
        let skipgram_total = data.skipgram_total;
        let skipgram2_total = data.skipgram2_total;
        let skipgram3_total = data.skipgram3_total;
        let trigram_total = data.trigram_total;

        let mut chars = vec![0; data.chars.len() + 3];
        let mut mapping = CharMapping::new();

        for (c, f) in data.chars {
            mapping.push(c);

            let i = mapping.get_u(c) as usize;
            chars[i] = (convert_f(f) * data.char_total as f64) as i64;
        }

        debug_assert!(chars.len() >= mapping.len());

        chars.truncate(mapping.len());

        let len = chars.len();

        let mut bigrams = vec![0; len.pow(2)];

        for ([c1, c2], f) in data.bigrams {
            let u1 = mapping.get_u(c1) as usize;
            let u2 = mapping.get_u(c2) as usize;

            let i = u1 * len + u2;
            debug_assert_eq!(bigrams[i], 0);
            bigrams[i] = (convert_f(f) * bigram_total as f64) as i64;
        }

        let mut skipgrams = vec![0; len.pow(2)];

        for ([c1, c2], f) in data.skipgrams {
            let u1 = mapping.get_u(c1) as usize;
            let u2 = mapping.get_u(c2) as usize;

            let i = u1 * len + u2;
            debug_assert_eq!(skipgrams[i], 0);
            skipgrams[i] = (convert_f(f) * skipgram_total as f64) as i64;
        }

        let mut skipgrams2 = vec![0; len.pow(2)];

        for ([c1, c2], f) in data.skipgrams2 {
            let u1 = mapping.get_u(c1) as usize;
            let u2 = mapping.get_u(c2) as usize;

            let i = u1 * len + u2;
            debug_assert_eq!(skipgrams2[i], 0);
            skipgrams2[i] = (convert_f(f) * skipgram2_total as f64) as i64;
        }

        let mut skipgrams3 = vec![0; len.pow(2)];

        for ([c1, c2], f) in data.skipgrams3 {
            let u1 = mapping.get_u(c1) as usize;
            let u2 = mapping.get_u(c2) as usize;

            let i = u1 * len + u2;
            debug_assert_eq!(skipgrams3[i], 0);
            skipgrams3[i] = (convert_f(f) * skipgram3_total as f64) as i64;
        }

        let mut trigrams = vec![0; len.pow(3)];

        for (&[c1, c2, c3], &f) in data.trigrams.iter() {
            let u1 = mapping.get_u(c1) as usize;
            let u2 = mapping.get_u(c2) as usize;
            let u3 = mapping.get_u(c3) as usize;

            let i = u1 * len.pow(2) + u2 * len + u3;
            debug_assert_eq!(trigrams[i], 0);
            trigrams[i] = (convert_f(f) * trigram_total as f64) as i64;
        }

        let gen_trigrams = data
            .trigrams
            .into_iter()
            .map(|([c1, c2, c3], f)| {
                let u1 = mapping.get_u(c1);
                let u2 = mapping.get_u(c2);
                let u3 = mapping.get_u(c3);
                ([u1, u2, u3], (convert_f(f) * trigram_total as f64) as i64)
            })
            .collect::<Box<_>>();

        let dsfb_ratio = weights.sfs / weights.sfbs;

        let sfwb = bigrams
            .iter()
            .zip(&skipgrams)
            .zip(&skipgrams2)
            .zip(&skipgrams3)
            .map(|(((&b, &s), &s2), &s3)| {
                let sfb = b as f64;
                let sfs = (s as f64) * dsfb_ratio;
                let sfs2 = (s2 as f64) * dsfb_ratio.powi(2);
                let sfs3 = (s3 as f64) * dsfb_ratio.powi(3);
                ((sfb + sfs + sfs2 + sfs3) * weights.sfbs) as i64
            })
            .collect::<Vec<_>>();

        let swb = bigrams
            .iter()
            .zip(&skipgrams)
            .zip(&skipgrams2)
            .zip(&skipgrams3)
            .map(|(((&b, &s), &s2), &s3)| {
                let sfb = b as f64;
                let sfs = (s as f64) * dsfb_ratio;
                let sfs2 = (s2 as f64) * dsfb_ratio.powi(2);
                let sfs3 = (s3 as f64) * dsfb_ratio.powi(3);
                ((sfb + sfs + sfs2 + sfs3) * weights.stretches) as i64
            })
            .collect::<Vec<_>>();

        let same_finger_weighted_bigrams = (0..sfwb.len())
            .map(|i| {
                let u1 = i / len;
                let u2 = i % len;
                let j = u2 * len + u1;
                sfwb[i] + sfwb[j]
            })
            .collect::<Box<_>>();

        let stretch_weighted_bigrams = (0..swb.len())
            .map(|i| {
                let u1 = i / len;
                let u2 = i % len;
                let j = u2 * len + u1;
                swb[i] + swb[j]
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
            gen_trigrams,
            same_finger_weighted_bigrams,
            stretch_weighted_bigrams,

            char_total,
            bigram_total,
            skipgram_total,
            skipgram2_total,
            skipgram3_total,
            trigram_total,

            mapping,
        }
    }

    /// Returns the number of unique characters in the mapping.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::analyzer_data::AnalyzerData;
    ///
    /// let analyzer_data = AnalyzerData::default();
    /// assert_eq!(analyzer_data.len(), 0);
    /// ```
    pub fn len(&self) -> usize {
        self.chars.len()
    }

    /// Returns true if there are no characters in the data.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::analyzer_data::AnalyzerData;
    ///
    /// let analyzer_data = AnalyzerData::default();
    /// assert!(analyzer_data.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    /// Returns the name of the corpus.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::analyzer_data::AnalyzerData;
    ///
    /// let analyzer_data = AnalyzerData::default();
    /// assert_eq!(analyzer_data.name(), "");
    /// ```
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the character frequency data.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::analyzer_data::AnalyzerData;
    ///
    /// let analyzer_data = AnalyzerData::default();
    /// assert!(analyzer_data.chars().is_empty());
    /// ```
    pub fn chars(&self) -> &[i64] {
        &self.chars
    }

    /// Returns the bigram frequency data.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::analyzer_data::AnalyzerData;
    ///
    /// let analyzer_data = AnalyzerData::default();
    /// assert!(analyzer_data.bigrams().is_empty());
    /// ```
    pub fn bigrams(&self) -> &[i64] {
        &self.bigrams
    }

    /// Returns the skipgram (1 distance) frequency data.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::analyzer_data::AnalyzerData;
    ///
    /// let analyzer_data = AnalyzerData::default();
    /// assert!(analyzer_data.skipgrams().is_empty());
    /// ```
    pub fn skipgrams(&self) -> &[i64] {
        &self.skipgrams
    }

    /// Returns the skipgram (2 distance) frequency data.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::analyzer_data::AnalyzerData;
    ///
    /// let analyzer_data = AnalyzerData::default();
    /// assert!(analyzer_data.skipgrams2().is_empty());
    /// ```
    pub fn skipgrams2(&self) -> &[i64] {
        &self.skipgrams2
    }

    /// Returns the skipgram (3 distance) frequency data.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::analyzer_data::AnalyzerData;
    ///
    /// let analyzer_data = AnalyzerData::default();
    /// assert!(analyzer_data.skipgrams3().is_empty());
    /// ```
    pub fn skipgrams3(&self) -> &[i64] {
        &self.skipgrams3
    }

    /// Returns the trigram frequency data.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::analyzer_data::AnalyzerData;
    ///
    /// let analyzer_data = AnalyzerData::default();
    /// assert!(analyzer_data.trigrams().is_empty());
    /// ```
    pub fn trigrams(&self) -> &[i64] {
        &self.trigrams
    }

    /// Returns the trigram data formatted for generation.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::analyzer_data::AnalyzerData;
    ///
    /// let analyzer_data = AnalyzerData::default();
    /// assert!(analyzer_data.gen_trigrams().is_empty());
    /// ```
    pub fn gen_trigrams(&self) -> &[([u8; 3], i64)] {
        &self.gen_trigrams
    }

    /// Returns the frequency of a specific character.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::analyzer_data::AnalyzerData;
    /// let analyzer_data = AnalyzerData::default();
    /// let freq = analyzer_data.get_char('e');
    /// ```
    pub fn get_char(&self, c: char) -> i64 {
        let u = self.mapping.get_u(c);

        self.get_char_u(u)
    }

    /// Returns the frequency of a specific bigram.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::analyzer_data::AnalyzerData;
    /// let analyzer_data = AnalyzerData::default();
    /// let freq = analyzer_data.get_bigram(['t', 'h']);
    /// ```
    pub fn get_bigram(&self, [c1, c2]: [char; 2]) -> i64 {
        let u1 = self.mapping.get_u(c1);
        let u2 = self.mapping.get_u(c2);

        self.get_bigram_u([u1, u2])
    }

    /// Returns the frequency of a specific skipgram (1 distance).
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::analyzer_data::AnalyzerData;
    /// let analyzer_data = AnalyzerData::default();
    /// let freq = analyzer_data.get_skipgram(['a', 'n']);
    /// ```
    pub fn get_skipgram(&self, [c1, c2]: [char; 2]) -> i64 {
        let u1 = self.mapping.get_u(c1);
        let u2 = self.mapping.get_u(c2);

        self.get_skipgram_u([u1, u2])
    }

    /// Returns the frequency of a specific trigram.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::analyzer_data::AnalyzerData;
    /// let analyzer_data = AnalyzerData::default();
    /// let freq = analyzer_data.get_trigram(['t', 'h', 'e']);
    /// ```
    pub fn get_trigram(&self, [c1, c2, c3]: [char; 3]) -> i64 {
        let u1 = self.mapping.get_u(c1);
        let u2 = self.mapping.get_u(c2);
        let u3 = self.mapping.get_u(c3);

        self.get_trigram_u([u1, u2, u3])
    }

    /// Returns the weighted frequency of a bigram for same-finger analysis.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::analyzer_data::AnalyzerData;
    /// let analyzer_data = AnalyzerData::default();
    /// let weighted_freq = analyzer_data.get_same_finger_weighted_bigram(['d', 'e']);
    /// ```
    pub fn get_same_finger_weighted_bigram(&self, [c1, c2]: [char; 2]) -> i64 {
        let u1 = self.mapping.get_u(c1);
        let u2 = self.mapping.get_u(c2);

        self.get_same_finger_weighted_bigram_u([u1, u2])
    }

    /// Returns the weighted frequency of a bigram for stretch analysis.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::analyzer_data::AnalyzerData;
    /// let analyzer_data = AnalyzerData::default();
    /// let weighted_freq = analyzer_data.get_stretch_weighted_bigram(['q', 'u']);
    /// ```
    pub fn get_stretch_weighted_bigram(&self, [c1, c2]: [char; 2]) -> i64 {
        let u1 = self.mapping.get_u(c1);
        let u2 = self.mapping.get_u(c2);

        self.get_stretch_weighted_bigram_u([u1, u2])
    }

    /// Returns the frequency of a character by its internal byte representation.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::analyzer_data::AnalyzerData;
    ///
    /// let analyzer_data = AnalyzerData::default();
    /// assert_eq!(analyzer_data.get_char_u(0), 0);
    /// ```
    #[inline]
    pub fn get_char_u(&self, c: u8) -> i64 {
        self.chars.get(c as usize).copied().unwrap_or_default()
    }

    /// Returns the frequency of a bigram by its internal byte representations.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::analyzer_data::AnalyzerData;
    ///
    /// let analyzer_data = AnalyzerData::default();
    /// assert_eq!(analyzer_data.get_bigram_u([0, 1]), 0);
    /// ```
    #[inline]
    pub fn get_bigram_u(&self, [c1, c2]: [u8; 2]) -> i64 {
        let u1 = c1 as usize;
        let u2 = c2 as usize;

        if u1 < self.len() && u2 < self.len() {
            let i = u1 * self.len() + u2;
            self.bigrams[i]
        } else {
            0
        }
    }

    /// Returns the frequency of a skipgram (1 distance) by its internal byte representations.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::analyzer_data::AnalyzerData;
    ///
    /// let analyzer_data = AnalyzerData::default();
    /// assert_eq!(analyzer_data.get_skipgram_u([0, 1]), 0);
    /// ```
    #[inline]
    pub fn get_skipgram_u(&self, [c1, c2]: [u8; 2]) -> i64 {
        let u1 = c1 as usize;
        let u2 = c2 as usize;

        if u1 < self.len() && u2 < self.len() {
            let i = u1 * self.len() + u2;
            self.skipgrams[i]
        } else {
            0
        }
    }

    /// Returns the frequency of a trigram by its internal byte representations.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::analyzer_data::AnalyzerData;
    ///
    /// let analyzer_data = AnalyzerData::default();
    /// assert_eq!(analyzer_data.get_trigram_u([0, 1, 2]), 0);
    /// ```
    #[inline]
    pub fn get_trigram_u(&self, [c1, c2, c3]: [u8; 3]) -> i64 {
        let u1 = c1 as usize;
        let u2 = c2 as usize;
        let u3 = c3 as usize;

        if u1 < self.len() && u2 < self.len() && u3 < self.len() {
            let i = u1 * self.len().pow(2) + u2 * self.len() + u3;
            self.trigrams[i]
        } else {
            0
        }
    }

    /// Returns the weighted frequency of a bigram for same-finger analysis by its internal byte representations.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::analyzer_data::AnalyzerData;
    ///
    /// let analyzer_data = AnalyzerData::default();
    /// assert_eq!(analyzer_data.get_same_finger_weighted_bigram_u([0, 1]), 0);
    /// ```
    #[inline]
    pub fn get_same_finger_weighted_bigram_u(&self, [c1, c2]: [u8; 2]) -> i64 {
        let u1 = c1 as usize;
        let u2 = c2 as usize;

        if u1 < self.len() && u2 < self.len() {
            let i = u1 * self.len() + u2;
            self.same_finger_weighted_bigrams[i]
        } else {
            0
        }
    }

    /// Returns the weighted frequency of a bigram for stretch analysis by its internal byte representations.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::analyzer_data::AnalyzerData;
    ///
    /// let analyzer_data = AnalyzerData::default();
    /// assert_eq!(analyzer_data.get_stretch_weighted_bigram_u([0, 1]), 0);
    /// ```
    #[inline]
    pub fn get_stretch_weighted_bigram_u(&self, [c1, c2]: [u8; 2]) -> i64 {
        let u1 = c1 as usize;
        let u2 = c2 as usize;

        if u1 < self.len() && u2 < self.len() {
            let i = u1 * self.len() + u2;
            self.stretch_weighted_bigrams[i]
        } else {
            0
        }
    }
}
