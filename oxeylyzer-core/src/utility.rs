use crate::weights::FingerWeights;

use libdof::prelude::{
    Finger::{self, *},
    PhysicalKey,
};
use nanorand::{Rng, tls_rng};

/// Randomly shuffles a slice while keeping elements at specified indices (pins) in their original positions.
///
/// # Examples:
/// ```
/// use oxeylyzer_core::utility::shuffle_pins;
///
/// let mut data = vec![1, 2, 3, 4, 5];
/// shuffle_pins(&mut data, &[0, 4]);
/// assert_eq!(data[0], 1);
/// assert_eq!(data[4], 5);
/// ```
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

/// Generates a default physical keyboard map with a 3x10 grid of keys.
///
/// # Examples:
/// ```
/// use oxeylyzer_core::utility::default_physical_map;
///
/// let map = default_physical_map();
/// assert_eq!(map.len(), 30);
/// ```
pub fn default_physical_map() -> Box<[PhysicalKey]> {
    let mut res = Vec::new();

    for y in 0..3 {
        for x in 0..10 {
            res.push(PhysicalKey::xy(x as f64, y as f64))
        }
    }

    res.into()
}

/// Default mapping of keys to fingers for a standard 30-key layout.
#[rustfmt::skip]
pub static DEFAULT_FINGERMAP: [Finger; 30] = [
    LP, LR, LM, LI, LI,  RI, RI, RM, RR, RP,
    LP, LR, LM, LI, LI,  RI, RI, RM, RR, RP,
    LP, LR, LM, LI, LI,  RI, RI, RM, RR, RP,
];

/// Default effort weights for each finger.
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

/// Trait for approximate equality comparison of floating point numbers.
pub trait ApproxEq {
    /// Returns true if two numbers are equal up to a certain number of decimal places.
    ///
    /// # Examples:
    /// ```
    /// use oxeylyzer_core::utility::ApproxEq;
    ///
    /// assert!((0.1234).approx_eq(0.123, 3));
    /// ```
    fn approx_eq(self, other: f64, dec: u8) -> bool;

    /// Same as `approx_eq`, but prints a message to stdout if the numbers are not equal.
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
