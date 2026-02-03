use indexmap::IndexMap;

use crate::REPLACEMENT_CHAR;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharMapping(IndexMap<char, u8>);

impl Default for CharMapping {
    fn default() -> Self {
        let mut mapping = Self(Default::default());

        mapping.push(REPLACEMENT_CHAR);
        // mapping.push(SHIFT_CHAR);
        // mapping.push(SPACE_CHAR);

        mapping
    }
}

impl CharMapping {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_single(&self, u: u8) -> char {
        match self.0.get_index(u as usize) {
            Some((c, _)) => *c,
            None => REPLACEMENT_CHAR,
        }
    }

    pub fn to_single(&mut self, c: char) -> u8 {
        if let Some(u) = self.0.get(&c) {
            *u
        } else {
            let new = self.len();
            self.0.insert(c, new);
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
        if let Some(u) = self.0.get(&c) {
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

    pub fn push(&mut self, c: char) {
        let new = self.len();
        if !self.0.contains_key(&c) {
            self.0.insert(c, new);
        }
    }

    pub fn get_u(&self, c: char) -> u8 {
        match self.0.get(&c) {
            Some(c) => *c,
            None => 0,
        }
    }

    pub fn get_c(&self, u: u8) -> char {
        match self.0.get_index(u as usize) {
            Some((c, _)) => *c,
            None => REPLACEMENT_CHAR,
        }
    }

    pub fn insert<T>(&mut self, input: T)
    where
        T: IntoIterator<Item = char>,
    {
        input.into_iter().for_each(|c| self.push(c));
    }

    pub fn as_str(&self, input: &[u8]) -> String {
        input.iter().map(|&u| self.from_single(u)).collect()
    }

    pub fn map_cs<'a>(&'a mut self, s: &'a str) -> impl Iterator<Item = u8> + 'a {
        s.chars().map(|c| self.to_single(c))
    }

    pub fn map_us<'a>(&'a self, u: &'a [u8]) -> impl Iterator<Item = char> + 'a {
        u.iter().map(|u| self.from_single(*u))
    }

    pub fn len(&self) -> u8 {
        self.0.len() as u8
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl From<&str> for CharMapping {
    fn from(value: &str) -> Self {
        Self::from_iter(value.chars())
    }
}

impl From<String> for CharMapping {
    fn from(value: String) -> Self {
        Self::from_iter(value.chars())
    }
}

impl<const N: usize> From<[char; N]> for CharMapping {
    fn from(arr: [char; N]) -> Self {
        arr.into_iter().collect()
    }
}

impl From<&[char]> for CharMapping {
    fn from(slice: &[char]) -> Self {
        slice.iter().collect()
    }
}

impl FromIterator<char> for CharMapping {
    fn from_iter<T: IntoIterator<Item = char>>(iter: T) -> Self {
        let mut res = Self::new();

        for c in iter {
            res.push(c)
        }

        res
    }
}

impl<'a> FromIterator<&'a char> for CharMapping {
    fn from_iter<T: IntoIterator<Item = &'a char>>(iter: T) -> Self {
        iter.into_iter().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_from() {
        let mapping_s = "abcdefhgijklmnopqrstuvwxyz ";
        let mut mapping = mapping_s.chars().collect::<CharMapping>();

        assert_eq!(mapping.len() as usize, mapping_s.len() + 1);

        let s = "this is epic-";
        let u = mapping.map_cs(s).collect::<Vec<_>>();
        let c = mapping.map_us(&u).collect::<String>();

        // assert_eq!(c, "this is epic�")
        assert_eq!(c, "this is epic-")
    }
}
