use anyhow::{bail, Result};
use itertools::Itertools;
use libdof::prelude::{Dof, Finger, Keyboard, PhysicalKey, Shape};

use crate::{utility::*, *};

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
    pub fn layout_str(&self, con: &ConvertU8) -> String {
        con.as_str(&self.matrix)
    }

    pub fn formatted_string(&self, con: &ConvertU8) -> String {
        let mut res = String::new();

        for (i, u) in self.matrix.iter().enumerate() {
            let c = con.from_single(*u);
            if i % 10 == 0 && i > 0 {
                res.push('\n');
            }
            if (i + 5) % 10 == 0 {
                res.push(' ');
            }
            res.push(c);
            res.push(' ');
        }

        res
    }

    pub fn from_dof(dof: Dof, convert: &mut ConvertU8) -> Result<Self> {
        use libdof::prelude::{Key, SpecialKey};

        let key_count = dof.main_layer().shape().inner().iter().sum::<usize>();
        if key_count != 30 {
            bail!("Invalid key count {key_count}, expected 30")
        }

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
        let matrix_physical = default_physical_map();

        let mut char_to_finger = Box::new([None; 60]);
        matrix
            .iter()
            .enumerate()
            .for_each(|(i, &c)| char_to_finger[c as usize] = Some(matrix_fingers[i]));

        let fspeed_indices = FSpeedIndices::new(&matrix_fingers, &matrix_physical);
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
        let shape = Shape::from(vec![10, 10, 10]);
        let score = 0.0;

        FastLayout {
            matrix,
            matrix_fingers,
            matrix_physical,
            char_to_finger,
            fspeed_indices,
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

fn dx_dy(k1: &PhysicalKey, k2: &PhysicalKey, _f1: Finger, _f2: Finger) -> (f64, f64) {
    let ox1 = (k1.width() * KEY_EDGE_OFFSET).min(KEY_EDGE_OFFSET);
    let ox2 = (k1.width() * KEY_EDGE_OFFSET).min(KEY_EDGE_OFFSET);

    let oy1 = (k2.height() * KEY_EDGE_OFFSET).min(KEY_EDGE_OFFSET);
    let oy2 = (k2.height() * KEY_EDGE_OFFSET).min(KEY_EDGE_OFFSET);

    let l1 = k1.x() + ox1;
    let r1 = k1.x() - ox1 + k1.width();
    let t1 = k1.y() + oy1;
    let b1 = k1.y() - oy1 + k1.height();

    let l2 = k2.x() + ox2;
    let r2 = k2.x() - ox2 + k2.width();
    let t2 = k2.y() + oy2;
    let b2 = k2.y() - oy2 + k2.height();

    let dx = (l1.max(l2) - r1.min(r2)).max(0.0);
    let dy = (t1.max(t2) - b1.min(b2)).max(0.0);

    (dx, dy)
}

fn dist(k1: &PhysicalKey, k2: &PhysicalKey, f1: Finger, f2: Finger) -> f64 {
    if f1 != f2 {
        todo!("only supports distance between keys pressed with the same finger")
    }

    // TODO: move this to weights
    static F_WEIGHTS: [f64; 10] = [1.4, 3.6, 4.8, 5.5, 3.3, 3.3, 5.5, 4.8, 3.6, 1.4];

    let (dx, dy) = dx_dy(k1, k2, f1, f2);

    // TODO: think about scaling differently
    dx.hypot(dy).powf(1.3) * (5.5 / F_WEIGHTS[f1 as usize])
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

        let fingers: Box<[_; 10]> = Finger::FINGERS
            .map(|finger| {
                fingers
                    .iter()
                    .zip(keyboard)
                    .zip(0usize..)
                    .filter_map(|((f, k), i)| (f == &finger).then_some((k, i)))
                    .tuple_combinations::<(_, _)>()
                    .map(|((k1, i1), (k2, i2))| BigramPair {
                        pair: PosPair(i1, i2),
                        dist: dist(k1, k2, finger, finger),
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

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    static CON: Lazy<ConvertU8> =
        Lazy::new(|| ConvertU8::with_chars("abcdefghijklmnopqrstuvwxyz'.,;/"));

    #[test]
    fn layout_str() {
        let qwerty_bytes = CON.to_lossy("qwertyuiopasdfghjkl;zxcvbnm,./".chars());
        println!("{qwerty_bytes:?}");
        let qwerty = FastLayout::try_from(qwerty_bytes.as_slice()).expect("couldn't create qwerty");

        assert_eq!(
            CON.from(qwerty.matrix.iter().copied()),
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
