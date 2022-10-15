use std::collections::HashMap;
use std::borrow::Cow;
use anyhow::Result;

pub struct Translator<'a> {
    pub table: HashMap<char, Cow<'a, str>>,
    pub is_raw: bool,
    pub(crate) ignore_unknown: bool,
    pub(crate) is_empty: bool,
    pub(crate) multiple_val: f64
}

impl<'a> Default for Translator<'a> {
    fn default() -> Self {
        Translator::new()
		    .default_formatting()
		    .build()
    }
}

impl<'a> std::ops::Add for Translator<'a> {
    type Output = Self;

    ///the table of the FIRST argument takes priority over the SECOND.
    fn add(mut self, rhs: Self) -> Self::Output {
        self.is_empty |= rhs.is_empty;
        if !self.is_empty {
            self.is_raw |= rhs.is_raw;
            self.ignore_unknown |= rhs.ignore_unknown;
            self.multiple_val = (self.multiple_val + rhs.multiple_val) / 2.0;

            let base = &Cow::from(" ");
            for (from, to) in rhs.table {
                let original = self.table.get(&from);
                if original.is_none() || original == Some(base) {
                    self.table.insert(from, to);
                }
            }
        }
        self
    }
}

impl<'a> Translator<'a> {
    pub fn new() -> TranslatorBuilder<'a> {
        TranslatorBuilder {
            table: HashMap::new(),
            is_raw: false,
            ignore_unknown: false
        }
    }

    pub fn language(language: &'a str) -> Result<Self> {
        Ok(Self::new()
            .language(language)?
            .build())
    }

    pub fn language_or_default(language: &'a str) -> Self {
        if let Ok(t) = Self::language(language) {
            t
        } else {
            Self::default()
        }
    }

    pub fn language_or_raw(language: &'a str) -> Self {
        if let Ok(t) = Self::language(language) {
            t
        } else {
            Self::raw()
        }
    }

    pub fn raw() -> Self {
        Translator::new()
            .raw()
            .ascii_lower()
            .normalize_punct()
            .keep_unknown()
            .build()
    }

    pub fn translate(&self, s: &str) -> Cow<'a, str> {
        let mut res: String;

        if self.is_empty {
            return Cow::from(s);
        } else if self.multiple_val == 0.0 {
            res = String::with_capacity(s.len()); 
        } else {
            let f64_len = s.len() as f64;
            let length = f64_len + f64_len / (0.025 * self.multiple_val);
            res = String::with_capacity(length as usize);
        }

        if self.ignore_unknown {
            for c in s.chars() {
                if let Some(replacement) = self.table.get(&c) {
                    res.push_str(replacement);
                } else {
                    res.push(c);
                }
            }
        } else {
            for c in s.chars() {
                if let Some(replacement) = self.table.get(&c) {
                    res.push_str(replacement);
                } else  {
                    res.push(' ');
                }
            }
        }

        res.shrink_to_fit();
        Cow::from(res)
	}
}

pub struct TranslatorBuilder<'a> {
    table: HashMap<char, Cow<'a, str>>,
    is_raw: bool,
    ignore_unknown: bool
}

impl<'a> TranslatorBuilder<'a> {
    pub fn keep_unknown(&mut self) -> &mut Self {
        self.ignore_unknown = false;
        self
    }

    pub fn to_nothing(&mut self, to_nothing: &str) -> &mut Self {
        for c in to_nothing.chars() {
            self.table.insert(c, Cow::from(""));
        }
        self
    }

    pub fn to_space(&mut self, to_string: &str) -> &mut Self {
        for c in to_string.chars() {
            self.table.insert(c, Cow::from(" "));
        }
        self
    }

    pub fn many_different_to_one(&mut self, from: &str, to: char) -> &mut Self {
        for c in from.chars() {
            self.table.insert(c, Cow::from(to.to_string()));
        }
        self
    }

    pub fn keep(&mut self, keep: &str) -> &mut Self {
        for c in keep.chars() {
            self.table.insert(c, Cow::from(c.to_string()));
        }
        self
    }

    pub fn one_to_one(&mut self, from: &str, to: &str) -> &mut Self {
        assert_eq!(from.chars().count(), to.chars().count());

        for (s, d) in from.chars().zip(to.chars()) {
            self.table.insert(s, Cow::from(d.to_string()));
        }
        self
    }

    pub fn one_multiple(&mut self, from: char, to: &'static str) -> &mut Self {
        self.table.insert(from, Cow::from(to));
        self
    }

    pub fn to_multiple(&mut self, trans: Vec<(char, &'static str)>) -> &mut Self {
        for (s, d) in trans {
            self.table.insert(s, Cow::from(d));
        }
        self
    }

    pub fn letters_to_lowercase(&mut self, letters: &str) -> &mut Self {
        for letter in letters.chars() {
            self.table.insert(letter, Cow::from(letter.to_string()));

            let mut upper_string = letter.to_uppercase();

            if upper_string.clone().count() == 1 {
                let uppercase_letter = upper_string.next().unwrap();
                
                let shifted = String::from_iter([' ', letter]);
                self.one_multiple(uppercase_letter, shifted.as_str());
            }
        }
        self
    }

    pub fn raw(&mut self) -> &mut Self {
        let mut letters = String::new();
        for i in 128u32..66_666 {
            if let Some(c) = char::from_u32(i)
            && c.is_alphabetic() {
                letters.push(c);
            }
        }

        self.is_raw = true;

        self
            .letters_to_lowercase(letters.as_str())
            .alphabet_lower()
            .punct_lower()
            .normalize_punct()
    }

    pub fn custom_unshift(&mut self, upper_version: &str, lower_version: &str) -> &mut Self {
        for (upper, lower) in upper_version.chars().zip(lower_version.chars()) {
            let shifted = String::from_iter([' ', lower]);
            self.one_multiple(upper, shifted.as_str());
        }

        self
            .keep(lower_version)
    }

    pub fn punct_lower(&mut self) -> &mut Self {
        for (upper, lower) in "{}?+_|\"<>:~".chars().zip("[]/=-\\',.;`".chars()) {
            let shifted = String::from_iter([' ', lower]);
            self.one_multiple(upper, shifted.as_str());
        }

        self
            .keep("[]/=-\\',.;`")
    }

    pub fn alphabet_lower(&mut self) -> &mut Self {
        self.letters_to_lowercase("abcdefghijklmnopqrstuvwxyz")
    }

    pub fn number_symbols_lower(&mut self) -> &mut Self {
        self.one_to_one("!@#$%^&*()", "1234567890")
    }

    pub fn ascii_lower(&mut self) -> &mut Self {
        self
            .punct_lower()
            .alphabet_lower()
    }

    pub fn normalize_punct(&mut self) -> &mut Self {
        self
            .one_to_one("«´»÷‘“”’–ʹ͵","'''/''''-''")
            .one_multiple('…', "...")
    }

    pub fn default_formatting(&mut self) -> &mut Self {
        self
            .ascii_lower()
            .normalize_punct()
    }

    pub fn language(&mut self, language: &str) -> Result<&mut Self> {
        self.default_formatting();
        match language.to_lowercase().as_str() {
            "akl" | "english" | "english2" | "toki_pona" | "indonesian" | "reddit" => Ok(self),
            "albanian" => Ok(self
                .letters_to_lowercase("çë")
            ),
            "bokmal" | "nynorsk" | "danish" => Ok(self
                .letters_to_lowercase("åøæ")
            ),
            "czech" => Ok(self
                .to_multiple(vec![
                    ('č', "*c"), ('ď', "*d"), ('é', "*x"), ('ň', "*n"), ('ó', "*o"), ('ř', "*r"),
                    ('š', "*s"), ('ť', "*t"), ('ů', "*u"), ('ú', "*b"), ('ý', "*y"), ('ž', "*z"),
                    ('Č', "*c"), ('Ď', "*d"), ('É', "*x"), ('Ň', "*n"), ('Ó', "*o"), ('Ř', "*r"),
                    ('Š', "*s"), ('Ť', "*t"), ('Ů', "*u"), ('Ú', "*b"), ('Ý', "*y"), ('Ž', "*z")
                ])
                .letters_to_lowercase("áíě")
            ),
            "dan-en70-30" => Ok(self
                .letters_to_lowercase("åøæ")
            ),
            "dan-en70-30a" => Ok(self
                .to_multiple(vec![
                    ('å', "*a"), ('ø', "*o"), ('æ', "*e")
                ])
            ),
            "dutch" => Ok(self.letters_to_lowercase("áèéçëíîó")),
            "dutch_repeat" => Ok(self.letters_to_lowercase("áèéçëíîó@")),
            "english_repeat" => Ok(self.keep("@")),
            "english_th" => Ok(self.letters_to_lowercase("þ")),
            "esperanto" => Ok(self
                .letters_to_lowercase("ŝĝĉŭĵĥ")
            ),
            "finnish" => Ok(self
                .letters_to_lowercase("åäö")
            ),
            "finnish_repeat" => Ok(self
                .letters_to_lowercase("åäö@")
            ),
            "french" | "french_qu" | "test" => Ok(self
                .to_multiple(vec![
                    ('ç', "*c"), ('Ç', "*c"), ('œ', "oe"),    ('á', "* a"), ('â', "* a"), ('è', "* e"),
                    ('ê', "* e"), ('ì', "* i"), ('í', "* i"), ('î', "* i"), ('ò', "* o"), ('ó', "* o"),
                    ('ô', "* o"), ('ù', "* u"), ('ú', "* u"), ('û', "* u"), ('Á', "* a"), ('Â', "* a"),
                    ('È', "* e"), ('Ê', "* e"), ('Ì', "* i"), ('Í', "* i"), ('Î', "* i"), ('Ò', "* o"),
                    ('Ó', "* o"), ('Ô', "* o"), ('Ù', "* u"), ('Ú', "* u"), ('Û', "* u"), ('ä', "* a"),
                    ('ë', "* e"), ('ï', "* i"), ('ö', "* o"), ('ü', "* u"), ('Ä', "* a"), ('Ë', "* e"),
                    ('Ï', "* i"), ('Ö', "* o"), ('Ü', "* u")
                ])
                .letters_to_lowercase("éà")
            ),
            "german" => Ok(self.letters_to_lowercase("äöüß")),
            "hungarian" => Ok(self
                .to_multiple(vec![
                    ('í', "*i"), ('ü', "*u"), ('ú', "* u"), ('ű', "* u"), ('Í', "*i"), ('Ü', "*u"),
                    ('Ú', "* u"), ('Ű', "* u")
                ])
                .letters_to_lowercase("áéöóő")
            ),
            "italian" => Ok(self
                .to_multiple(vec![
                    ('à', "*a"), ('è', "*e"), ('ì', "*i"), ('ò', "*o"), ('ù', "*u"), ('À', "*a"),
                    ('È', "*e"), ('Ì', "*i"), ('Ò', "*o"), ('Ù', "*u")
                ])
            ),
            "korean" => Ok(self
                .to_space("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ")
                .keep("ㅣㅡㅜㅏㅊㅈㅅㅂㅁㄹㄷㄴㄱㅇㅋㅌㅍㅐㅑㅓㅕㅗㅎㅔㅛㅠ")
                .one_to_one("ㄲㄸㅆㅃㅉㅒㅖ", "ㄱㄷㅅㅂㅈㅐㅔ")
                .to_multiple(vec![
                    ('ㄳ', "ㄱㅅ"), ('ㅥ', "ㄴㄴ"), ('ㅦ', "ㄴㄷ"), ('ㅧ', "ㄴㅅ"), ('ㄵ', "ㄴㅈ"),
                    ('ㄶ', "ㄴㅎ"), ('ㄺ', "ㄹㄱ"), ('ㅩ', "ㄹㄱㅅ"), ('ㅪ', "ㄹㄷ"), ('ㄻ', "ㄹㅁ"),
                    ('ㄼ', "ㄹㅂ"), ('ㅫ', "ㄹㅂㅅ"), ('ㄽ', "ㄹㅅ"), ('ㄾ', "ㄹㅌ"), ('ㄿ', "ㄹㅍ"),
                    ('ㅀ', "ㄹㅎ"), ('ㅮ', "ㅁㅂ"), ('ㅯ', "ㅁㅅ"), ('ㅲ', "ㅂㄱ"), ('ㅳ', "ㅂㄷ"),
                    ('ㅄ', "ㅂㅅ"), ('ㅴ', "ㅂㅅㄱ"), ('ㅵ', "ㅂㅅㄷ"), ('ㅶ', "ㅂㅈ"), ('ㅷ', "ㅂㅌ"),
                    ('ㅹ', "ㅂㅂ"), ('ㅺ', "ㅅㄱ"), ('ㅻ', "ㅅㄴ"), ('ㅼ', "ㅅㄷ"), ('ㅽ', "ㅅㅂ"),
                    ('ㅾ', "ㅅㅈ"), ('ㆀ', "ㅇㅇ"), ('ㆄ', "ㅍ"), ('ㆅ', "ㅎㅎ"), ('ㅘ', "ㅗㅏ"),
                    ('ㅙ', "ㅗㅐ"), ('ㅚ', "ㅗㅣ"), ('ㆇ', "ㅛㅑ"), ('ㆈ', "ㅛㅐ"), ('ㆉ', "ㅛㅣ"),
                    ('ㅝ', "ㅜㅓ"), ('ㅞ', "ㅜㅔ"), ('ㅟ', "ㅜㅣ"), ('ㆊ', "ㅠㅖ"), ('ㆋ', "ㅠㅖ"),
                    ('ㆌ', "ㅠㅣ"), ('ㅢ', "ㅡㅣ"), ('ㅸ', "ㅂ"), ('ㅱ', "ㅁ")
                ])
            ),
            "luxembourgish" => Ok(self
                .to_multiple(vec![
                    ('œ', " "), ('e', " ´"), ('u', " ¨"), ('i', " ˆ"), ('s', " ß"), ('d', " ∂"),
                    ('c', " ç")
                ])
            ),
            "polish" => Ok(self
                .to_multiple(vec![
                    ('ą', "*a"), ('ó', "*o"), ('ź', "*z"), ('ś', "*s"), ('ć', "*c"), ('ń', "*n")
                ])
                .letters_to_lowercase("łęż")
            ),
            "russian" => Ok(self
                .letters_to_lowercase("абвгдеёжзийклмнопрстуфхцчшщъыьэюя")
                .to_space("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ")
            ),
            "spanish" => Ok(self
                .to_multiple(vec![
                    ('á', "*a"), ('é', "*e"), ('í', "*i"), ('ó', "*o"), ('ú', "*u"), ('ü', "*y"),
                    ('Á', "*a"), ('É', "*e"), ('Í', "*i"), ('Ó', "*o"), ('Ú', "*u"), ('Ü', "*y"),
                    ('ñ', "*n"), ('Ñ', "*n")
                ])
            ),
            "swedish" => Ok(
                self.letters_to_lowercase("äåö")
            ),
            "welsh" => Ok(self
                .to_multiple(vec![
                    ('â', "*a"), ('ê', "*e"), ('î', "*i"), ('ô', "*o"), ('û', "*u"), ('ŵ', "*w"),
                    ('ŷ', "*y"), ('Â', "*a"), ('Ê', "*e"), ('Î', "*i"), ('Ô', "*o"), ('Û', "*u"),
                    ('Ŵ', "*w"), ('Ŷ', "*y")
                ])
                .letters_to_lowercase("ΔⳐ")
            ),
            "welsh_pure" => Ok(self
                .to_multiple(vec![
                    ('â', "*a"), ('ê', "*e"), ('î', "*i"), ('ô', "*o"), ('û', "*u"), ('ŵ', "*w"),
                    ('ŷ', "*y"), ('Â', "*a"), ('Ê', "*e"), ('Î', "*i"), ('Ô', "*o"), ('Û', "*u"),
                    ('Ŵ', "*w"), ('Ŷ', "*y")
                ])
            ),
            _ => Err(anyhow::format_err!("This language is not available. You'll have to make your own formatter, sorry!"))
        }
    }

    fn check_multiple_val(&self) -> f64 {
        // assume a 2.5% occurence of every 1 -> n translation to be safe
        // subtract from total length with a factor of 0.1 in case of a 1 -> 0 translation

        let mut res = 0.0;
        for (_, trans) in self.table.iter() {
            if trans.len() > 0 {
                res += trans.len() as f64 - 1.0;
            } else {
                res -= 0.1;
            }
        }
        res
    }

    pub fn build(&mut self) -> Translator {
        Translator {
            is_empty: self.table.len() == 0,
            is_raw: self.is_raw,
            ignore_unknown: self.ignore_unknown,
            multiple_val: self.check_multiple_val(),
            table: std::mem::take(&mut self.table)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALPHABET: &str =       "abcdefghijklmnopqrstuvwxyz";
    const ALPHABET_UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    const NUMS: &str =           "1234567890";
    const NUMS_UPPER: &str =     "!@#$%^&*()";
    const SYMBOLS: &str =        "`[]/=-\\',.;";
    const SYMBOLS_UPPER: &str =  "~{}?+_|\"<>:";
    
    #[test]
    fn test_translate_default() {
        let translator = Translator::default();

        assert_eq!(translator.translate(ALPHABET), translator.translate(ALPHABET_UPPER));
        assert_eq!(translator.translate(NUMS), "          ");
        assert_eq!(translator.translate(NUMS_UPPER), "          ");
        assert_eq!(translator.translate(SYMBOLS), translator.translate(SYMBOLS_UPPER));
        assert_eq!(translator.translate("žø"), "  ");
        assert_eq!(translator.translate("…"), "...");
        assert_eq!(translator.translate("«´»÷‘“”’–ʹ͵"), "'''/''''-''");
    }

    #[test]
    fn test_keep_all() {
        let translator = Translator::new()
            .keep_unknown()
            .build();
        
        assert_eq!(translator.translate("ŽAamong us"), "ŽAamong us");
        assert_eq!(translator.translate(NUMS), NUMS);
    }

    #[test]
    fn test_multiple() {
        let translator = Translator::new()
            .to_multiple(vec![('Ž', "*z")])
            .letters_to_lowercase("aď")
            .build();
        
        assert_eq!(translator.translate("ŽAaØ ď"), "*zaa  ď");
    }
}