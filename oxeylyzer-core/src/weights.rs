use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Deserialize, Debug, Clone, Default)]
pub struct WeightDefaults {
    pub language: String,
    pub trigram_precision: usize,
    pub max_cores: usize,
}

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
    pub heatmap: f64,
    pub lateral_penalty: f64,
    pub fspeed: f64,
    pub dsfb_ratio: f64,
    #[serde(default)]
    pub dsfb_ratio2: f64,
    #[serde(default)]
    pub dsfb_ratio3: f64,
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
    pub heatmap: i64,
    pub lateral_penalty: i64,
    pub fspeed: i64,
    pub dsfb_ratio: i64,
    #[serde(default)]
    pub dsfb_ratio2: i64,
    #[serde(default)]
    pub dsfb_ratio3: i64,
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
    pub max_finger_use: AnalyzerMaxFingerUse,
}

impl From<Weights> for AnalyzerWeights {
    fn from(weights: Weights) -> Self {
        let scale = |float| (float * 100.0) as i64;

        let max_finger_use = AnalyzerMaxFingerUse {
            penalty: scale(weights.max_finger_use.penalty),
            pinky: scale(weights.max_finger_use.pinky),
            ring: scale(weights.max_finger_use.ring),
            middle: scale(weights.max_finger_use.middle),
            index: scale(weights.max_finger_use.index),
        };

        Self {
            heatmap: scale(weights.heatmap),
            lateral_penalty: scale(weights.lateral_penalty),
            fspeed: scale(weights.fspeed),
            dsfb_ratio: scale(weights.dsfb_ratio),
            dsfb_ratio2: scale(weights.dsfb_ratio2),
            dsfb_ratio3: scale(weights.dsfb_ratio3),
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
            max_finger_use,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    pub defaults: WeightDefaults,
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

        load.weights.max_finger_use = MaxFingerUse {
            penalty: load.weights.max_finger_use.penalty,
            pinky: load.weights.max_finger_use.pinky / 100.0,
            ring: load.weights.max_finger_use.ring / 100.0,
            middle: load.weights.max_finger_use.middle / 100.0,
            index: load.weights.max_finger_use.index / 100.0,
        };

        load.weights.dsfb_ratio2 = load.weights.dsfb_ratio.powi(2);
        load.weights.dsfb_ratio3 = load.weights.dsfb_ratio.powi(3);
        Self {
            defaults: WeightDefaults {
                language: load.defaults.language,
                trigram_precision: load.defaults.trigram_precision,
                max_cores: load.defaults.max_cores,
            },
            weights: load.weights,
        }
    }

    pub fn with_defaults() -> Self {
        Self {
            defaults: WeightDefaults {
                language: "english".to_string(),
                // keyboard_type: KeyboardType::AnsiAngle,
                trigram_precision: 100000,
                max_cores: 128,
            },
            weights: Weights {
                heatmap: 0.85,
                lateral_penalty: 1.3,
                fspeed: 8.0,
                dsfb_ratio: 0.12,
                dsfb_ratio2: (0.10 * 6.0f64).powi(2),
                dsfb_ratio3: (0.08 * 6.0f64).powi(3),
                scissors: 5.0,
                stretches: 3.0,
                lsbs: 2.0,
                pinky_ring_bigrams: 0.0,
                inrolls: 1.6,
                outrolls: 1.3,
                onehands: 0.8,
                alternates: 0.7,
                alternates_sfs: 0.35,
                redirects: 1.5,
                redirects_sfs: 2.75,
                bad_redirects: 4.0,
                bad_redirects_sfs: 6.0,
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
        self.defaults.trigram_precision
    }
}
