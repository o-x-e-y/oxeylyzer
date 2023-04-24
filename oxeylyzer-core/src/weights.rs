use crate::utility::KeyboardType;
use serde::Deserialize;
use std::fs::File;
use std::io::Read;

#[derive(Deserialize, Debug)]
pub struct WeightDefaultsLoad {
    pub language: String,
    pub keyboard_type: String,
    trigram_precision: usize,
}

#[derive(Deserialize, Debug)]
pub struct WeightDefaults {
    pub language: String,
    pub keyboard_type: KeyboardType,
    pub trigram_precision: usize,
}

#[derive(Deserialize, Clone, Debug)]
pub struct MaxFingerUse {
    pub penalty: f64,
    pub pinky: f64,
    pub ring: f64,
    pub middle: f64,
    pub index: f64,
}

#[derive(Deserialize, Clone, Debug)]
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
    pub lsbs: f64,
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

#[derive(Deserialize)]
struct ConfigLoad {
    pub pins: String,
    pub defaults: WeightDefaultsLoad,
    pub weights: Weights,
}

impl ConfigLoad {
    pub fn new() -> Self {
        let mut f = File::open("config.toml").expect("The config.toml is missing! Help!");

        let mut buf = Vec::new();
        f.read_to_end(&mut buf)
            .expect("Failed to read config.toml for some reason");

        let mut res: Self =
            toml::from_slice(&buf).expect("Failed to parse config.toml. Values might be missing.");
        res.pins = res.pins.trim().replace(' ', "").replace('\n', "");
        res
    }
}

pub struct Config {
    pub pins: Vec<usize>,
    pub defaults: WeightDefaults,
    pub weights: Weights,
}

impl Config {
    pub fn new() -> Self {
        let mut load = ConfigLoad::new();

        load.weights.max_finger_use = MaxFingerUse {
            penalty: load.weights.max_finger_use.penalty,
            pinky: load.weights.max_finger_use.pinky / 100.0,
            ring: load.weights.max_finger_use.ring / 100.0,
            middle: load.weights.max_finger_use.middle / 100.0,
            index: load.weights.max_finger_use.index / 100.0,
        };
        let mut pins = Vec::new();
        for (i, c) in load.pins.chars().enumerate() {
            if c == 'x' {
                pins.push(i);
            }
        }
        load.weights.dsfb_ratio2 = (load.weights.dsfb_ratio * 6.0).powi(3) / 6.5;
        load.weights.dsfb_ratio3 = (load.weights.dsfb_ratio * 6.0).powi(5) / 7.0;
        Self {
            pins,
            defaults: WeightDefaults {
                language: load.defaults.language,
                keyboard_type: KeyboardType::try_from(load.defaults.keyboard_type)
                    .unwrap_or(KeyboardType::AnsiAngle),
                trigram_precision: load.defaults.trigram_precision,
            },
            weights: load.weights,
        }
    }

    pub fn default() -> Self {
        Self {
            defaults: WeightDefaults {
                language: "english".to_string(),
                keyboard_type: KeyboardType::AnsiAngle,
                trigram_precision: 1000,
            },
            weights: Weights {
                heatmap: 0.85,
                lateral_penalty: 1.3,
                fspeed: 8.0,
                dsfb_ratio: 0.12,
                dsfb_ratio2: (0.10 * 6.0f64).powi(2),
                dsfb_ratio3: (0.08 * 6.0f64).powi(3),
                scissors: 5.0,
                lsbs: 2.0,
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
            pins: Vec::new(),
        }
    }

    pub fn trigram_precision(&self) -> usize {
        self.defaults.trigram_precision
    }
}
