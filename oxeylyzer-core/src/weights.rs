use libdof::prelude::Finger;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Deserialize, Clone, Debug, Default)]
pub struct MaxFingerUse {
    pub penalty: f64,
    pub pinky: f64,
    pub ring: f64,
    pub middle: f64,
    pub index: f64,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Weights {
    pub lateral_penalty: f64,
    pub sfbs: f64,
    pub sfs: f64,
    pub scissors: f64,
    pub stretches: f64,
    pub lsbs: f64,
    pub pinky_ring_bigrams: f64,
    pub inrolls: f64,
    pub outrolls: f64,
    pub onehands: f64,
    pub alternates: f64,
    pub alternates_sfs: f64,
    pub redirects: f64,
    pub redirects_sfs: f64,
    pub bad_redirects: f64,
    pub bad_redirects_sfs: f64,
    pub max_finger_use: MaxFingerUse,
    pub finger_weights: FingerWeights,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct AnalyzerMaxFingerUse {
    pub penalty: i64,
    pub pinky: i64,
    pub ring: i64,
    pub middle: i64,
    pub index: i64,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct AnalyzerWeights {
    pub lateral_penalty: i64,
    pub sfbs: i64,
    pub sfs: i64,
    pub scissors: i64,
    pub stretches: i64,
    pub lsbs: i64,
    pub pinky_ring_bigrams: i64,
    pub inrolls: i64,
    pub outrolls: i64,
    pub onehands: i64,
    pub alternates: i64,
    pub alternates_sfs: i64,
    pub redirects: i64,
    pub redirects_sfs: i64,
    pub bad_redirects: i64,
    pub bad_redirects_sfs: i64,
    pub finger_weights: FingerWeights,
    pub max_finger_use: AnalyzerMaxFingerUse,
}

impl From<Weights> for AnalyzerWeights {
    fn from(weights: Weights) -> Self {
        let scale = |float| (float * 100.0) as i64;

        let max_finger_use = AnalyzerMaxFingerUse {
            // TODO: this is probably wrong
            penalty: scale(weights.max_finger_use.penalty),
            pinky: scale(weights.max_finger_use.pinky),
            ring: scale(weights.max_finger_use.ring),
            middle: scale(weights.max_finger_use.middle),
            index: scale(weights.max_finger_use.index),
        };

        Self {
            lateral_penalty: scale(weights.lateral_penalty),
            sfbs: scale(weights.sfbs),
            sfs: scale(weights.sfs),
            scissors: scale(weights.scissors),
            stretches: scale(weights.stretches),
            lsbs: scale(weights.lsbs),
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

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    pub language: String,
    pub trigram_precision: usize,
    pub max_cores: usize,
    pub weights: Weights,
}

impl Config {
    pub fn with_loaded_weights<P: AsRef<Path>>(path: P) -> Self {
        let mut f = File::open(path).expect("The config.toml is missing! Help!");

        let mut buf = String::new();
        f.read_to_string(&mut buf)
            .expect("Failed to read config.toml for some reason");

        let mut load = toml::from_str::<Self>(&buf)
            .expect("Failed to parse config.toml. Values might be missing.");

        // TODO: figure out how this should even work
        load.weights.max_finger_use = MaxFingerUse {
            penalty: load.weights.max_finger_use.penalty,
            pinky: load.weights.max_finger_use.pinky / 100.0,
            ring: load.weights.max_finger_use.ring / 100.0,
            middle: load.weights.max_finger_use.middle / 100.0,
            index: load.weights.max_finger_use.index / 100.0,
        };

        Self {
            language: load.language,
            trigram_precision: load.trigram_precision,
            max_cores: load.max_cores,
            weights: load.weights,
        }
    }

    pub fn with_defaults() -> Self {
        Self {
            language: "english".to_string(),
            trigram_precision: 100000,
            max_cores: 128,
            weights: Weights {
                lateral_penalty: 1.3,
                sfbs: -8.0,
                sfs: -1.0,
                scissors: -5.0,
                stretches: -0.3,
                lsbs: -2.0,
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
                finger_weights: FingerWeights::default(),
                max_finger_use: MaxFingerUse {
                    penalty: 2.5,
                    pinky: 9.0,
                    ring: 16.0,
                    middle: 19.5,
                    index: 18.0,
                },
            },
        }
    }

    pub fn trigram_precision(&self) -> usize {
        self.trigram_precision
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FingerWeights {
    pub lp: f64,
    pub lr: f64,
    pub lm: f64,
    pub li: f64,
    pub lt: f64,
    pub rt: f64,
    pub ri: f64,
    pub rm: f64,
    pub rr: f64,
    pub rp: f64,
}

impl FingerWeights {
    #[inline]
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
