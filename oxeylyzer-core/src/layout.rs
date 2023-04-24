use crate::trigram_patterns::{TrigramPattern, TRIGRAM_COMBINATIONS};
use crate::utility::*;

pub type CharToFinger = [usize; 60];
pub type Matrix<T> = [T; 30];

pub trait Layout<T: Copy + Default> {
    fn new() -> Self;

    fn random(available_chars: [T; 30]) -> Self;

    fn random_pins(layout_chars: [T; 30], pins: &[usize]) -> Self;

    fn c(&self, i: usize) -> T;

    unsafe fn cu(&self, i: usize) -> T;

    fn char(&self, x: usize, y: usize) -> T;

    fn swap(&mut self, i1: usize, i2: usize) -> Option<()>;

    unsafe fn swap_xy_no_bounds(&mut self, i1: usize, i2: usize);

    fn swap_pair(&mut self, pair: &PosPair) -> Option<()>;

    unsafe fn swap_no_bounds(&mut self, pair: &PosPair);

    unsafe fn swap_cols_no_bounds(&mut self, col1: usize, col2: usize);

    fn swap_indexes(&mut self);

    fn get_index(&self, index: usize) -> [T; 6];

    fn get_trigram_pattern(&self, trigram: &[T; 3]) -> TrigramPattern;

    unsafe fn get_trigram_pattern_unchecked(&self, trigram: &[T; 3]) -> TrigramPattern;
}

#[derive(Debug, Clone, PartialEq)]
pub struct FastLayout {
    pub matrix: Matrix<u8>,
    pub char_to_finger: CharToFinger,
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
            new_layout.char_to_finger[byte as usize] = I_TO_COL[i];
        }
        new_layout
    }
}

impl TryFrom<&[u8]> for FastLayout {
    type Error = anyhow::Error;

    fn try_from(layout_bytes: &[u8]) -> Result<Self, Self::Error> {
        if layout_bytes.len() >= 30 {
            let mut new_layout = FastLayout::new();

            for (i, &byte) in layout_bytes.into_iter().enumerate() {
                new_layout.matrix[i] = byte;
                new_layout.char_to_finger[byte as usize] = I_TO_COL[i];
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
}

impl Layout<u8> for FastLayout {
    fn new() -> FastLayout {
        FastLayout {
            matrix: [u8::MAX; 30],
            char_to_finger: [usize::MAX; 60],
            score: 0.0,
        }
    }

    fn random(mut with_chars: [u8; 30]) -> FastLayout {
        shuffle_pins::<30, u8>(&mut with_chars, &[]);
        FastLayout::from(with_chars)
    }

    fn random_pins(mut layout_chars: [u8; 30], pins: &[usize]) -> FastLayout {
        shuffle_pins::<30, u8>(&mut layout_chars, pins);
        FastLayout::from(layout_chars)
    }

    #[inline(always)]
    fn c(&self, i: usize) -> u8 {
        self.matrix[i]
    }

    #[inline(always)]
    unsafe fn cu(&self, i: usize) -> u8 {
        *self.matrix.get_unchecked(i)
    }

    #[inline]
    fn char(&self, x: usize, y: usize) -> u8 {
        assert!(x < 10 && y < 3);
        self.matrix[x + 10 * y]
    }

    fn swap(&mut self, i1: usize, i2: usize) -> Option<()> {
        if i1 < 30 && i2 < 30 {
            let char1 = self.matrix[i1];
            let char2 = self.matrix[i2];

            self.matrix[i1] = char2;
            self.matrix[i2] = char1;
            self.char_to_finger[char1 as usize] = I_TO_COL[i2];
            self.char_to_finger[char2 as usize] = I_TO_COL[i1];

            return Some(());
        } else {
            println!("Invalid coordinate, swap was cancelled");
            None
        }
    }

    #[inline(always)]
    unsafe fn swap_xy_no_bounds(&mut self, i1: usize, i2: usize) {
        let char1 = self.cu(i1);
        let char2 = self.cu(i2);

        *self.matrix.get_unchecked_mut(i1) = char2;
        *self.matrix.get_unchecked_mut(i2) = char1;

        *self.char_to_finger.get_unchecked_mut(char1 as usize) = *I_TO_COL.get_unchecked(i2);
        *self.char_to_finger.get_unchecked_mut(char2 as usize) = *I_TO_COL.get_unchecked(i1);
    }

    #[inline(always)]
    fn swap_pair(&mut self, pair: &PosPair) -> Option<()> {
        self.swap(pair.0, pair.1)
    }

    #[inline(always)]
    unsafe fn swap_no_bounds(&mut self, pair: &PosPair) {
        self.swap_xy_no_bounds(pair.0, pair.1);
    }

    unsafe fn swap_cols_no_bounds(&mut self, col1: usize, col2: usize) {
        self.swap_xy_no_bounds(col1, col2);
        self.swap_xy_no_bounds(col1 + 10, col2 + 10);
        self.swap_xy_no_bounds(col1 + 20, col2 + 20);
    }

    fn swap_indexes(&mut self) {
        unsafe {
            self.swap_cols_no_bounds(3, 6);
            self.swap_cols_no_bounds(4, 5);
        }
    }

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

    fn get_trigram_pattern(&self, trigram: &[u8; 3]) -> TrigramPattern {
        let a = *self
            .char_to_finger
            .get(trigram[0] as usize)
            .unwrap_or_else(|| &usize::MAX);
        let b = *self
            .char_to_finger
            .get(trigram[1] as usize)
            .unwrap_or_else(|| &usize::MAX);
        let c = *self
            .char_to_finger
            .get(trigram[2] as usize)
            .unwrap_or_else(|| &usize::MAX);
        if (a | b | c) == usize::MAX {
            return TrigramPattern::Invalid;
        }
        // a, b and c are numbers between 0 and 7. This means they fit in exactly 3 bits (7 == 0b111)
        let combination = (a << 6) | (b << 3) | c;
        TRIGRAM_COMBINATIONS[combination]
    }

    unsafe fn get_trigram_pattern_unchecked(&self, trigram: &[u8; 3]) -> TrigramPattern {
        let a = *self
            .char_to_finger
            .get(trigram[0] as usize)
            .unwrap_unchecked();
        let b = *self
            .char_to_finger
            .get(trigram[1] as usize)
            .unwrap_unchecked();
        let c = *self
            .char_to_finger
            .get(trigram[2] as usize)
            .unwrap_unchecked();
        // a, b and c are numbers between 0 and 7. This means they fit in exactly 3 bits (7 == 0b111)
        let combination = (a << 6) | (b << 3) | c;
        TRIGRAM_COMBINATIONS[combination]
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
            CON.from(qwerty.matrix),
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

        unsafe { qwerty.swap_xy_no_bounds(9, 12) };
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

        unsafe { qwerty.swap_cols_no_bounds(1, 9) };
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
        unsafe { qwerty.swap_no_bounds(&new_swap) };
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
            Some(&0usize)
        );
        assert_eq!(
            qwerty.char_to_finger.get(CON.to_single_lossy('w') as usize),
            Some(&1usize)
        );
        assert_eq!(
            qwerty.char_to_finger.get(CON.to_single_lossy('c') as usize),
            Some(&2usize)
        );

        assert_eq!(
            qwerty.char_to_finger.get(CON.to_single_lossy('r') as usize),
            Some(&3usize)
        );
        assert_eq!(
            qwerty.char_to_finger.get(CON.to_single_lossy('b') as usize),
            Some(&3usize)
        );

        assert_eq!(
            qwerty.char_to_finger.get(CON.to_single_lossy('h') as usize),
            Some(&4usize)
        );
        assert_eq!(
            qwerty.char_to_finger.get(CON.to_single_lossy('u') as usize),
            Some(&4usize)
        );

        assert_eq!(
            qwerty.char_to_finger.get(CON.to_single_lossy('i') as usize),
            Some(&5usize)
        );
        assert_eq!(
            qwerty.char_to_finger.get(CON.to_single_lossy('.') as usize),
            Some(&6usize)
        );
        assert_eq!(
            qwerty.char_to_finger.get(CON.to_single_lossy(';') as usize),
            Some(&7usize)
        );
    }

    #[test]
    fn char() {
        let qwerty_bytes = CON.to_lossy("qwertyuiopasdfghjkl;zxcvbnm,./".chars());
        let qwerty = FastLayout::try_from(qwerty_bytes.as_slice()).expect("couldn't create qwerty");

        assert_eq!(qwerty.char(4, 1), CON.to_single_lossy('g'));
        assert_eq!(qwerty.char(9, 2), CON.to_single_lossy('/'));
        assert_eq!(qwerty.char(8, 1), CON.to_single_lossy('l'));
    }

    #[test]
    fn char_by_index() {
        let qwerty_bytes = CON.to_lossy("qwertyuiopasdfghjkl;zxcvbnm,./".chars());
        let qwerty = FastLayout::try_from(qwerty_bytes.as_slice()).expect("couldn't create qwerty");

        assert_eq!(qwerty.c(10), CON.to_single_lossy('a'));
        assert_eq!(qwerty.c(24), CON.to_single_lossy('b'));
        assert_eq!(qwerty.c(22), CON.to_single_lossy('c'));
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
