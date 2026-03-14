use crate::weights::FingerWeights;

use libdof::prelude::{
    Finger::{self, *},
    PhysicalKey,
};
use nanorand::{Rng, tls_rng};
use serde::Deserialize;

#[inline]
pub fn shuffle_pins<T>(slice: &mut [T], pins: &[usize]) {
    let mapping = (0..slice.len())
        .filter(|x| !pins.contains(x))
        .collect::<Vec<_>>();

    let mut rng = tls_rng();

    for (m, &swap1) in mapping.iter().enumerate() {
        let swap2 = rng.generate_range(m..mapping.len());
        slice.swap(swap1, mapping[swap2]);
    }
}

pub fn default_physical_map() -> Box<[PhysicalKey]> {
    let mut res = Vec::new();

    for y in 0..3 {
        for x in 0..10 {
            res.push(PhysicalKey::xy(x as f64, y as f64))
        }
    }

    res.into()
}

#[rustfmt::skip]
pub static DEFAULT_FINGERMAP: [Finger; 30] = [
    LP, LR, LM, LI, LI,  RI, RI, RM, RR, RP,
    LP, LR, LM, LI, LI,  RI, RI, RM, RR, RP,
    LP, LR, LM, LI, LI,  RI, RI, RM, RR, RP,
];

pub static DEFAULT_FINGER_WEIGHTS: FingerWeights = FingerWeights {
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
};

// TODO: remove
#[derive(Deserialize, Debug, Clone, Default)]
pub enum KeyboardType {
    #[default]
    AnsiAngle,
    IsoAngle,
    RowstagDefault,
    Ortho,
    Colstag,
}

impl TryFrom<String> for KeyboardType {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, &'static str> {
        let lower = value.to_lowercase();
        let split = lower.split_whitespace().collect::<Vec<&str>>();

        if split.len() == 1 {
            match split[0] {
                "ortho" => Ok(Self::Ortho),
                "colstag" => Ok(Self::Colstag),
                "rowstag" | "iso" | "ansi" | "jis" => Ok(Self::RowstagDefault),
                _ => Err("Couldn't parse keyboard type!"),
            }
        } else if split.len() == 2 {
            match (split[0], split[1]) {
                ("ansi", "angle") => Ok(Self::AnsiAngle),
                ("iso", "angle") => Ok(Self::IsoAngle),
                _ => Err("Couldn't parse keyboard type!"),
            }
        } else {
            Err("Couldn't parse keyboard type!")
        }
    }
}

pub trait ApproxEq {
    fn approx_eq(self, other: f64, dec: u8) -> bool;

    fn approx_eq_dbg(self, other: f64, dec: u8) -> bool;
}

impl ApproxEq for f64 {
    fn approx_eq(self, other: f64, dec: u8) -> bool {
        let factor = 10.0f64.powi(dec as i32);
        let a = (self * factor).trunc();
        let b = (other * factor).trunc();
        a == b
    }

    fn approx_eq_dbg(self, other: f64, dec: u8) -> bool {
        let factor = 10.0f64.powi(dec as i32);
        let a = (self * factor).trunc();
        let b = (other * factor).trunc();

        if a != b {
            println!("approx not equal: {self} != {other}");
        }
        a == b
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn approx_eq() {
        assert!((0.123456789).approx_eq(0.0, 0));
        assert!((0.123456789).approx_eq(0.1, 1));
        assert!((0.123456789).approx_eq(0.12, 2));
        assert!((0.123456789).approx_eq(0.123, 3));
        assert!((0.123456789).approx_eq(0.1234, 4));
        assert!((0.123456789).approx_eq(0.12345, 5));
        assert!((0.123456789).approx_eq(0.123456, 6));
        assert!((0.123456789).approx_eq(0.1234567, 7));
        assert!((0.123456789).approx_eq(0.12345678, 8));
        assert!((0.123456789).approx_eq(0.123456789, 9));

        assert!(!(0.123456789).approx_eq(0.0, 3));
        assert!(!(0.123456789).approx_eq(0.1, 4));

        assert!((0.123456789).approx_eq_dbg(0.0, 0));
        assert!((0.123456789).approx_eq_dbg(0.1, 1));
        assert!((0.123456789).approx_eq_dbg(0.12, 2));
        assert!((0.123456789).approx_eq_dbg(0.123, 3));
        assert!((0.123456789).approx_eq_dbg(0.1234, 4));
        assert!((0.123456789).approx_eq_dbg(0.12345, 5));
        assert!((0.123456789).approx_eq_dbg(0.123456, 6));
        assert!((0.123456789).approx_eq_dbg(0.1234567, 7));
        assert!((0.123456789).approx_eq_dbg(0.12345678, 8));
        assert!((0.123456789).approx_eq_dbg(0.123456789, 9));

        assert!(!(0.123456789).approx_eq_dbg(0.0, 3));
        assert!(!(0.123456789).approx_eq_dbg(0.1, 4));
    }
}
