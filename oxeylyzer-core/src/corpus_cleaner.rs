use ahash::{AHashMap as HashMap, AHashSet as HashSet};

use sliding_window_alt::SlidingWindow;
use thiserror::Error;

use crate::*;

#[derive(Debug, Clone, Error)]
pub enum CorpusError {}

#[derive(Debug, Clone)]
pub struct CorpusCleaner {
    map: HashMap<char, Vec<char>>,
    shift_key: Option<char>,
    repeat_key: bool,
    raw: bool,
}

impl Default for CorpusCleaner {
    fn default() -> Self {
        Self {
            map: HashMap::default(),
            shift_key: Some(SHIFT_CHAR),
            repeat_key: false,
            raw: true,
        }
    }
}

impl CorpusCleaner {
    pub fn builder() -> CorpusCleanerBuilder {
        CorpusCleanerBuilder {
            shift_char: Some(SHIFT_CHAR),
            chars: HashSet::default(),
            shifted_chars: HashMap::default(),
            mappings: HashMap::default(),
            repeat_key: false,
        }
    }

    pub fn raw() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct CorpusCleanerBuilder {
    shift_char: Option<char>,
    chars: HashSet<char>,
    shifted_chars: HashMap<char, char>,
    mappings: HashMap<char, Vec<char>>,
    repeat_key: bool,
}

impl CorpusCleanerBuilder {
    fn include_as_uppercase(&mut self, (lower, upper): (char, char)) {
        self.shifted_chars.insert(upper, lower);
        self.chars.insert(lower);
    }

    pub fn with_uppercase_mappings(
        &mut self,
        mappings: impl IntoIterator<Item = (char, char)>,
    ) -> &mut Self {
        mappings
            .into_iter()
            .for_each(|ft| self.include_as_uppercase(ft));

        self
    }

    fn with_char(&mut self, c: char) {
        if self.chars.insert(c) && !c.is_uppercase() {
            let upper = c.to_uppercase();

            if upper.len() == 1 {
                let cu = upper
                    .into_iter()
                    .next()
                    .expect("we know the length to be 1");

                if cu != c {
                    self.include_as_uppercase((c, cu));
                }
            }
        }
    }

    pub fn with_chars(&mut self, chars: impl IntoIterator<Item = char>) -> &mut Self {
        chars.into_iter().for_each(|c| self.with_char(c));

        self
    }

    pub fn repeat_key(&mut self, enable: bool) -> &mut Self {
        self.repeat_key = enable;

        self
    }

    pub fn shift_char(&mut self, shift_char: Option<char>) -> &mut Self {
        self.shift_char = shift_char;

        self
    }

    fn with_mapping(&mut self, (from, to): (char, Vec<char>)) {
        self.mappings.insert(from, to);
    }

    pub fn with_mappings(
        &mut self,
        mappings: impl IntoIterator<Item = (char, Vec<char>)>,
    ) -> &mut Self {
        mappings.into_iter().for_each(|ft| self.with_mapping(ft));

        self
    }

    pub fn with_char_mappings(
        &mut self,
        mappings: impl IntoIterator<Item = (char, char)>,
    ) -> &mut Self {
        mappings
            .into_iter()
            .for_each(|(from, to)| self.with_mapping((from, vec![to])));

        self
    }

    pub fn with_dead_key(
        &mut self,
        mappings: impl IntoIterator<Item = (char, char)>,
        dead_key: char,
        // include_uppercase: bool,
    ) -> &mut Self {
        mappings
            .into_iter()
            .for_each(|(from, to)| self.with_mapping((from, vec![dead_key, to])));

        self
    }

    pub fn qwerty_punctuation_mappings(&mut self, enable: bool) -> &mut Self {
        if enable {
            self.with_uppercase_mappings([
                ('`', '~'),
                ('1', '!'),
                ('2', '@'),
                ('3', '#'),
                ('4', '$'),
                ('5', '%'),
                ('6', '^'),
                ('7', '&'),
                ('8', '*'),
                ('9', '('),
                ('0', ')'),
                ('[', '{'),
                (']', '}'),
                ('/', '?'),
                ('=', '+'),
                ('-', '_'),
                ('\\', '|'),
                ('\'', '"'),
                (',', '<'),
                ('.', '>'),
                (';', ':'),
            ])
        } else {
            self
        }
    }

    pub fn normalize_misc_punctuation(&mut self, normalize: bool) -> &mut Self {
        if normalize {
            self.with_char_mappings([
                ('´', '\''),
                ('‘', '\''),
                ('’', '\''),
                ('÷', '/'),
                ('‐', '-'),
                ('–', '-'),
                ('—', '-'),
            ])
            .with_uppercase_mappings([('\'', '«'), ('\'', '»'), ('\'', '“'), ('\'', '”')])
            .with_mappings([('…', vec!['.', '.', '.'])])
        } else {
            self
        }
    }

    pub fn build(&mut self) -> CorpusCleaner {
        use std::mem::take;

        let mut chars = take(&mut self.chars);
        let shifted_chars = take(&mut self.shifted_chars);
        let mut mappings = take(&mut self.mappings);

        if chars.remove(&' ') {
            mappings.insert(' ', vec![SPACE_CHAR]);
        }

        let map = chars
            .into_iter()
            .map(|c| (c, vec![c]))
            .chain(
                shifted_chars
                    .into_iter()
                    .map(|(from, to)| match self.shift_char {
                        Some(sc) => (from, vec![sc, to]),
                        None => (from, vec![to]),
                    }),
            )
            .chain(mappings)
            .collect();

        let shift_key = self.shift_char;
        let repeat_key = self.repeat_key;
        let raw = false;

        CorpusCleaner {
            map,
            shift_key,
            repeat_key,
            raw,
        }
    }
}

#[must_use = "iterators are lazy and do nothing unless consumed"]
#[derive(Debug)]
pub struct CorpusCleanerIterator<'a, I> {
    cleaner: &'a CorpusCleaner,
    iter: I,
    window: SlidingWindow<char>,
    shift_pressed: bool,
    use_window: bool,
}

impl<I> Iterator for CorpusCleanerIterator<'_, I>
where
    I: Iterator<Item = char>,
{
    type Item = Vec<char>;

    fn next(&mut self) -> Option<Self::Item> {
        let c = self.iter.next()?;
        if self.cleaner.raw {
            return Some(vec![c]);
        }

        if self.use_window {
            self.window.push(c);

            if self.cleaner.repeat_key && self.window[0] == self.window[1] {
                return Some(vec![REPEAT_KEY]);
            }
        }

        if let Some(sk) = self.cleaner.shift_key {
            // match self.cleaner.map.get(&c).map(|v| v.as_slice()) {
            //     Some(&[f]) if f == sk => Some(vec![REPLACEMENT_CHAR]),
            //     Some(&[f]) if self.shift_pressed => {
            //         self.shift_pressed = false;
            //         Some(vec![sk, f])
            //     }
            //     Some(&[f]) => Some(vec![f]),
            //     Some(&[f, c]) if f == sk && self.shift_pressed => Some(vec![c]),
            //     Some(&[f, c]) if self.shift_pressed => {
            //         self.shift_pressed = false;
            //         Some(vec![sk, f, c])
            //     }
            //     Some(&[f, c]) if f == sk => {
            //         self.shift_pressed = true;
            //         Some(vec![f, c])
            //     }
            //     Some(&[f, c]) => Some(vec![f, c]),
            //     Some(s @ &[f, ..]) if f == sk && self.shift_pressed => Some(s[1..].to_vec()),
            //     Some(s @ &[f, ..]) if f == sk => {
            //         self.shift_pressed = true;
            //         Some(s.to_vec())
            //     }
            //     Some(s) if self.shift_pressed => {
            //         self.shift_pressed = false;
            //         let mut res = vec![sk];
            //         res.extend(s);
            //         Some(res)
            //     }
            //     Some(s) => Some(s.to_vec()),
            //     _ => Some(vec![REPLACEMENT_CHAR]),
            // }
            match self.cleaner.map.get(&c).map(|v| v.as_slice()) {
                Some(&[f]) if f == sk => Some(vec![REPLACEMENT_CHAR]),
                Some(&[f]) if self.shift_pressed => {
                    self.shift_pressed = false;
                    Some(vec![f])
                }
                Some(&[f]) => Some(vec![f]),
                Some(&[f, c]) if f == sk && self.shift_pressed => Some(vec![c]),
                Some(&[f, c]) if self.shift_pressed => {
                    self.shift_pressed = false;
                    Some(vec![f, c])
                }
                Some(&[f, c]) if f == sk => {
                    self.shift_pressed = true;
                    Some(vec![f, c])
                }
                Some(&[f, c]) => Some(vec![f, c]),
                Some(s @ &[f, ..]) if f == sk && self.shift_pressed => Some(s[1..].to_vec()),
                Some(s @ &[f, ..]) if f == sk => {
                    self.shift_pressed = true;
                    Some(s.to_vec())
                }
                Some(s) if self.shift_pressed => {
                    self.shift_pressed = false;
                    Some(s.to_vec())
                }
                Some(s) => Some(s.to_vec()),
                _ => Some(vec![REPLACEMENT_CHAR]),
            }
        } else {
            match self.cleaner.map.get(&c) {
                None => Some(vec![REPLACEMENT_CHAR]),
                some => some.cloned(),
            }
        }
    }
}

pub trait CleanCorpus: Iterator {
    fn clean_corpus(
        self,
        cleaner: &CorpusCleaner,
    ) -> CorpusCleanerIterator<'_, impl Iterator<Item = char>>
    where
        Self: Iterator<Item = char>,
        Self: Sized,
    {
        let window = match cleaner.repeat_key {
            true => SlidingWindow::new(2, REPLACEMENT_CHAR),
            false => SlidingWindow::new(1, REPLACEMENT_CHAR),
        };
        let iter = self;

        CorpusCleanerIterator {
            cleaner,
            iter,
            window,
            shift_pressed: false,
            use_window: cleaner.repeat_key,
        }
    }
}

impl<I: Iterator> CleanCorpus for I where I: Iterator<Item = char> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean() {
        let corpus = "AABcd :dof:;";

        let cleaner = CorpusCleaner::builder()
            .with_chars("abcde".chars())
            .qwerty_punctuation_mappings(true)
            .build();

        let translation = corpus
            .chars()
            .clean_corpus(&cleaner)
            .flatten()
            .collect::<String>();

        for c in translation.chars() {
            println!("{c}");
        }
    }

    fn _gen_save_data(name: &str, cleaner: &CorpusCleaner) {
        let data = crate::data::Data::from_path(
            format!("../corpora/{name}"),
            &format!("{name}_no_space"),
            cleaner,
        )
        .expect("couldn't create data:");

        data.save("../data").expect("couldn't save data:");
    }

    // #[test]
    fn _generate_data() {
        let cleaner_ru = CorpusCleaner::builder()
            .with_chars("абвгдеёжзийклмнопрстуфхцчшщъыьэюя".chars())
            .qwerty_punctuation_mappings(true)
            .normalize_misc_punctuation(true)
            // .with_chars([' '])
            .build();

        _gen_save_data("russian", &cleaner_ru);

        let cleaner_de = CorpusCleaner::builder()
            .with_chars("abcdefghijklmnopqrstuvwxyzäöüß".chars())
            .qwerty_punctuation_mappings(true)
            .normalize_misc_punctuation(true)
            // .with_chars([' '])
            .build();

        _gen_save_data("german", &cleaner_de);

        let cleaner_fr = CorpusCleaner::builder()
            .with_chars("abcdefghijklmnopqrstuvwxyzéàçœâêîôûèìòùáíóúäëïöü".chars())
            .qwerty_punctuation_mappings(true)
            .normalize_misc_punctuation(true)
            // .with_chars([' '])
            .build();

        _gen_save_data("french", &cleaner_fr);

        let cleaner_no = CorpusCleaner::builder()
            .with_chars("abcdefghijklmnopqrstuvwxyzåøæ".chars())
            .qwerty_punctuation_mappings(true)
            .normalize_misc_punctuation(true)
            // .with_chars([' '])
            .build();

        _gen_save_data("bokmal", &cleaner_no);
        _gen_save_data("nynorsk", &cleaner_no);

        let cleaner_it = CorpusCleaner::builder()
            .with_chars("abcdefghijklmnopqrstuvwxyz".chars())
            .with_dead_key(
                [('à', 'a'), ('è', 'e'), ('ì', 'i'), ('ò', 'o'), ('ù', 'u')],
                '*',
            )
            .qwerty_punctuation_mappings(true)
            .normalize_misc_punctuation(true)
            // .with_chars([' '])
            .build();

        _gen_save_data("italian", &cleaner_it);

        let cleaner_en = CorpusCleaner::builder()
            .with_chars("abcdefghijklmnopqrstuvwxyz".chars())
            .qwerty_punctuation_mappings(true)
            .normalize_misc_punctuation(true)
            // .with_chars([' '])
            .build();

        _gen_save_data("english", &cleaner_en);
        _gen_save_data("dutch", &cleaner_en);

        let cleaner_sw = CorpusCleaner::builder()
            .with_chars("abcdefghijklmnopqrstuvwxyzäåö".chars())
            .qwerty_punctuation_mappings(true)
            .normalize_misc_punctuation(true)
            // .with_chars([' '])
            .build();

        _gen_save_data("finnish", &cleaner_sw);
        _gen_save_data("swedish", &cleaner_sw);
    }
}
