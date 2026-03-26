use libdof::prelude::Finger;
use serde::{Deserialize, Serialize};
use serde_with::{OneOrMany, serde_as};
use std::path::{Path, PathBuf};

use crate::{OxeylyzerError, OxeylyzerResultExt, Result};

/// Configuration for penalizing excessive finger usage.
#[derive(Deserialize, Clone, Debug, Default)]
pub struct MaxFingerUse {
    /// The penalty multiplier applied when a finger exceeds its usage limit.
    pub penalty: f64,
    /// Maximum usage threshold for the pinky finger.
    pub pinky: f64,
    /// Maximum usage threshold for the ring finger.
    pub ring: f64,
    /// Maximum usage threshold for the middle finger.
    pub middle: f64,
    /// Maximum usage threshold for the index finger.
    pub index: f64,
    /// Maximum usage threshold for the thumb.
    pub thumb: f64,
}

#[derive(Deserialize, Clone, Debug, Default)]
/// Holds weights used for calculating various layout penalties and rewards.
///
/// # Examples:
/// ```
/// # use oxeylyzer_core::weights::Weights;
/// let weights = Weights::default();
/// ```
pub struct Weights {
    /// Penalty for lateral stretches.
    pub lateral_penalty: f64,
    /// Penalty for same-finger bigrams.
    pub sfbs: f64,
    /// Penalty for same-finger skips.
    pub sfs: f64,
    /// Penalty for finger stretches.
    pub stretches: f64,
    /// Penalty for pinky-ring bigrams.
    pub pinky_ring_bigrams: f64,
    /// Reward for inward rolls.
    pub inrolls: f64,
    /// Reward for outward rolls.
    pub outrolls: f64,
    /// Reward/penalty for one-handed trigrams.
    pub onehands: f64,
    /// Reward for alternating hands.
    pub alternates: f64,
    /// Reward for alternating hands with same-finger skip.
    pub alternates_sfs: f64,
    /// Penalty for hand redirects.
    pub redirects: f64,
    /// Penalty for redirects with same-finger skip.
    pub redirects_sfs: f64,
    /// Penalty for uncomfortable hand redirects.
    pub bad_redirects: f64,
    /// Penalty for uncomfortable redirects with same-finger skip.
    pub bad_redirects_sfs: f64,
    /// Maximum usage thresholds for fingers.
    pub max_finger_use: MaxFingerUse,
    /// Specific weights for each finger.
    pub finger_weights: FingerWeights,
}

#[derive(Deserialize, Clone, Debug, Default)]
/// Max finger usage thresholds scaled for internal calculations.
///
/// # Examples:
/// ```
/// # use oxeylyzer_core::weights::AnalyzerMaxFingerUse;
/// let analyzer = AnalyzerMaxFingerUse::default();
/// ```
pub struct AnalyzerMaxFingerUse {
    /// Scaled penalty multiplier.
    pub penalty: i64,
    /// Scaled max pinky usage.
    pub pinky: i64,
    /// Scaled max ring usage.
    pub ring: i64,
    /// Scaled max middle usage.
    pub middle: i64,
    /// Scaled max index usage.
    pub index: i64,
    /// Scaled max thumb usage.
    pub thumb: i64,
}

#[derive(Deserialize, Clone, Debug, Default)]
/// Analyzer weights scaled for internal integer calculations.
///
/// # Examples:
/// ```
/// # use oxeylyzer_core::weights::AnalyzerWeights;
/// let analyzer = AnalyzerWeights::default();
/// ```
pub struct AnalyzerWeights {
    /// Scaled lateral penalty.
    pub lateral_penalty: i64,
    /// Scaled SFB penalty.
    pub sfbs: i64,
    /// Scaled SFS penalty.
    pub sfs: i64,
    /// Scaled stretch penalty.
    pub stretches: i64,
    /// Scaled pinky-ring bigram penalty.
    pub pinky_ring_bigrams: i64,
    /// Scaled inroll reward.
    pub inrolls: i64,
    /// Scaled outroll reward.
    pub outrolls: i64,
    /// Scaled onehand penalty.
    pub onehands: i64,
    /// Scaled alternates reward.
    pub alternates: i64,
    /// Scaled alternates with SFS reward.
    pub alternates_sfs: i64,
    /// Scaled redirects penalty.
    pub redirects: i64,
    /// Scaled redirects with SFS penalty.
    pub redirects_sfs: i64,
    /// Scaled bad redirects penalty.
    pub bad_redirects: i64,
    /// Scaled bad redirects with SFS penalty.
    pub bad_redirects_sfs: i64,
    /// Specific weights for each finger.
    pub finger_weights: FingerWeights,
    /// Max finger usage thresholds.
    pub max_finger_use: AnalyzerMaxFingerUse,
}

impl From<Weights> for AnalyzerWeights {
    fn from(weights: Weights) -> Self {
        let scale = |float| (float * 100.0) as i64;

        let max_finger_use = AnalyzerMaxFingerUse {
            penalty: scale(weights.max_finger_use.penalty),
            pinky: weights.max_finger_use.pinky as i64,
            ring: weights.max_finger_use.ring as i64,
            middle: weights.max_finger_use.middle as i64,
            index: weights.max_finger_use.index as i64,
            thumb: weights.max_finger_use.thumb as i64,
        };

        Self {
            lateral_penalty: scale(weights.lateral_penalty),
            sfbs: scale(weights.sfbs),
            sfs: scale(weights.sfs),
            stretches: scale(weights.stretches),
            pinky_ring_bigrams: scale(weights.pinky_ring_bigrams),
            inrolls: scale(weights.inrolls),
            outrolls: scale(weights.outrolls),
            onehands: scale(weights.onehands),
            alternates: scale(weights.alternates),
            alternates_sfs: scale(weights.alternates_sfs),
            redirects: scale(weights.redirects),
            redirects_sfs: scale(weights.redirects_sfs),
            bad_redirects: scale(weights.bad_redirects),
            bad_redirects_sfs: scale(weights.bad_redirects_sfs),
            finger_weights: weights.finger_weights,
            max_finger_use,
        }
    }
}

#[serde_as]
#[derive(Debug, Clone, Default, Deserialize)]
/// Configuration for the layout generation.
///
/// # Examples:
/// ```
/// # use oxeylyzer_core::weights::Config;
/// let config = Config::default();
/// ```
pub struct Config {
    /// Path to the corpus file.
    pub corpus: PathBuf,
    #[serde_as(as = "OneOrMany<_>")]
    /// Paths to layout files.
    pub layouts: Vec<PathBuf>,
    /// Path to the corpus configurations directory.
    pub corpus_configs: PathBuf,
    /// Scaling factor for trigram precision.
    pub trigram_precision: usize,
    /// Max number of threads/cores to use.
    pub max_cores: usize,
    /// Configured weights for the generator.
    pub weights: Weights,
}

impl Config {
    /// Loads a configuration from a file path.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::weights::Config;
    /// let config = Config::with_loaded_weights("config.toml");
    /// ```
    pub fn with_loaded_weights<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(&path).path_context(&path)?;

        toml::from_str::<Self>(&content).path_context(path)
    }

    /// Creates a configuration with default values.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::weights::Config;
    /// let config = Config::with_defaults();
    /// ```
    pub fn with_defaults() -> Self {
        Self {
            corpus: PathBuf::from("./static/language_data/english.json"),
            layouts: vec![PathBuf::from("./static/layouts/english")],
            corpus_configs: PathBuf::from("./static/corpus_configs/**/"),
            trigram_precision: 100000,
            max_cores: 128,
            weights: Weights {
                lateral_penalty: 1.3,
                sfbs: -8.0,
                sfs: -1.0,
                stretches: -0.3,
                pinky_ring_bigrams: -0.0,
                inrolls: 1.6,
                outrolls: 1.3,
                onehands: 0.8,
                alternates: 0.7,
                alternates_sfs: 0.35,
                redirects: -1.5,
                redirects_sfs: -2.75,
                bad_redirects: -4.0,
                bad_redirects_sfs: -6.0,
                finger_weights: FingerWeights {
                    lp: 1.4,
                    lr: 3.6,
                    lm: 4.8,
                    li: 5.5,
                    lt: 3.3,
                    rt: 3.3,
                    ri: 5.5,
                    rm: 4.8,
                    rr: 3.6,
                    rp: 1.4,
                },
                max_finger_use: MaxFingerUse {
                    penalty: 2.5,
                    pinky: 9.0,
                    ring: 16.0,
                    middle: 19.5,
                    index: 18.0,
                    thumb: 22.0,
                },
            },
        }
    }

    /// Gets the trigram precision.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::weights::Config;
    /// let precision = Config::default().trigram_precision();
    /// ```
    pub fn trigram_precision(&self) -> usize {
        self.trigram_precision
    }

    /// Retrieves the name of the configured corpus.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::weights::Config;
    /// let config = Config::default();
    /// let corpus_name = config.corpus_name();
    /// ```
    pub fn corpus_name(&self) -> Result<String> {
        self.corpus
            .file_stem()
            .map(|o| o.display().to_string())
            .ok_or_else(|| OxeylyzerError::InvalidCorpusPath(self.corpus.clone()))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Relative weights applied to each finger.
///
/// # Examples:
/// ```
/// # use oxeylyzer_core::weights::FingerWeights;
/// let finger_weights = FingerWeights::default();
/// ```
pub struct FingerWeights {
    /// Left pinky weight.
    pub lp: f64,
    /// Left ring weight.
    pub lr: f64,
    /// Left middle weight.
    pub lm: f64,
    /// Left index weight.
    pub li: f64,
    /// Left thumb weight.
    pub lt: f64,
    /// Right thumb weight.
    pub rt: f64,
    /// Right index weight.
    pub ri: f64,
    /// Right middle weight.
    pub rm: f64,
    /// Right ring weight.
    pub rr: f64,
    /// Right pinky weight.
    pub rp: f64,
}

impl FingerWeights {
    #[inline]
    /// Gets the weight of a specific finger.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::weights::FingerWeights;
    /// # use libdof::prelude::Finger;
    /// let weights = FingerWeights::default();
    /// let val = weights.get(Finger::LI);
    /// ```
    pub const fn get(&self, f: Finger) -> f64 {
        use Finger::*;

        match f {
            LP => self.lp,
            LR => self.lr,
            LM => self.lm,
            LI => self.li,
            LT => self.lt,
            RT => self.rt,
            RI => self.ri,
            RM => self.rm,
            RR => self.rr,
            RP => self.rp,
        }
    }

    #[inline]
    /// Computes the maximum finger weight.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::weights::FingerWeights;
    /// let max_val = FingerWeights::default().max();
    /// ```
    pub fn max(&self) -> f64 {
        Finger::FINGERS
            .into_iter()
            .map(|f| self.get(f))
            .max_by(|a, b| a.total_cmp(b))
            .unwrap_or_default()
    }
}

impl Default for FingerWeights {
    fn default() -> Self {
        Self {
            lp: 1.0,
            lr: 1.0,
            lm: 1.0,
            li: 1.0,
            lt: 1.0,
            rt: 1.0,
            ri: 1.0,
            rm: 1.0,
            rr: 1.0,
            rp: 1.0,
        }
    }
}
