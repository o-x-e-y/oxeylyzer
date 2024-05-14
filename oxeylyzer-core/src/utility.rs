use std::collections::hash_map::Entry;

use crate::languages_cfg::read_cfg;

use arrayvec::ArrayVec;
use ahash::AHashMap as HashMap;
use nanorand::{tls_rng, Rng};
use serde::Deserialize;

#[inline]
pub fn shuffle_pins<const N: usize, T>(slice: &mut [T], pins: &[usize]) {
    let mapping: ArrayVec<_, N> = (0..slice.len()).filter(|x| !pins.contains(x)).collect();
    let mut rng = tls_rng();

    for (m, &swap1) in mapping.iter().enumerate() {
        let swap2 = rng.generate_range(m..mapping.len());
        slice.swap(swap1, mapping[swap2]);
    }
}

pub static I_TO_COL: [usize; 30] = [
    0, 1, 2, 3, 3, 4, 4, 5, 6, 7, 0, 1, 2, 3, 3, 4, 4, 5, 6, 7, 0, 1, 2, 3, 3, 4, 4, 5, 6, 7,
];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct PosPair(pub usize, pub usize);

const AFFECTS_SCISSOR: [bool; 30] = [
    true, true, true, true, true, true, true, true, true, true, true, true, false, false, false,
    false, false, false, true, true, true, true, true, false, true, false, false, true, true, true,
];

const AFFECTS_LSB: [bool; 30] = [
    false, false, true, false, true, true, false, true, false, false, false, false, true, false,
    true, true, false, true, false, false, false, false, true, false, true, true, false, true,
    false, false,
];

const AFFECTS_PINKY_RING: [bool; 30] = [
    true, true, false, false, false, false, false, false, true, true,
    true, true, false, false, false, false, false, false, true, true,
    true, true, false, false, false, false, false, false, true, true,
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
        unsafe { *AFFECTS_SCISSOR.get_unchecked(self.0) || *AFFECTS_SCISSOR.get_unchecked(self.1) }
    }

    #[inline]
    pub fn affects_lsb(&self) -> bool {
        unsafe { *AFFECTS_LSB.get_unchecked(self.0) || *AFFECTS_LSB.get_unchecked(self.1) }
    }

    #[inline]
    pub fn affects_pinky_ring(&self) -> bool {
        unsafe { *AFFECTS_PINKY_RING.get_unchecked(self.0) || *AFFECTS_PINKY_RING.get_unchecked(self.1) }
    }
}

impl std::fmt::Display for PosPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

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

#[derive(Clone, Debug, Default)]
pub struct ConvertU8 {
    from: Vec<char>,
    to: HashMap<char, u8>,
}

impl ConvertU8 {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_single(&self, c: u8) -> char {
        *self.from.get(c as usize).unwrap_or(&' ')
    }

    pub fn from<T>(&self, input: T) -> Vec<char>
    where
        T: IntoIterator<Item = u8>,
    {
        input.into_iter().map(|c| self.from_single(c)).collect()
    }

    pub fn to_single(&mut self, c: char) -> u8 {
        if let Some(u) = self.to.get(&c) {
            *u
        } else {
            let new = self.len();
            self.from.push(c);
            self.to.insert(c, new);
            new
        }
    }

    pub fn to_bigram(&mut self, from: [char; 2]) -> [u8; 2] {
        [self.to_single(from[0]), self.to_single(from[1])]
    }

    pub fn to_trigram(&mut self, from: [char; 3]) -> [u8; 3] {
        [
            self.to_single(from[0]),
            self.to_single(from[1]),
            self.to_single(from[2]),
        ]
    }

    pub fn to<T>(&mut self, input: T) -> Vec<u8>
    where
        T: IntoIterator<Item = char>,
    {
        input.into_iter().map(|c| self.to_single(c)).collect()
    }

    pub fn to_single_lossy(&self, c: char) -> u8 {
        if let Some(u) = self.to.get(&c) {
            *u
        } else {
            self.len()
        }
    }

    pub fn to_bigram_lossy(&self, from: [char; 2], char_count: usize) -> usize {
        let c1 = self.to_single_lossy(from[0]) as usize;
        let c2 = self.to_single_lossy(from[1]) as usize;
        if c1 < char_count && c2 < char_count {
            c1 * char_count + c2
        } else {
            u8::MAX as usize
        }
    }

    pub fn to_trigram_lossy(&self, from: [char; 3]) -> [u8; 3] {
        [
            self.to_single_lossy(from[0]),
            self.to_single_lossy(from[1]),
            self.to_single_lossy(from[2]),
        ]
    }

    pub fn to_lossy<T>(&self, input: T) -> Vec<u8>
    where
        T: IntoIterator<Item = char>,
    {
        input.into_iter().map(|c| self.to_single_lossy(c)).collect()
    }

    pub fn insert_single(&mut self, c: char) {
        let new = self.len();
        if let Entry::Vacant(e) = self.to.entry(c) {
            self.from.push(c);
            e.insert(new);
        }
    }

    pub fn insert<T>(&mut self, input: T)
    where
        T: IntoIterator<Item = char>,
    {
        input.into_iter().for_each(|c| self.insert_single(c));
    }

    pub fn with_chars(s: &str) -> Self {
        let mut res = Self::default();
        res.insert(s.chars());
        res
    }

    pub fn as_str(&self, input: &[u8]) -> String {
        input
            .iter()
            .map(|&c| self.from.get(c as usize).unwrap_or(&' '))
            .collect()
    }

    pub fn len(&self) -> u8 {
        debug_assert_eq!(self.to.len(), self.from.len());

        self.to.len() as u8
    }

    pub fn is_empty(&self) -> bool {
        self.to.len() == 0
    }
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

pub fn get_effort_map(heatmap_weight: f64, ktype: KeyboardType) -> [f64; 30] {
    use KeyboardType::*;

    let mut res = match ktype {
        IsoAngle => [
            3.0, 2.4, 2.0, 2.2, 2.4, 3.3, 2.2, 2.0, 2.4, 3.0, 1.8, 1.3, 1.1, 1.0, 2.6, 2.6, 1.0,
            1.1, 1.3, 1.8, 3.3, 2.8, 2.4, 1.8, 2.2, 2.2, 1.8, 2.4, 2.8, 3.3,
        ],
        AnsiAngle => [
            3.0, 2.4, 2.0, 2.2, 2.4, 3.3, 2.2, 2.0, 2.4, 3.0, 1.8, 1.3, 1.1, 1.0, 2.6, 2.6, 1.0,
            1.1, 1.3, 1.8, 3.7, 2.8, 2.4, 1.8, 2.2, 2.2, 1.8, 2.4, 2.8, 3.3,
        ],
        RowstagDefault => [
            3.0, 2.4, 2.0, 2.2, 2.4, 3.3, 2.2, 2.0, 2.4, 3.0, 1.8, 1.3, 1.1, 1.0, 2.6, 2.6, 1.0,
            1.1, 1.3, 1.8, 3.5, 3.0, 2.7, 2.3, 3.7, 2.2, 1.8, 2.4, 2.8, 3.3,
        ],
        Ortho => [
            3.0, 2.4, 2.0, 2.2, 3.1, 3.1, 2.2, 2.0, 2.4, 3.0, 1.7, 1.3, 1.1, 1.0, 2.6, 2.6, 1.0,
            1.1, 1.3, 1.7, 3.2, 2.6, 2.3, 1.6, 3.0, 3.0, 1.6, 2.3, 2.6, 3.2,
        ],
        Colstag => [
            3.0, 2.4, 2.0, 2.2, 3.1, 3.1, 2.2, 2.0, 2.4, 3.0, 1.7, 1.3, 1.1, 1.0, 2.6, 2.6, 1.0,
            1.1, 1.3, 1.7, 3.4, 2.6, 2.2, 1.8, 3.2, 3.2, 1.8, 2.2, 2.6, 3.4,
        ],
    };

    for r in &mut res {
        *r -= 0.2;
        *r /= 4.5;
        *r *= heatmap_weight;
    }

    res
}

pub fn get_fspeed(lat_multiplier: f64) -> [(PosPair, f64); 48] {
    let mut res = Vec::new();
    for (b, dist) in get_sfb_indices().iter().zip(get_distances(lat_multiplier)) {
        res.push((*b, dist));
    }
    res.try_into().unwrap()
}

pub fn get_distances(lat_multiplier: f64) -> [f64; 48] {
    let mut res = [0.0; 48];
    let mut i = 0;
    let help = |f: f64, r: f64| f.powi(2).powf(0.65) * r;

    let fweights = [1.4, 3.6, 4.8, 4.8, 3.6, 1.4];
    let mut fweight_i = 0;

    while fweight_i < 6 {
        let fweight = fweights[fweight_i];
        let ratio = 5.5 / fweight;

        res[i] = help(1.0, ratio);
        res[i + 1] = help(2.0, ratio);
        res[i + 2] = help(1.0, ratio);

        fweight_i += 1;
        i += 3;
    }

    let mut c = 0;
    while c <= 2 {
        let index = [
            ((0, 0), (0, 1)),
            ((0, 0), (0, 2)),
            ((0, 0), (1, 0)),
            ((0, 0), (1, 1)),
            ((0, 0), (1, 2)),
            ((0, 1), (0, 2)),
            ((0, 1), (1, 0)),
            ((0, 1), (1, 1)),
            ((0, 1), (1, 2)),
            ((0, 2), (1, 0)),
            ((0, 2), (1, 1)),
            ((0, 2), (1, 2)),
            ((1, 0), (1, 1)),
            ((1, 0), (1, 2)),
            ((1, 1), (1, 2)),
        ];
        let mut pair_i = 0;
        while pair_i < 15 {
            let ((x1, y1), (x2, y2)) = index[pair_i];

            let x_dist = (x1 - x2) as f64;
            let y_dist = (y1 - y2) as f64;
            let distance = (x_dist.powi(2) * lat_multiplier + y_dist.powi(2)).powf(0.65);
            res[i] = distance;

            i += 1;
            pair_i += 1;
        }
        c += 2;
    }
    res
}

pub const fn get_sfb_indices() -> [PosPair; 48] {
    let mut res = [PosPair::default(); 48];
    let mut i = 0;

    let mut col_i = 0;
    let cols = [0, 1, 2, 7, 8, 9];
    while col_i < cols.len() {
        let col = cols[col_i];
        res[i] = PosPair(col, col + 10);
        res[i + 1] = PosPair(col, col + 20);
        res[i + 2] = PosPair(col + 10, col + 20);

        col_i += 1;
        i += 3;
    }

    let mut c = 0;
    while c <= 2 {
        let index = [
            (3 + c, 13 + c),
            (3 + c, 23 + c),
            (3 + c, 4 + c),
            (3 + c, 14 + c),
            (3 + c, 24 + c),
            (13 + c, 23 + c),
            (13 + c, 4 + c),
            (13 + c, 14 + c),
            (13 + c, 24 + c),
            (23 + c, 4 + c),
            (23 + c, 14 + c),
            (23 + c, 24 + c),
            (4 + c, 14 + c),
            (4 + c, 24 + c),
            (14 + c, 24 + c),
        ];
        let mut pair_i = 0;
        while pair_i < 15 {
            res[i] = PosPair(index[pair_i].0, index[pair_i].1);
            i += 1;
            pair_i += 1;
        }
        c += 2;
    }
    res
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

pub(crate) fn is_kb_file(entry: &std::fs::DirEntry) -> bool {
    if let Some(ext_os) = entry.path().extension() {
        if let Some(ext) = ext_os.to_str() {
            return ext == "kb";
        }
    }
    false
}

pub(crate) fn layout_name(entry: &std::fs::DirEntry) -> Option<String> {
    if let Some(name_os) = entry.path().file_stem() {
        if let Some(name_str) = name_os.to_str() {
            return Some(name_str.to_string());
        }
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
