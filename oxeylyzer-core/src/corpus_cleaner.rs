use ahash::{AHashMap as HashMap, AHashSet as HashSet};

use sliding_window_alt::SlidingWindow;
use thiserror::Error;

use crate::*;

/// Errors that can occur during corpus cleaning.
#[derive(Debug, Clone, Error)]
pub enum CorpusError {}

/// A cleaner that transforms raw corpus text into a format suitable for analysis.
///
/// It handles character mappings, shift keys, and repeat keys.
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
    /// Creates a new [`CorpusCleanerBuilder`] to configure a cleaner.
    ///
    /// # Examples:
    /// ```
    /// use oxeylyzer_core::corpus_cleaner::CorpusCleaner;
    ///
    /// let builder = CorpusCleaner::builder();
    /// ```
    pub fn builder() -> CorpusCleanerBuilder {
        CorpusCleanerBuilder {
            shift_char: Some(SHIFT_CHAR),
            chars: HashSet::default(),
            shifted_chars: HashMap::default(),
            mappings: HashMap::default(),
            repeat_key: false,
        }
    }

    /// Creates a raw `CorpusCleaner` that performs no transformations.
    ///
    /// # Examples:
    /// ```
    /// use oxeylyzer_core::corpus_cleaner::CorpusCleaner;
    ///
    /// let cleaner = CorpusCleaner::raw();
    /// assert!(cleaner.is_raw());
    /// ```
    pub fn raw() -> Self {
        Self::default()
    }

    /// Returns the shift key character if one is configured.
    ///
    /// # Examples:
    /// ```
    /// use oxeylyzer_core::corpus_cleaner::CorpusCleaner;
    ///
    /// let cleaner = CorpusCleaner::raw();
    /// assert_eq!(cleaner.shift_key(), Some('⇑'));
    /// ```
    pub fn shift_key(&self) -> Option<char> {
        self.shift_key
    }

    /// Returns true if a repeat key is enabled.
    ///
    /// # Examples:
    /// ```
    /// use oxeylyzer_core::corpus_cleaner::CorpusCleaner;
    ///
    /// let cleaner = CorpusCleaner::raw();
    /// assert!(!cleaner.repeat_key());
    /// ```
    pub fn repeat_key(&self) -> bool {
        self.repeat_key
    }

    /// Returns true if this is a raw cleaner.
    ///
    /// # Examples:
    /// ```
    /// use oxeylyzer_core::corpus_cleaner::CorpusCleaner;
    ///
    /// let cleaner = CorpusCleaner::raw();
    /// assert!(cleaner.is_raw());
    /// ```
    pub fn is_raw(&self) -> bool {
        self.raw
    }
}

/// Builder for creating a [`CorpusCleaner`].
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

    /// Adds mappings for uppercase characters.
    ///
    /// # Examples:
    /// ```
    /// use oxeylyzer_core::corpus_cleaner::CorpusCleaner;
    ///
    /// let cleaner = CorpusCleaner::builder()
    ///     .with_uppercase_mappings([('a', 'A')])
    ///     .build();
    /// ```
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

            if upper.clone().count() == 1 {
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

    /// Adds a set of characters to be included in the cleaned corpus.
    ///
    /// # Examples:
    /// ```
    /// use oxeylyzer_core::corpus_cleaner::CorpusCleaner;
    ///
    /// let cleaner = CorpusCleaner::builder()
    ///     .with_chars("abc".chars())
    ///     .build();
    /// ```
    pub fn with_chars(&mut self, chars: impl IntoIterator<Item = char>) -> &mut Self {
        chars.into_iter().for_each(|c| self.with_char(c));

        self
    }

    /// Enables or disables the repeat key.
    ///
    /// # Examples:
    /// ```
    /// use oxeylyzer_core::corpus_cleaner::CorpusCleaner;
    ///
    /// let cleaner = CorpusCleaner::builder()
    ///     .repeat_key(true)
    ///     .build();
    /// assert!(cleaner.repeat_key());
    /// ```
    pub fn repeat_key(&mut self, enable: bool) -> &mut Self {
        self.repeat_key = enable;

        self
    }

    /// Sets the character used to represent a shift key press.
    ///
    /// # Examples:
    /// ```
    /// use oxeylyzer_core::corpus_cleaner::CorpusCleaner;
    ///
    /// let cleaner = CorpusCleaner::builder()
    ///     .shift_char(Some('^'))
    ///     .build();
    /// assert_eq!(cleaner.shift_key(), Some('^'));
    /// ```
    pub fn shift_char(&mut self, shift_char: Option<char>) -> &mut Self {
        self.shift_char = shift_char;

        self
    }

    fn with_mapping(&mut self, (from, to): (char, Vec<char>)) {
        self.mappings.insert(from, to);
    }

    /// Adds complex mappings from one character to multiple characters.
    ///
    /// # Examples:
    /// ```
    /// use oxeylyzer_core::corpus_cleaner::CorpusCleaner;
    ///
    /// let cleaner = CorpusCleaner::builder()
    ///     .with_mappings([('æ', vec!['a', 'e'])])
    ///     .build();
    /// ```
    pub fn with_mappings(
        &mut self,
        mappings: impl IntoIterator<Item = (char, Vec<char>)>,
    ) -> &mut Self {
        mappings.into_iter().for_each(|ft| self.with_mapping(ft));

        self
    }

    /// Adds simple character-to-character mappings.
    ///
    /// # Examples:
    /// ```
    /// use oxeylyzer_core::corpus_cleaner::CorpusCleaner;
    ///
    /// let cleaner = CorpusCleaner::builder()
    ///     .with_char_mappings([('A', 'a')])
    ///     .build();
    /// ```
    pub fn with_char_mappings(
        &mut self,
        mappings: impl IntoIterator<Item = (char, char)>,
    ) -> &mut Self {
        mappings
            .into_iter()
            .for_each(|(from, to)| self.with_mapping((from, vec![to])));

        self
    }

    /// Adds mappings where a character maps exactly to itself.
    ///
    /// # Examples:
    /// ```
    /// use oxeylyzer_core::corpus_cleaner::CorpusCleaner;
    ///
    /// let cleaner = CorpusCleaner::builder()
    ///     .with_exact_mappings(['a', 'b'])
    ///     .build();
    /// ```
    pub fn with_exact_mappings(&mut self, chars: impl IntoIterator<Item = char>) -> &mut Self {
        chars
            .into_iter()
            .for_each(|c| self.with_mapping((c, vec![c])));

        self
    }

    /// Adds mappings that involve a dead key followed by another character.
    ///
    /// # Examples:
    /// ```
    /// use oxeylyzer_core::corpus_cleaner::CorpusCleaner;
    ///
    /// let cleaner = CorpusCleaner::builder()
    ///     .with_dead_key([('á', 'a')], '´')
    ///     .build();
    /// ```
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

    /// Adds standard QWERTY punctuation mappings (e.g., '!' maps to Shift + '1').
    ///
    /// # Examples:
    /// ```
    /// use oxeylyzer_core::corpus_cleaner::CorpusCleaner;
    ///
    /// let cleaner = CorpusCleaner::builder()
    ///     .qwerty_punctuation_mappings(true)
    ///     .build();
    /// ```
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

    /// Adds normalization for various miscellaneous punctuation marks (e.g., smart quotes to straight quotes).
    ///
    /// # Examples:
    /// ```
    /// use oxeylyzer_core::corpus_cleaner::CorpusCleaner;
    ///
    /// let cleaner = CorpusCleaner::builder()
    ///     .normalize_misc_punctuation(true)
    ///     .build();
    /// ```
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

    /// Builds the [`CorpusCleaner`].
    ///
    /// # Examples:
    /// ```
    /// use oxeylyzer_core::corpus_cleaner::CorpusCleaner;
    ///
    /// let cleaner = CorpusCleaner::builder().build();
    /// ```
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

/// An iterator that cleans corpus text using a [`CorpusCleaner`].
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
            let last = self.window[0];

            if self.cleaner.repeat_key && c == last && self.cleaner.map.contains_key(&c) {
                self.window.push(REPEAT_KEY);
                return Some(vec![REPEAT_KEY]);
            } else {
                self.window.push(c)
            }
        }

        if let Some(sk) = self.cleaner.shift_key {
            match self.cleaner.map.get(&c).map(|v| v.as_slice()) {
                Some(&[f]) if f == sk => Some(vec![REPLACEMENT_CHAR]),
                Some(&[REPEAT_KEY]) => Some(vec![REPEAT_KEY]),
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

/// Trait providing the `clean_corpus` method to any character iterator.
pub trait CleanCorpus: Iterator {
    /// Wraps the iterator with a [`CorpusCleanerIterator`].
    ///
    /// # Examples:
    /// ```
    /// use oxeylyzer_core::corpus_cleaner::{CorpusCleaner, CleanCorpus};
    ///
    /// let cleaner = CorpusCleaner::raw();
    /// let cleaned: String = "abc".chars().clean_corpus(&cleaner).flatten().collect();
    /// assert_eq!(cleaned, "abc");
    /// ```
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

        assert_eq!(translation, "⇑aabcd�⇑;d��⇑;;");
    }

    #[test]
    fn repeat_key() {
        let corpus = "AAAAAaBcd :dof:;";

        let cleaner = CorpusCleaner::builder()
            .with_chars("abcde".chars())
            .qwerty_punctuation_mappings(true)
            .repeat_key(true)
            .build();

        let translation = corpus
            .chars()
            .clean_corpus(&cleaner)
            .flatten()
            .collect::<String>();

        assert_eq!(
            translation,
            format!("⇑a{}a{}aa⇑bcd�⇑;d��⇑;;", REPEAT_KEY, REPEAT_KEY)
        );
    }

    #[test]
    fn compare_repeat_key() {
        use fancy_regex::{Captures, Regex};

        let path = concat!(
            std::env!("CARGO_MANIFEST_DIR"),
            "/../static/text/monkeyracer/mr.txt"
        );
        let monkeyracer = std::fs::read_to_string(path)
            .unwrap()
            .chars()
            .take(100_000)
            .collect::<String>();

        let re = Regex::new(r"(.)\1").unwrap();
        let monkeyracer_repeat = re.replace_all(&monkeyracer, |caps: &Captures| {
            format!("{}{}", &caps[1], REPEAT_KEY)
        });

        let vanilla_cleaner = CorpusCleaner::builder()
            .normalize_misc_punctuation(true)
            .qwerty_punctuation_mappings(true)
            .with_exact_mappings([REPEAT_KEY])
            .with_chars("abcdefghijklmnopqrstuvwxyz".chars())
            .build();

        let repeat_cleaner = CorpusCleaner {
            map: vanilla_cleaner.map.clone(),
            shift_key: vanilla_cleaner.shift_key,
            repeat_key: true,
            raw: false,
        };

        let repeat_cleaned_monkeyracer = monkeyracer
            .chars()
            .clean_corpus(&repeat_cleaner)
            .flatten()
            .collect::<String>();

        let cleaned_repeat_monkeyracer = monkeyracer_repeat
            .chars()
            .clean_corpus(&vanilla_cleaner)
            .flatten()
            .collect::<String>();

        assert_eq!(
            repeat_cleaned_monkeyracer.len(),
            cleaned_repeat_monkeyracer.len()
        );

        repeat_cleaned_monkeyracer
            .chars()
            .zip(cleaned_repeat_monkeyracer.chars())
            .zip(0usize..)
            .for_each(|((mr, rpt), i)| {
                if mr != rpt {
                    let cleaned_mr_context = repeat_cleaned_monkeyracer
                        .chars()
                        .skip(i.saturating_sub(10))
                        .take(20)
                        .collect::<String>();

                    let cleaned_rpt_context = cleaned_repeat_monkeyracer
                        .chars()
                        .skip(i.saturating_sub(10))
                        .take(20)
                        .collect::<String>();

                    println!("repeat basic context:  {cleaned_mr_context}");
                    println!("manual repeat context: {cleaned_rpt_context}");
                    panic!()
                }
            })
    }
}
