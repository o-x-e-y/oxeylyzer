use crate::{languages_cfg::read_cfg, weights::FingerWeights};

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

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PosPair(pub usize, pub usize);

impl From<(usize, usize)> for PosPair {
    fn from((p1, p2): (usize, usize)) -> Self {
        Self(p1, p2)
    }
}

#[rustfmt::skip]
const AFFECTS_SCISSOR: [bool; 30] = [
    true,   true,   true,   true,   true,      true,   true,   true,   true,   true,
    true,   true,   false,  false,  false,     false,  false,  false,  true,   true,
    true,   true,   true,   false,  true,      false,  false,  true,   true,   true,
];

#[rustfmt::skip]
const AFFECTS_LSB: [bool; 30] = [
    false,  false,  true,   false,  true,      true,   false,  true,   false,  false,
    false,  false,  true,   false,  true,      true,   false,  true,   false,  false,
    false,  false,  true,   false,  true,      true,   false,  true,   false,  false,
];

#[rustfmt::skip]
const AFFECTS_PINKY_RING: [bool; 30] = [
    true,   true,   false,  false,  false,     false,  false,  false,  true,   true,
    true,   true,   false,  false,  false,     false,  false,  false,  true,   true,
    true,   true,   false,  false,  false,     false,  false,  false,  true,   true,
];

impl PosPair {
    pub const fn default() -> Self {
        Self(0, 0)
    }

    pub const fn new(x1: usize, x2: usize) -> Self {
        Self(x1, x2)
    }

    #[inline]
    pub fn affects_scissor(&self) -> bool {
        *AFFECTS_SCISSOR.get(self.0).unwrap() || *AFFECTS_SCISSOR.get(self.1).unwrap()
    }

    #[inline]
    pub fn affects_lsb(&self) -> bool {
        *AFFECTS_LSB.get(self.0).unwrap() || *AFFECTS_LSB.get(self.1).unwrap()
    }

    #[inline]
    pub fn affects_pinky_ring(&self) -> bool {
        *AFFECTS_PINKY_RING.get(self.0).unwrap() || *AFFECTS_PINKY_RING.get(self.1).unwrap()
    }
}

impl std::fmt::Display for PosPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

// TODO: create this on a by-layout basis
pub const POSSIBLE_SWAPS: [PosPair; 435] = get_possible_swaps();

const fn get_possible_swaps() -> [PosPair; 435] {
    let mut res = [PosPair::default(); 435];
    let mut i = 0;
    let mut pos1 = 0;

    while pos1 < 30 {
        let mut pos2 = pos1 + 1;
        while pos2 < 30 {
            res[i] = PosPair(pos1, pos2);
            i += 1;
            pos2 += 1;
        }
        pos1 += 1;
    }
    res
}

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

pub const fn get_lsb_indices() -> [PosPair; 16] {
    let mut res = [PosPair::default(); 16];
    let left = [
        (2, 4),
        (2, 14),
        (2, 24),
        (12, 4),
        (12, 14),
        (22, 4),
        (22, 14),
        (22, 24),
    ];
    let right = [
        (5, 7),
        (5, 17),
        (5, 27),
        (15, 7),
        (15, 17),
        (15, 27),
        (25, 17),
        (25, 27),
    ];

    let mut i = 0;
    while i < left.len() {
        res[i] = PosPair(left[i].0, left[i].1);
        res[i + 8] = PosPair(right[i].0, right[i].1);
        i += 1;
    }
    res
}

pub const fn get_pinky_ring_indices() -> [PosPair; 18] {
    [
        PosPair(0, 1),
        PosPair(0, 11),
        PosPair(0, 21),
        PosPair(11, 1),
        PosPair(11, 11),
        PosPair(11, 21),
        PosPair(21, 1),
        PosPair(21, 11),
        PosPair(21, 21),
        PosPair(8, 9),
        PosPair(8, 19),
        PosPair(8, 29),
        PosPair(18, 9),
        PosPair(18, 19),
        PosPair(18, 29),
        PosPair(28, 9),
        PosPair(28, 19),
        PosPair(28, 29),
    ]
}

pub const fn get_scissor_indices() -> [PosPair; 17] {
    let mut res = [PosPair::default(); 17];

    //these add normal stretching between ajacent columns that stretch between 2 rows except for
    //qwerty mi and ce (assuming c is typed with index)
    // let mut i = 0;
    // let cols = [0, 1, 2, 6, 7, 8];

    // while i < cols.len() {
    // 	let col_nr = cols[i];
    // 	if col_nr != 2 {
    // 		res[i] = PosPair(col_nr, col_nr+21);
    // 	}
    // 	if col_nr != 6 {
    // 		res[i+6] = PosPair(col_nr+1, col_nr+20);
    // 	}
    // 	i += 1;
    // }

    res[0] = PosPair(0, 21);
    res[1] = PosPair(1, 22);
    res[2] = PosPair(6, 27);
    res[3] = PosPair(7, 28);
    res[4] = PosPair(8, 29);
    res[5] = PosPair(1, 20);
    res[6] = PosPair(2, 21);
    res[7] = PosPair(3, 22);
    res[8] = PosPair(8, 27);
    res[9] = PosPair(9, 28);

    //pinky->ring 1u stretches
    res[10] = PosPair(0, 11);
    res[11] = PosPair(9, 18);
    res[12] = PosPair(10, 21);
    res[13] = PosPair(19, 28);

    //inner index scissors (no qwerty `ni` because of stagger)
    res[14] = PosPair(2, 24);
    res[15] = PosPair(22, 4);
    res[16] = PosPair(5, 27);

    res
}

pub fn chars_for_generation(language: &str) -> [char; 30] {
    let languages_cfg_map = read_cfg();

    if let Some(cfg) = languages_cfg_map.get(language) {
        cfg.chars().collect::<Vec<char>>().try_into().unwrap()
    } else {
        let default = languages_cfg_map.get(&String::from("default")).unwrap();
        default.chars().collect::<Vec<char>>().try_into().unwrap()
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

pub(crate) fn is_kb_file(path: &&std::path::PathBuf) -> bool {
    if let Some(ext) = path.extension() {
        return ext == "kb";
    }
    false
}

pub(crate) fn is_dof_file(entry: &&std::path::PathBuf) -> bool {
    if let Some(ext_os) = entry.extension() {
        return ext_os == "dof";
    }
    false
}

pub(crate) fn layout_name(entry: &std::path::Path) -> Option<String> {
    if let Some(name_os) = entry.file_stem()
        && let Some(name_str) = name_os.to_str()
    {
        return Some(name_str.to_string());
    }
    None
}

pub(crate) fn format_layout_str(layout_str: &str) -> String {
    layout_str
        .split('\n')
        .take(3)
        .map(|line| line.split_whitespace().take(10).collect::<String>())
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ahash::AHashSet as HashSet;

    #[test]
    fn affects_scissors() {
        let indices = get_scissor_indices()
            .into_iter()
            .flat_map(|PosPair(i1, i2)| [i1, i2])
            .collect::<HashSet<_>>();

        (0..30).for_each(|i| {
            if indices.contains(&i) {
                assert!(AFFECTS_SCISSOR[i], "failed on {i}");
            } else {
                assert!(!AFFECTS_SCISSOR[i], "failed on {i}");
            }
        });
    }

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

    #[test]
    fn format_layout_string() {
        let str1 = "v m l c p  q z u o , \ns t r d y  f n e a i \nx k j g w  b h ; ' .";
        let str2 = "a b    c d e f g h i \n j k l \n m n o p q \n r s t u v w x y z";

        assert_eq!(format_layout_str(str1), "vmlcpqzuo,strdyfneaixkjgwbh;'.");
        assert_eq!(format_layout_str(str2), "abcdefghijklmnopq");
    }
}
