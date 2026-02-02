use std::collections::hash_map::Entry;

use ahash::AHashMap as HashMap;

#[derive(Clone, Debug, Default)]
pub struct CharMapping {
    from: Vec<char>,
    to: HashMap<char, u8>,
}

impl CharMapping {
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
