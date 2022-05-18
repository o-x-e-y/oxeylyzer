use core::str::Chars;
use std::iter::FromIterator;

pub trait Characters<'a> {
    type T: Iterator<Item = char>;
    fn characters(&'a self) -> Self::T;
}

impl<'a> Characters<'a> for &'a str {
    type T = Chars<'a>;

    fn characters(&'a self) -> Self::T {
        self.chars()
    }
}

impl<'a> Characters<'a> for String {
    type T = Chars<'a>;

    fn characters(&'a self) -> Self::T {
        self.chars()
    }
}

pub trait Bigrams<'a> : Characters<'a> {
    fn bigrams(&'a self) -> Vec<String> {
        if self.characters().count() < 2 {
            return Vec::new();
        }

        let it_1 = self.characters();
        let mut it_2 = self.characters();
        it_2.next().unwrap();

        let res = it_1
            .zip(it_2)
            .map(|(a, b): (char, char)| String::from_iter([a, b]))
            .collect::<Vec<String>>();
        res
    }

    fn bigrams_unchecked(&'a self) -> Vec<String> {
        let it_1 = self.characters();
        let mut it_2 = self.characters();
        it_2.next().unwrap();

        let res = it_1
            .zip(it_2)
            .map(|(a, b): (char, char)| String::from_iter([a, b]))
            .collect::<Vec<String>>();
        res
    }
}

impl<'a> Bigrams<'a> for &'a str {}
impl<'a> Bigrams<'a> for String {}

pub trait Skipgrams<'a> : Characters<'a> {
    fn skipgrams(&'a self) -> Vec<String> {
        if self.characters().count() < 3 {
            return Vec::new();
        }

        let it_1 = self.characters();
        let mut it_3 = self.characters();
        it_3.next().unwrap();
        it_3.next().unwrap();

        let res = it_1
            .zip(it_3)
            .map(|(a, c): (char, char)| String::from_iter([a, c]))
            .collect::<Vec<String>>();
        res
    }

    fn skipgrams_unchecked(&'a self) -> Vec<String> {
        let it_1 = self.characters();
        let mut it_3 = self.characters();
        it_3.next().unwrap();
        it_3.next().unwrap();

        let res = it_1
            .zip(it_3)
            .map(|(a, c): (char, char)| String::from_iter([a, c]))
            .collect::<Vec<String>>();
        res
    }
}

impl<'a> Skipgrams<'a> for &'a str {}
impl<'a> Skipgrams<'a> for String {}

pub trait Trigrams<'a> : Characters<'a> {
    fn trigrams(&'a self) -> Vec<String> {
        if self.characters().count() < 3 {
            return Vec::new();
        }

        let it_1 = self.characters();
        let mut it_2 = self.characters();
        it_2.next().unwrap();
        let mut it_3 = self.characters();
        it_3.next().unwrap();
        it_3.next().unwrap();

        let res = it_1
            .zip(it_2)
            .zip(it_3)
            .map(|((a, b), c): ((char, char), char)| String::from_iter([a, b, c]))
            .collect::<Vec<String>>();
        res
    }

    fn trigrams_unchecked(&'a self) -> Vec<String> {
        let it_1 = self.characters();
        let mut it_2 = self.characters();
        it_2.next().unwrap();
        let mut it_3 = self.characters();
        it_3.next().unwrap();
        it_3.next().unwrap();

        let res = it_1
            .zip(it_2)
            .zip(it_3)
            .map(|((a, b), c): ((char, char), char)| String::from_iter([a, b, c]))
            .collect::<Vec<String>>();
        res
    }
}

impl<'a> Trigrams<'a> for &'a str {}
impl<'a> Trigrams<'a> for String {}


pub trait BigramsArr<'a> : Characters<'a> {
    fn bigrams_arr(&'a self) -> Vec<[char; 2]> {
        if self.characters().count() < 2 {
            return Vec::new();
        }

        let it_1 = self.characters();
        let mut it_2 = self.characters();
        it_2.next().unwrap();

        let res = it_1
            .zip(it_2)
            .map(|(a, b): (char, char)| [a, b])
            .collect::<Vec<[char; 2]>>();
        res
    }

    fn bigrams_arr_unchecked(&'a self) -> Vec<[char; 2]> {
        let it_1 = self.characters();
        let mut it_2 = self.characters();
        it_2.next().unwrap();

        let res = it_1
            .zip(it_2)
            .map(|(a, b): (char, char)| [a, b])
            .collect::<Vec<[char; 2]>>();
        res
    }
}

impl<'a> BigramsArr<'a> for &'a str {}
impl<'a> BigramsArr<'a> for String {}

pub trait TrigramsArr<'a> : Characters<'a> {
    fn trigrams_arr(&'a self) -> Vec<[char; 3]> {
        if self.characters().count() < 3 {
            return Vec::new();
        }

        let it_1 = self.characters();
        let mut it_2 = self.characters();
        it_2.next().unwrap();
        let mut it_3 = self.characters();
        it_3.next().unwrap();
        it_3.next().unwrap();

        let res = it_1
            .zip(it_2)
            .zip(it_3)
            .map(|((a, b), c): ((char, char), char) | [a, b, c])
            .collect::<Vec<[char; 3]>>();
        res
    }

    fn trigrams_arr_unchecked(&'a self) -> Vec<[char; 3]> {
        let it_1 = self.characters();
        let mut it_2 = self.characters();
        it_2.next().unwrap();
        let mut it_3 = self.characters();
        it_3.next().unwrap();
        it_3.next().unwrap();

        let res = it_1
            .zip(it_2)
            .zip(it_3)
            .map(|((a, b), c): ((char, char), char) | [a, b, c])
            .collect::<Vec<[char; 3]>>();
        res
    }
}

impl<'a> TrigramsArr<'a> for &'a str {}
impl<'a> TrigramsArr<'a> for String {}