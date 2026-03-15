use std::sync::Arc;

use ahash::AHashMap as HashMap;
use itertools::Itertools;
use libdof::prelude::{Finger, PhysicalKey, Shape};
use serde::Serialize;

use crate::{
    char_mapping::CharMapping,
    layout::{LayoutMetadata, Pos, PosPair},
    utility::*,
    weights::FingerWeights,
};

const KEY_EDGE_OFFSET: f64 = 0.5;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(into = "crate::layout::Layout")]
pub struct FastLayout {
    pub name: Option<String>,
    pub matrix: Box<[u8]>,
    pub char_to_finger: Box<[Option<Finger>]>,
    pub matrix_fingers: Box<[Finger]>,
    pub matrix_physical: Box<[PhysicalKey]>,
    pub fspeed_indices: FSpeedIndices,
    pub scissor_indices: ScissorIndices,
    pub lsb_indices: LsbIndices,
    pub pinky_ring_indices: PinkyRingIndices,
    pub stretch_indices: StretchCache,
    pub usage_indices: UsageIndices,
    pub possible_swaps: Box<[PosPair]>,
    pub mapping: Arc<CharMapping>,
    pub metadata: Arc<LayoutMetadata>,
    pub shape: Shape,
}

impl FastLayout {
    #[inline(always)]
    pub fn finger(&self, pos: Pos) -> Option<Finger> {
        self.matrix_fingers.get(pos as usize).copied()
    }

    pub fn layout_str(&self) -> String {
        self.mapping.map_us(&self.matrix).collect()
    }

    pub fn formatted_string(&self) -> String {
        let mut res = String::new();

        let mut iter = self.matrix.iter();

        for &l in self.shape.inner().iter() {
            let mut i = 0;
            for u in iter.by_ref() {
                let c = self.mapping.get_c(*u);
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

        res.trim().to_string()
    }

    pub fn random(&self) -> Self {
        self.random_with_pins(&[])
    }

    pub fn random_with_pins(&self, pins: &[usize]) -> Self {
        let mut res = self.clone();

        res.name = None;
        res.char_to_finger = Box::new([None; 60]);

        shuffle_pins(&mut res.matrix, pins);

        res.matrix
            .iter()
            .enumerate()
            .for_each(|(i, &c)| res.char_to_finger[c as usize] = Some(res.matrix_fingers[i]));

        res
    }

    #[inline(always)]
    pub fn char(&self, i: Pos) -> Option<u8> {
        self.matrix.get(i as usize).copied()
    }

    #[inline(always)]
    pub fn swap(&mut self, i1: Pos, i2: Pos) -> Option<()> {
        let char1 = self.char(i1)?;
        let char2 = self.char(i2)?;

        *self.matrix.get_mut(i1 as usize)? = char2;
        *self.matrix.get_mut(i2 as usize)? = char1;

        *self.char_to_finger.get_mut(char1 as usize)? =
            Some(*self.matrix_fingers.get(i2 as usize)?);
        *self.char_to_finger.get_mut(char2 as usize)? =
            Some(*self.matrix_fingers.get(i1 as usize)?);

        Some(())
    }

    #[inline(always)]
    pub fn swap_pair(&mut self, pair: &PosPair) -> Option<()> {
        self.swap(pair.0, pair.1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BigramPair {
    pub pair: PosPair,
    pub dist: i64,
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
        finger_weights: &FingerWeights,
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

        let max_finger_weight = finger_weights.max();

        let fingers: Box<[_; 10]> = Finger::FINGERS
            .map(|finger| {
                fingers
                    .iter()
                    .zip(keyboard)
                    .zip(0u8..)
                    .filter_map(|((f, k), i)| (f == &finger).then_some((k, i)))
                    .tuple_combinations::<(_, _)>()
                    .map(|((k1, i1), (k2, i2))| {
                        let pair = PosPair(i1, i2);
                        let dist = (dist(k1, k2, finger, finger)
                            * 100.0
                            * (max_finger_weight / finger_weights.get(finger)))
                            as i64;

                        BigramPair { pair, dist }
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
pub struct ScissorIndices {
    pub pairs: Box<[PosPair]>,
    pub keys_in_scissor: Box<[bool]>,
}

impl ScissorIndices {
    pub fn new(fingers: &[Finger], keyboard: &[PhysicalKey]) -> Self {
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

        fn adjacent_fingers_same_hand(f1: Finger, f2: Finger) -> bool {
            use Finger::*;

            if f1.hand() != f2.hand() {
                return false;
            }

            matches!(
                (f1, f2),
                (LP, LR)
                    | (LR, LP)
                    | (LR, LM)
                    | (LM, LR)
                    | (LM, LI)
                    | (LI, LM)
                    | (RI, RM)
                    | (RM, RI)
                    | (RM, RR)
                    | (RR, RM)
                    | (RR, RP)
                    | (RP, RR)
            )
        }

        let pairs = fingers
            .iter()
            .zip(keyboard)
            .enumerate()
            .map(|(i, t)| (i as u8, t))
            .tuple_combinations::<(_, _)>()
            .flat_map(|((i1, (&f1, k1)), (i2, (&f2, k2)))| {
                if !adjacent_fingers_same_hand(f1, f2) {
                    return None;
                }

                let (_, dy) = ((k1.x() - k2.x()).abs(), (k1.y() - k2.y()).abs());

                if dy.abs() <= 1.9 {
                    return None;
                }

                if f1.is_index() && f2.is_middle() && k1.y() >= k2.y() {
                    return None;
                }
                if f2.is_index() && f1.is_middle() && k1.y() <= k2.y() {
                    return None;
                }

                Some(PosPair(i1, i2))
            })
            .collect::<Box<_>>();

        let mut keys_in_scissor = vec![false; fingers.len()].into_boxed_slice();
        for PosPair(i1, i2) in &pairs {
            if let Some(v) = keys_in_scissor.get_mut(*i1 as usize) {
                *v = true;
            }
            if let Some(v) = keys_in_scissor.get_mut(*i2 as usize) {
                *v = true;
            }
        }

        Self {
            pairs,
            keys_in_scissor,
        }
    }

    #[inline]
    pub fn affects_scissor_idx(&self, pos: Pos) -> bool {
        self.keys_in_scissor
            .get(pos as usize)
            .copied()
            .unwrap_or(false)
    }

    #[inline]
    pub fn affects_scissor(&self, PosPair(a, b): PosPair) -> bool {
        self.affects_scissor_idx(a) || self.affects_scissor_idx(b)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LsbIndices {
    pub pairs: Box<[PosPair]>,
}

impl LsbIndices {
    pub fn new(fingers: &[Finger], keyboard: &[PhysicalKey]) -> Self {
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

        let pairs = fingers
            .iter()
            .zip(keyboard)
            .enumerate()
            .map(|(i, t)| (i as u8, t))
            .tuple_combinations::<(_, _)>()
            .filter_map(|((i1, (&f1, k1)), (i2, (&f2, k2)))| {
                if f1.hand() != f2.hand() {
                    return None;
                }

                if f1.is_middle() && f2.is_index() || f2.is_middle() && f1.is_index() {
                    let (dx, _) = dx_dy(k1, k2, f1, f2);
                    if dx.abs() >= 1.5 {
                        Some(PosPair(i1, i2))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Box<_>>();

        Self { pairs }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PinkyRingIndices {
    pub pairs: Box<[PosPair]>,
    pub keys_in_pinky_ring: Box<[bool]>,
}

impl PinkyRingIndices {
    pub fn new(fingers: &[Finger]) -> Self {
        assert!(
            fingers.len() <= u8::MAX as usize,
            "Too many keys to index with u8, max is {}",
            u8::MAX
        );

        use Finger::*;

        let is_pinky = |f: Finger| matches!(f, LP | RP);
        let is_ring = |f: Finger| matches!(f, LR | RR);

        let pairs = fingers
            .iter()
            .enumerate()
            .map(|(i, t)| (i as u8, t))
            .tuple_combinations::<(_, _)>()
            .filter_map(|((i1, &f1), (i2, &f2))| {
                // same hand only
                if f1.hand() != f2.hand() {
                    return None;
                }

                let (a_pinky_b_ring, a_ring_b_pinky) =
                    (is_pinky(f1) && is_ring(f2), is_ring(f1) && is_pinky(f2));

                if a_pinky_b_ring || a_ring_b_pinky {
                    Some(PosPair(i1, i2))
                } else {
                    None
                }
            })
            .collect::<Box<_>>();

        let mut keys_in_pinky_ring = vec![false; fingers.len()].into_boxed_slice();
        for PosPair(i1, i2) in &pairs {
            if let Some(v) = keys_in_pinky_ring.get_mut(*i1 as usize) {
                *v = true;
            }
            if let Some(v) = keys_in_pinky_ring.get_mut(*i2 as usize) {
                *v = true;
            }
        }

        Self {
            pairs,
            keys_in_pinky_ring,
        }
    }

    #[inline]
    pub fn affects_pinky_ring_idx(&self, pos: Pos) -> bool {
        self.keys_in_pinky_ring
            .get(pos as usize)
            .copied()
            .unwrap_or(false)
    }

    #[inline]
    pub fn affects_pinky_ring(&self, PosPair(a, b): PosPair) -> bool {
        self.affects_pinky_ring_idx(a) || self.affects_pinky_ring_idx(b)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct StretchCache {
    pub all_pairs: Box<[BigramPair]>,
    pub per_key_pair: HashMap<PosPair, Box<[BigramPair]>>,
}

impl StretchCache {
    pub fn new(keys: &[char], fingers: &[Finger], keyboard: &[PhysicalKey]) -> Self {
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
            .map(|(i, t)| (i as u8, t))
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
                    dist: (stretch * 100.0) as i64,
                })
            })
            .collect::<Box<[_]>>();

        // println!("pair count: {}", all_pairs.len());

        let per_keypair = (0..(fingers.len() as u8))
            .cartesian_product(0..(fingers.len() as u8))
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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct UsageIndices {
    pub per_finger: Box<[Box<[usize]>; 10]>, // TODO: use Pos or something rather than usize
}

impl UsageIndices {
    pub fn new(fingers: &[Finger]) -> Self {
        let per_finger = Finger::FINGERS
            .map(|f| {
                fingers
                    .iter()
                    .enumerate()
                    .filter_map(|(pos, &lf)| (f == lf).then_some(pos))
                    .collect::<Box<[_]>>()
            })
            .into();

        Self { per_finger }
    }

    pub fn get(&self, finger: Finger) -> &[usize] {
        &self.per_finger[finger as usize]
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::{generate::LayoutGeneration, layout::Layout};

    use super::*;
    use once_cell::sync::Lazy;

    static GEN: Lazy<LayoutGeneration> =
        Lazy::new(|| LayoutGeneration::new("english", "static", None).unwrap());

    static QWERTY: Lazy<FastLayout> = Lazy::new(|| {
        let dof_str = r#"
            {
                "name": "Qwerty",
                "board": "ansi",
                "layers": {
                    "main": [
                        "q w e r t  y u i o p",
                        "a s d f g  h j k l ;",
                        "z x c v b  n m , . /"
                    ]
                },
                "fingering": "traditional"
            }
        "#;

        let layout = serde_json::from_str::<Layout>(dof_str).unwrap();

        GEN.fast_layout(&layout, &[])
    });

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
        let v = "abcdefghijklmnopqrstuvwxyz";
        assert_eq!(
            GEN.mapping.map_cs(v).collect::<Vec<_>>(),
            QWERTY.mapping.map_cs(v).collect::<Vec<_>>()
        );

        assert_eq!(
            GEN.mapping.map_us(&QWERTY.matrix).collect::<Vec<_>>(),
            vec![
                'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p', 'a', 's', 'd', 'f', 'g', 'h',
                'j', 'k', 'l', ';', 'z', 'x', 'c', 'v', 'b', 'n', 'm', ',', '.', '/'
            ]
        );
        assert_eq!(
            QWERTY.layout_str(),
            "qwertyuiopasdfghjkl;zxcvbnm,./".to_string()
        );
    }

    #[test]
    fn random() {
        let random = QWERTY.random();

        assert_eq!(random.usage_indices, QWERTY.usage_indices);
        assert_eq!(random.fspeed_indices, QWERTY.fspeed_indices);
        assert_eq!(random.stretch_indices, QWERTY.stretch_indices);
        assert_eq!(random.mapping, QWERTY.mapping);
        assert_eq!(random.matrix_fingers, QWERTY.matrix_fingers);
        assert_eq!(random.matrix_physical, QWERTY.matrix_physical);
        assert_eq!(random.possible_swaps, QWERTY.possible_swaps);
        assert_eq!(random.shape, QWERTY.shape);

        assert_eq!(random.name, None);

        let r_hs = random.layout_str().chars().collect::<HashSet<_>>();
        let q_hs = QWERTY.layout_str().chars().collect::<HashSet<_>>();

        assert_eq!(r_hs, q_hs);

        for (i, &u) in random.matrix.iter().enumerate() {
            let qwerty_eq = QWERTY.matrix[i];

            assert_eq!(
                random.char_to_finger[u as usize],
                QWERTY.char_to_finger[qwerty_eq as usize]
            );
        }
    }

    #[test]
    fn swap() {
        let mut qwerty = QWERTY.clone();

        qwerty.swap(10, 11);
        assert_eq!(
            qwerty.layout_str(),
            "qwertyuiopsadfghjkl;zxcvbnm,./".to_owned()
        );
    }

    #[test]
    fn swap_no_bounds() {
        let mut qwerty = QWERTY.clone();

        qwerty.swap(9, 12).unwrap();
        assert_eq!(
            qwerty.layout_str(),
            "qwertyuiodaspfghjkl;zxcvbnm,./".to_string()
        );
    }

    #[test]
    fn swap_pair() {
        let mut qwerty = QWERTY.clone();

        let new_swap = PosPair::new(0, 29);
        qwerty.swap_pair(&new_swap);
        assert_eq!(
            qwerty.layout_str(),
            "/wertyuiopasdfghjkl;zxcvbnm,.q".to_string()
        );
    }

    #[test]
    fn swap_pair_no_bounds() {
        let mut qwerty = QWERTY.clone();

        let new_swap = PosPair::new(0, 29);
        qwerty.swap_pair(&new_swap).unwrap();
        assert_eq!(
            qwerty.layout_str(),
            "/wertyuiopasdfghjkl;zxcvbnm,.q".to_string()
        );
    }

    #[test]
    fn char_to_finger() {
        let qwerty = QWERTY.clone();

        assert_eq!(
            qwerty.char_to_finger.get(GEN.mapping.get_u('a') as usize),
            Some(&Some(Finger::LP))
        );
        assert_eq!(
            qwerty.char_to_finger.get(GEN.mapping.get_u('w') as usize),
            Some(&Some(Finger::LR))
        );
        assert_eq!(
            qwerty.char_to_finger.get(GEN.mapping.get_u('c') as usize),
            Some(&Some(Finger::LM))
        );

        assert_eq!(
            qwerty.char_to_finger.get(GEN.mapping.get_u('r') as usize),
            Some(&Some(Finger::LI))
        );
        assert_eq!(
            qwerty.char_to_finger.get(GEN.mapping.get_u('b') as usize),
            Some(&Some(Finger::LI))
        );

        assert_eq!(
            qwerty.char_to_finger.get(GEN.mapping.get_u('h') as usize),
            Some(&Some(Finger::RI))
        );
        assert_eq!(
            qwerty.char_to_finger.get(GEN.mapping.get_u('u') as usize),
            Some(&Some(Finger::RI))
        );

        assert_eq!(
            qwerty.char_to_finger.get(GEN.mapping.get_u('i') as usize),
            Some(&Some(Finger::RM))
        );
        assert_eq!(
            qwerty.char_to_finger.get(GEN.mapping.get_u('.') as usize),
            Some(&Some(Finger::RR))
        );
        assert_eq!(
            qwerty.char_to_finger.get(GEN.mapping.get_u(';') as usize),
            Some(&Some(Finger::RP))
        );
    }

    #[test]
    fn char() {
        let qwerty = QWERTY.clone();

        assert_eq!(qwerty.char(4 + (1 * 10)), Some(GEN.mapping.get_u('g')));
        assert_eq!(qwerty.char(9 + (2 * 10)), Some(GEN.mapping.get_u('/')));
        assert_eq!(qwerty.char(8 + (1 * 10)), Some(GEN.mapping.get_u('l')));
    }

    #[test]
    fn char_by_index() {
        let qwerty = QWERTY.clone();

        assert_eq!(qwerty.char(10), Some(GEN.mapping.get_u('a')));
        assert_eq!(qwerty.char(24), Some(GEN.mapping.get_u('b')));
        assert_eq!(qwerty.char(22), Some(GEN.mapping.get_u('c')));
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
