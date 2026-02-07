use ahash::AHashMap as HashMap;
use anyhow::Result;
use itertools::Itertools;
use libdof::prelude::{Dof, Finger, Keyboard, PhysicalKey, Shape};

use crate::{char_mapping::CharMapping, utility::*, *};

const KEY_EDGE_OFFSET: f64 = 0.5;

pub trait Layout<T: Copy + Default> {
    fn new() -> Self;

    fn random(available_chars: &mut [u8]) -> Self;

    fn random_pins(layout_chars: &mut [u8], pins: &[usize]) -> Self;

    fn char(&self, i: usize) -> Option<T>;

    fn swap(&mut self, i1: usize, i2: usize) -> Option<()>;

    fn swap_pair(&mut self, pair: &PosPair) -> Option<()>;

    fn swap_cols(&mut self, col1: usize, col2: usize) -> Option<()>;

    fn swap_indexes(&mut self);

    fn get_index(&self, index: usize) -> [T; 6];
}

#[derive(Debug, Clone, PartialEq)]
pub struct FastLayout {
    pub matrix: Box<[u8]>,
    pub char_to_finger: Box<[Option<Finger>]>,
    pub matrix_fingers: Box<[Finger]>,
    pub matrix_physical: Box<[PhysicalKey]>,
    pub fspeed_indices: FSpeedIndices,
    pub stretch_indices: StretchCache,
    pub shape: Shape,
    pub score: f64,
}

impl Default for FastLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl From<[u8; 30]> for FastLayout {
    fn from(layout: [u8; 30]) -> Self {
        let mut new_layout = FastLayout::new();

        for (i, byte) in layout.into_iter().enumerate() {
            new_layout.matrix[i] = byte;
            new_layout.char_to_finger[byte as usize] = Some(new_layout.matrix_fingers[i]);
        }
        new_layout
    }
}

impl TryFrom<&[u8]> for FastLayout {
    type Error = anyhow::Error;

    fn try_from(layout_bytes: &[u8]) -> Result<Self, Self::Error> {
        if layout_bytes.len() >= 30 {
            let mut new_layout = FastLayout::new();

            for (i, &byte) in layout_bytes.iter().enumerate().take(30) {
                new_layout.matrix[i] = byte;
                new_layout.char_to_finger[byte as usize] = Some(new_layout.matrix_fingers[i]);
            }
            Ok(new_layout)
        } else {
            anyhow::bail!("you should provide at least 30 bytes to create a layout from.")
        }
    }
}

impl FastLayout {
    pub fn layout_str(&self, con: &CharMapping) -> String {
        con.as_str(&self.matrix)
    }

    pub fn formatted_string(&self, con: &CharMapping) -> String {
        let mut res = String::new();

        let mut iter = self.matrix.iter();

        for &l in self.shape.inner().iter() {
            let mut i = 0;
            for u in iter.by_ref() {
                let c = con.from_single(*u);
                res.push_str(&format!("{c} "));

                i += 1;

                if l == i {
                    break;
                } else if i == 5 {
                    res.push(' ');
                }
            }
            res.push('\n');
        }

        res
    }

    pub fn from_dof(dof: Dof, convert: &mut CharMapping) -> Result<Self> {
        use libdof::prelude::{Key, SpecialKey};

        // let key_count = dof.main_layer().shape().inner().iter().sum::<usize>();
        // if key_count != 30 {
        //     bail!("Invalid key count {key_count}, expected 30")
        // }

        let matrix = dof
            .main_layer()
            .keys()
            .map(|k| match k {
                Key::Char(c) => *c,
                Key::Special(s) => match s {
                    SpecialKey::Repeat => REPEAT_KEY,
                    SpecialKey::Space => SPACE_CHAR,
                    SpecialKey::Shift => SHIFT_CHAR,
                    _ => REPLACEMENT_CHAR,
                },
                _ => REPLACEMENT_CHAR,
            })
            .map(|c| convert.to_single(c))
            .collect::<Box<_>>();

        let matrix_fingers = dof.fingering().keys().copied().collect::<Box<_>>();
        let matrix_physical = dof.board().keys().cloned().collect::<Box<_>>();

        let mut char_to_finger = Box::new([None; 60]);
        matrix
            .iter()
            .enumerate()
            .for_each(|(i, &c)| char_to_finger[c as usize] = Some(matrix_fingers[i]));

        let fspeed_indices = FSpeedIndices::new(&matrix_fingers, &matrix_physical);
        let stretch_indices = StretchCache::new(&matrix, &matrix_fingers, &matrix_physical);
        let shape = dof.shape();

        // let name = dof.name().to_owned();
        // let keyboard = dof.board().keys().cloned().map(Into::into).collect();
        // let shape = dof.main_layer().shape();

        let layout = Self {
            matrix,
            matrix_fingers,
            matrix_physical,
            char_to_finger,
            fspeed_indices,
            stretch_indices,
            shape,
            score: 0.0,
        };

        Ok(layout)
    }
}

impl Layout<u8> for FastLayout {
    fn new() -> FastLayout {
        let matrix = Box::new([u8::MAX; 30]);
        let matrix_fingers = Box::new(DEFAULT_FINGERMAP);
        let matrix_physical = default_physical_map();
        let char_to_finger = Box::new([None; 64]);
        let fspeed_indices = FSpeedIndices::new(matrix_fingers.as_slice(), &matrix_physical);
        let stretch_indices = StretchCache::new(
            matrix.as_slice(),
            matrix_fingers.as_slice(),
            &matrix_physical,
        );
        let shape = Shape::from(vec![10, 10, 10]);
        let score = 0.0;

        FastLayout {
            matrix,
            matrix_fingers,
            matrix_physical,
            char_to_finger,
            fspeed_indices,
            stretch_indices,
            shape,
            score,
        }
    }

    fn random(layout_chars: &mut [u8]) -> FastLayout {
        shuffle_pins::<30, u8>(layout_chars, &[]);
        let non_mut: &[u8] = layout_chars;
        FastLayout::try_from(non_mut).unwrap()
    }

    fn random_pins(layout_chars: &mut [u8], pins: &[usize]) -> FastLayout {
        shuffle_pins::<30, u8>(layout_chars, pins);
        let non_mut: &[u8] = layout_chars;
        FastLayout::try_from(non_mut).unwrap()
    }

    #[inline(always)]
    fn char(&self, i: usize) -> Option<u8> {
        self.matrix.get(i).copied()
    }

    fn swap_cols(&mut self, col1: usize, col2: usize) -> Option<()> {
        if col1 == col2 {
            return Some(());
        }
        if col1 > 9 || col2 > 9 {
            return None;
        }
        // TODO: handle errors properly here
        self.swap(col1, col2).unwrap();
        self.swap(col1 + 10, col2 + 10).unwrap();
        self.swap(col1 + 20, col2 + 20).unwrap();

        Some(())
    }

    fn swap_indexes(&mut self) {
        self.swap_cols(3, 6);
        self.swap_cols(4, 5);
    }

    /// Gets all keys in a certain index column. 0 = left index, 1 = right index.
    fn get_index(&self, index: usize) -> [u8; 6] {
        let mut new_index = [0; 6];
        let start_pos = index * 2 + 3;
        for i in 0..2 {
            for j in 0..3 {
                new_index[2 * j + i] = self.matrix[start_pos + i + 10 * j];
            }
        }
        new_index
    }

    #[inline(always)]
    fn swap(&mut self, i1: usize, i2: usize) -> Option<()> {
        let char1 = self.char(i1)?;
        let char2 = self.char(i2)?;

        *self.matrix.get_mut(i1)? = char2;
        *self.matrix.get_mut(i2)? = char1;

        *self.char_to_finger.get_mut(char1 as usize)? = Some(*self.matrix_fingers.get(i2)?);
        *self.char_to_finger.get_mut(char2 as usize)? = Some(*self.matrix_fingers.get(i1)?);

        Some(())
    }

    #[inline(always)]
    fn swap_pair(&mut self, pair: &PosPair) -> Option<()> {
        self.swap(pair.0, pair.1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BigramPair {
    pub pair: PosPair,
    pub dist: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FSpeedIndices {
    pub fingers: Box<[Box<[BigramPair]>; 10]>,
    pub all: Box<[BigramPair]>,
}

impl FSpeedIndices {
    pub fn get_finger(&self, finger: Finger) -> &[BigramPair] {
        &self.fingers[finger as usize]
    }

    pub fn new(
        fingers: &[Finger],
        keyboard: &[PhysicalKey],
        // finger_weights: &FingerWeights,
    ) -> Self {
        assert!(
            fingers.len() <= u8::MAX as usize,
            "Too many keys to index with u8, max is {}",
            u8::MAX
        );
        assert_eq!(
            fingers.len(),
            keyboard.len(),
            "finger len is not the same as keyboard len: "
        );

        // TODO: move this to weights
        static F_WEIGHTS: [f64; 10] = [1.4, 3.6, 4.8, 5.5, 3.3, 3.3, 5.5, 4.8, 3.6, 1.4];

        let fingers: Box<[_; 10]> = Finger::FINGERS
            .map(|finger| {
                fingers
                    .iter()
                    .zip(keyboard)
                    .zip(0usize..)
                    .filter_map(|((f, k), i)| (f == &finger).then_some((*f, k, i)))
                    .tuple_combinations::<(_, _)>()
                    .map(|((f1, k1, i1), (_, k2, i2))| BigramPair {
                        pair: PosPair(i1, i2),
                        // TODO: think about scaling differently
                        dist: dist(k1, k2, finger, finger).powf(1.3)
                            * (5.5 / F_WEIGHTS[f1 as usize]),
                        // * finger_weights.get(finger),
                    })
                    .collect::<Box<_>>()
            })
            .into();

        let all = fingers
            .iter()
            .flat_map(|f| f.iter())
            .cloned()
            .collect::<Box<_>>();

        Self { fingers, all }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct StretchCache {
    pub all_pairs: Box<[BigramPair]>,
    pub per_key_pair: HashMap<PosPair, Box<[BigramPair]>>,
}

impl StretchCache {
    pub fn new(keys: &[u8], fingers: &[Finger], keyboard: &[PhysicalKey]) -> Self {
        assert!(
            fingers.len() <= u8::MAX as usize,
            "Too many keys to index with u8, max is {}",
            u8::MAX
        );
        assert_eq!(
            fingers.len(),
            keyboard.len(),
            "finger len is not the same as keyboard len: "
        );

        let all_pairs = keyboard
            .iter()
            .zip(fingers)
            .zip(keys)
            .enumerate()
            .tuple_combinations::<(_, _)>()
            .filter(|((_, ((_, f1), _)), (_, ((_, f2), _)))| f1 != f2 && (f1.hand() == f2.hand()))
            .filter_map(|((i1, ((k1, &f1), _c1)), (i2, ((k2, &f2), _c2)))| {
                let diff = (f1 as u8).abs_diff(f2 as u8) as f64;
                let fd = diff * 1.35;
                // let minimum_diff = diff * 0.9;
                let (dx, dy) = dx_dy(k1, k2, f1, f2);
                let negative_lsb = 0.0; //(minimum_diff - dx.abs() - 1.0).max(0.0) * 2.0;
                let dist = dx.hypot(dy);

                let xo = x_overlap(dx, dy, f1, f2);

                let stretch = dist + xo + negative_lsb - fd;

                // if stretch > 0.001 {
                //     println!("{_c1}{_c2}: {}", (stretch * 100.0) as i64);
                // }

                (stretch > 0.001).then_some(BigramPair {
                    pair: PosPair(i1, i2),
                    dist: stretch,
                })
            })
            .collect::<Box<[_]>>();

        // println!("pair count: {}", all_pairs.len());

        let per_keypair = (0..(fingers.len()))
            .cartesian_product(0..(fingers.len()))
            .map(|(i1, i2)| {
                let is = [i1, i2];

                let pairs = all_pairs
                    .iter()
                    .filter(move |b| is.contains(&b.pair.0) || is.contains(&b.pair.1))
                    .copied()
                    .collect::<Box<[_]>>();

                (PosPair(i1, i2), pairs)
            })
            .collect::<HashMap<_, _>>();

        Self {
            all_pairs,
            per_key_pair: per_keypair,
        }
    }
}

fn x_finger_overlap(f1: Finger, f2: Finger) -> f64 {
    use Finger::*;

    match (f1, f2) {
        (LP, LR) => 0.8,
        (LR, LP) => 0.8,
        (LR, LM) => 0.4,
        (LM, LR) => 0.4,
        (LM, LI) => 0.1,
        (LI, LM) => 0.1,
        (LI, LT) => -2.5,
        (LT, LI) => -2.5,
        (RT, RI) => -2.5,
        (RI, RT) => -2.5,
        (RI, RM) => 0.1,
        (RM, RI) => 0.1,
        (RM, RR) => 0.4,
        (RR, RM) => 0.4,
        (RR, RP) => 0.8,
        (RP, RR) => 0.8,
        _ => 0.0,
    }
}

fn x_overlap(dx: f64, dy: f64, f1: Finger, f2: Finger) -> f64 {
    let x_offset = x_finger_overlap(f1, f2);

    let dx_offset = x_offset - dx * 1.3;
    let dy_offset = 0.3333 * dy;

    (dx_offset + dy_offset).max(0.0)
}

fn dx_dy(k1: &PhysicalKey, k2: &PhysicalKey, f1: Finger, f2: Finger) -> (f64, f64) {
    let f_len = |f: Finger| match f {
        Finger::LP | Finger::RP => -0.15,
        Finger::LR | Finger::RR => 0.35,
        Finger::LM | Finger::RM => 0.25,
        Finger::LI | Finger::RI => -0.30,
        Finger::LT | Finger::RT => -1.80,
    };

    let ox1 = (k1.width() * KEY_EDGE_OFFSET).min(KEY_EDGE_OFFSET);
    let ox2 = (k1.width() * KEY_EDGE_OFFSET).min(KEY_EDGE_OFFSET);

    let oy1 = (k2.height() * KEY_EDGE_OFFSET).min(KEY_EDGE_OFFSET);
    let oy2 = (k2.height() * KEY_EDGE_OFFSET).min(KEY_EDGE_OFFSET);

    let l1 = k1.x() + ox1;
    let r1 = k1.x() - ox1 + k1.width();
    let t1 = k1.y() + oy1 + f_len(f1);
    let b1 = k1.y() - oy1 + k1.height() + f_len(f1);

    let l2 = k2.x() + ox2;
    let r2 = k2.x() - ox2 + k2.width();
    let t2 = k2.y() + oy2 + f_len(f2);
    let b2 = k2.y() - oy2 + k2.height() + f_len(f2);

    let dx = (l1.max(l2) - r1.min(r2)).max(0.0);
    let dy = (t1.max(t2) - b1.min(b2)).max(0.0);

    // Checks whether or not a finger is below or to the side of another finger, in which case the
    // distance is considered negative. To the side meaning, where the distance between qwerty `er`
    // pressed with middle and index is considered 1, if each key were pressed with the other
    // finger, the distance is negative (because who the fuck is doing that, that's not good).

    let xo = x_finger_overlap(f1, f2);

    // match (f1.hand(), f2.hand()) {
    //     (Hand::Left, Hand::Left) => match ((f1 as u8) > (f2 as u8), (f1 as u8) < (f2 as u8)) {
    //         (true, false) if r1 < l2 => (-dx, dy),
    //         (false, true) if l1 > r2 => (-dx, dy),
    //         _ => (dx, dy),
    //     },
    //     (Hand::Right, Hand::Right) => match ((f2 as u8) > (f1 as u8), (f2 as u8) < (f1 as u8)) {
    //         (true, false) if r1 > l2 => (-dx, dy),
    //         (false, true) if l1 < r2 => (-dx, dy),
    //         _ => (dx, dy),
    //     },
    //     _ => (dx, dy)
    // }
    match ((f1 as u8) > (f2 as u8), (f1 as u8) < (f2 as u8)) {
        (true, false) if r1 < l2 + xo => (-dx, dy),
        (false, true) if l1 + xo > r2 => (-dx, dy),
        _ => (dx, dy),
    }
}

fn dist(k1: &PhysicalKey, k2: &PhysicalKey, f1: Finger, f2: Finger) -> f64 {
    let (dx, dy) = dx_dy(k1, k2, f1, f2);

    dx.hypot(dy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    static CON: Lazy<CharMapping> =
        Lazy::new(|| CharMapping::from("abcdefghijklmnopqrstuvwxyz'.,;/"));

    #[test]
    fn test_key_dist() {
        let k1 = "1 0 0 0"
            .parse::<PhysicalKey>()
            .expect("couldn't create k1");

        let k2 = "2 1 0 0"
            .parse::<PhysicalKey>()
            .expect("couldn't create k2");

        let d = dist(&k1, &k2, Finger::RP, Finger::RP);

        approx::assert_abs_diff_eq!(d, 2f64.sqrt(), epsilon = 1e-9);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_square_shapes() {
        // fn print_key_info(layout: &FastLayout, c: char) {
        //     let i = match layout.matrix.iter().position(|k| k == &c) {
        //         Some(i) => i,
        //         None => {
        //             println!("layout '{}' does not contain '{c}'", layout.name);
        //             return;
        //         }
        //     };

        //     let p = &layout.matrix_physical[i];
        //     let f = &layout.matrix_fingers[i];

        //     println!("{c} uses {f}, key: {p:?}")
        // }

        let k1 = "6.25 3 1 1"
            .parse::<PhysicalKey>()
            .expect("couldn't create k1");

        let k2 = "3.75 4 6.25 1 "
            .parse::<PhysicalKey>()
            .expect("couldn't create k2");

        let d = dist(&k1, &k2, Finger::LP, Finger::LP);

        approx::assert_abs_diff_eq!(d, 1.0, epsilon = 1e-9);

        // let layout = crate::layout::Layout::load("../layouts/qwerty.dof").unwrap();

        // print_key_info(&layout, 'b');
        // print_key_info(&layout, '␣');
    }

    #[test]
    fn layout_str() {
        let qwerty_bytes = CON.to_lossy("qwertyuiopasdfghjkl;zxcvbnm,./".chars());
        println!("{qwerty_bytes:?}");
        let qwerty = FastLayout::try_from(qwerty_bytes.as_slice()).expect("couldn't create qwerty");

        assert_eq!(
            CON.map_us(&qwerty.matrix).collect::<Vec<_>>(),
            vec![
                'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p', 'a', 's', 'd', 'f', 'g', 'h',
                'j', 'k', 'l', ';', 'z', 'x', 'c', 'v', 'b', 'n', 'm', ',', '.', '/'
            ]
        );
        assert_eq!(
            qwerty.layout_str(&CON),
            "qwertyuiopasdfghjkl;zxcvbnm,./".to_string()
        );
    }

    #[test]
    fn swap() {
        let qwerty_bytes = CON.to_lossy("qwertyuiopasdfghjkl;zxcvbnm,./".chars());
        println!("{qwerty_bytes:?}");
        let mut qwerty =
            FastLayout::try_from(qwerty_bytes.as_slice()).expect("couldn't create qwerty");

        qwerty.swap(10, 11);
        assert_eq!(
            qwerty.layout_str(&CON),
            "qwertyuiopsadfghjkl;zxcvbnm,./".to_owned()
        );
    }

    #[test]
    fn swap_no_bounds() {
        let qwerty_bytes = CON.to_lossy("qwertyuiopasdfghjkl;zxcvbnm,./".chars());
        let mut qwerty =
            FastLayout::try_from(qwerty_bytes.as_slice()).expect("couldn't create qwerty");

        qwerty.swap(9, 12).unwrap();
        assert_eq!(
            qwerty.layout_str(&CON),
            "qwertyuiodaspfghjkl;zxcvbnm,./".to_string()
        );
    }

    #[test]
    fn swap_cols_no_bounds() {
        let qwerty_bytes = CON.to_lossy("qwertyuiopasdfghjkl;zxcvbnm,./".chars());
        let mut qwerty =
            FastLayout::try_from(qwerty_bytes.as_slice()).expect("couldn't create qwerty");

        qwerty.swap_cols(1, 9).unwrap();
        assert_eq!(
            qwerty.layout_str(&CON),
            "qpertyuiowa;dfghjklsz/cvbnm,.x".to_string()
        );
    }

    #[test]
    fn swap_pair() {
        let qwerty_bytes = CON.to_lossy("qwertyuiopasdfghjkl;zxcvbnm,./".chars());
        let mut qwerty =
            FastLayout::try_from(qwerty_bytes.as_slice()).expect("couldn't create qwerty");

        let new_swap = PosPair::new(0, 29);
        qwerty.swap_pair(&new_swap);
        assert_eq!(
            qwerty.layout_str(&CON),
            "/wertyuiopasdfghjkl;zxcvbnm,.q".to_string()
        );
    }

    #[test]
    fn swap_pair_no_bounds() {
        let qwerty_bytes = CON.to_lossy("qwertyuiopasdfghjkl;zxcvbnm,./".chars());
        let mut qwerty =
            FastLayout::try_from(qwerty_bytes.as_slice()).expect("couldn't create qwerty");

        let new_swap = PosPair::new(0, 29);
        qwerty.swap_pair(&new_swap).unwrap();
        assert_eq!(
            qwerty.layout_str(&CON),
            "/wertyuiopasdfghjkl;zxcvbnm,.q".to_string()
        );
    }

    #[test]
    fn char_to_finger() {
        let qwerty_bytes = CON.to_lossy("qwertyuiopasdfghjkl;zxcvbnm,./".chars());
        let qwerty = FastLayout::try_from(qwerty_bytes.as_slice()).expect("couldn't create qwerty");

        assert_eq!(
            qwerty.char_to_finger.get(CON.to_single_lossy('a') as usize),
            Some(&Some(Finger::LP))
        );
        assert_eq!(
            qwerty.char_to_finger.get(CON.to_single_lossy('w') as usize),
            Some(&Some(Finger::LR))
        );
        assert_eq!(
            qwerty.char_to_finger.get(CON.to_single_lossy('c') as usize),
            Some(&Some(Finger::LM))
        );

        assert_eq!(
            qwerty.char_to_finger.get(CON.to_single_lossy('r') as usize),
            Some(&Some(Finger::LI))
        );
        assert_eq!(
            qwerty.char_to_finger.get(CON.to_single_lossy('b') as usize),
            Some(&Some(Finger::LI))
        );

        assert_eq!(
            qwerty.char_to_finger.get(CON.to_single_lossy('h') as usize),
            Some(&Some(Finger::RI))
        );
        assert_eq!(
            qwerty.char_to_finger.get(CON.to_single_lossy('u') as usize),
            Some(&Some(Finger::RI))
        );

        assert_eq!(
            qwerty.char_to_finger.get(CON.to_single_lossy('i') as usize),
            Some(&Some(Finger::RM))
        );
        assert_eq!(
            qwerty.char_to_finger.get(CON.to_single_lossy('.') as usize),
            Some(&Some(Finger::RR))
        );
        assert_eq!(
            qwerty.char_to_finger.get(CON.to_single_lossy(';') as usize),
            Some(&Some(Finger::RP))
        );
    }

    #[test]
    fn char() {
        let qwerty_bytes = CON.to_lossy("qwertyuiopasdfghjkl;zxcvbnm,./".chars());
        let qwerty = FastLayout::try_from(qwerty_bytes.as_slice()).expect("couldn't create qwerty");

        assert_eq!(qwerty.char(4 + (1 * 10)), Some(CON.to_single_lossy('g')));
        assert_eq!(qwerty.char(9 + (2 * 10)), Some(CON.to_single_lossy('/')));
        assert_eq!(qwerty.char(8 + (1 * 10)), Some(CON.to_single_lossy('l')));
    }

    #[test]
    fn char_by_index() {
        let qwerty_bytes = CON.to_lossy("qwertyuiopasdfghjkl;zxcvbnm,./".chars());
        let qwerty = FastLayout::try_from(qwerty_bytes.as_slice()).expect("couldn't create qwerty");

        assert_eq!(qwerty.char(10), Some(CON.to_single_lossy('a')));
        assert_eq!(qwerty.char(24), Some(CON.to_single_lossy('b')));
        assert_eq!(qwerty.char(22), Some(CON.to_single_lossy('c')));
    }

    // #[test]
    // fn random_layouts() {
    // 	use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
    // 	use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
    // 	use std::io::Write;
    // 	use crate::analyze::LayoutAnalysis;

    // 	let anal = LayoutAnalysis::new("english", None).unwrap();
    // 	let available_chars = available_chars("english");

    // 	let pb = ProgressBar::new(10_000_000);
    // 	pb.set_style(ProgressStyle::default_bar()
    // 		.template("[{elapsed_precise}] [{bar:40.white/white}] [eta: {eta}] - {per_sec:>4} {pos:>6}/{len}")
    // 		.progress_chars("=>-"));

    // 	let mut res = Vec::with_capacity(10_000_000);

    // 	let start = std::time::Instant::now();

    // 	(0..10_000_000)
    // 		.into_par_iter()
    // 		.progress_with(pb)
    // 		.map(|_| -> f32 {
    // 			let r = FastLayout::random(available_chars);
    // 			anal.score(&r, 5_000) as f32
    // 		})
    // 		.collect_into_vec(&mut res);

    // 	let end = std::time::Instant::now();
    // 	res.sort_unstable_by(|a, b| b.partial_cmp(a).unwrap());
    // 	println!("that took {}s.", (end - start).as_secs_f64());

    // 	let mut f = std::fs::OpenOptions::new()
    // 		.write(true)
    // 		.create(true)
    // 		.truncate(true)
    // 		.open("10mil_scores")
    // 		.unwrap();

    // 	let mut to_save_vec = Vec::new();
    // 	res
    // 		.into_par_iter()
    // 		.map(|v| v.to_string())
    // 		.collect_into_vec(&mut to_save_vec);
    // 	let to_save = to_save_vec.join("\n");

    // 	f.write(to_save.as_bytes()).unwrap();
    // }
}
