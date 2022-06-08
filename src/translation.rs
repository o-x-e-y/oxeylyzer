use std::collections::HashMap;
use std::borrow::Cow;
use anyhow::Result;

pub struct Translator {
    pub table: HashMap<char, Cow<'static, str>>,
    pub(crate) ignore_unknown: bool,
    pub(crate) is_empty: bool,
    pub(crate) multiple_val: f64
}

impl Default for Translator {
    fn default() -> Self {
        Translator::new()
		    .default_formatting()
		    .build()
    }
}

impl Translator {
    pub fn new() -> TranslatorBuilder {
        TranslatorBuilder {
            table: HashMap::new(),
            ignore_unknown: false
        }
    }

    pub fn language(language: &str) -> Result<Self> {
        Ok(Self::new()
            .language(language)?
            .build())
    }

    pub fn language_or_default(language: &str) -> Self {
        if let Ok(t) = Self::language(language) {
            t
        } else {
            Self::default()
        }
    }

    pub fn language_or_passthrough(language: &str) -> Self {
        if let Ok(t) = Self::language(language) {
            t
        } else {
            Self::passthrough()
        }
    }

    pub fn passthrough() -> Self {
        Translator::new()
            .passthrough()
            .ascii_lower()
            .normalize_punct()
            .keep_unknown()
            .build()
    }

    pub fn translate<'a>(&self, s: &'a str) -> Cow<'a, str> {
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

pub struct TranslatorBuilder {
    table: HashMap<char, Cow<'static, str>>,
    ignore_unknown: bool
}

impl TranslatorBuilder {
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

    pub fn to_one(&mut self, from: &str, to: char) -> &mut Self {
        for c in from.chars() {
            self.table.insert(c, Cow::from(to.to_string()));
        }
        self
    }

    pub fn keep_same(&mut self, keep: &str) -> &mut Self {
        for c in keep.chars() {
            self.table.insert(c, Cow::from(c.to_string()));
        }
        self
    }

    pub fn to_another(&mut self, from: &str, to: &str) -> &mut Self {
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

    pub fn letters(&mut self, letters: &str) -> &mut Self {
        for letter in letters.chars() {
            self.table.insert(letter, Cow::from(letter.to_string()));

            let upper_string = String::from_iter(letter.to_uppercase());

            let new_upper = upper_string.chars().next().unwrap();
            self.table.insert(new_upper, Cow::from(letter.to_string()));
        }
        self
    }

    pub fn passthrough(&mut self) -> &mut Self {
        let mut letters = String::new();
        for i in 128u32..1250 {
            if let Some(c) = char::from_u32(i)
            && c.is_alphabetic() {
                letters.push(c);
            }
        }

        self
            .letters(letters.as_str())
            .alphabet_lower()
            .punct_lower()
            .normalize_punct()
    }

    pub fn punct_lower(&mut self) -> &mut Self {
        self
            .keep_same("[]/=-\\',.;`")
            .to_another("{}?+_|\"<>:~", "[]/=-\\',.;`")
    }

    pub fn alphabet_lower(&mut self) -> &mut Self {
        self.letters("abcdefghijklmnopqrstuvwxyz")
    }

    pub fn number_symbols_lower(&mut self) -> &mut Self {
        self.to_another("!@#$%^&*()", "1234567890")
    }

    pub fn ascii_lower(&mut self) -> &mut Self {
        self
            .punct_lower()
            .alphabet_lower()
    }

    pub fn normalize_punct(&mut self) -> &mut Self {
        self
            .to_another("«´»÷‘“”’–ʹ͵","'''/''''-''")
            .one_multiple('…', "...")
    }

    pub fn default_formatting(&mut self) -> &mut Self {
        self
            .ascii_lower()
            .normalize_punct()
    }

    pub fn language(&mut self, language: &str) -> Result<&mut Self> {
        self.default_formatting();
        let language = language.to_lowercase();
        match language.as_str() {
            "akl" | "english" | "english2" | "toki_pona" | "indonesian"=> Ok(self),
            "albanian" => Ok(self.letters("çë")),
            "bokmal" | "nynorsk" => Ok(self.letters("åøæ")),
            "czech" => {
            Ok(self
                .to_multiple(vec![
                    ('á', "*a"), ('č', "*c"), ('ď', "*d"), ('ě', "*e"), ('é', "*x"), ('í', "*i"),
                    ('ň', "*n"), ('ó', "*o"), ('ř', "*r"), ('š', "*s"), ('ť', "*t"), ('ů', "*u"),
                    ('ú', "*b"), ('ý', "*y"), ('ž', "*z"), ('Á', "*a"), ('Č', "*c"), ('Ď', "*d"),
                    ('Ě', "*e"), ('É', "*x"), ('Í', "*i"), ('Ň', "*n"), ('Ó', "*o"), ('Ř', "*r"),
                    ('Š', "*s"), ('Ť', "*t"), ('Ů', "*u"), ('Ú', "*b"), ('Ý', "*y"), ('Ž', "*z")
                ])
                .letters("áíě"))
            },
            "dutch" => Ok(self.letters("áèéçëíîó")),
            "dutch_repeat" => Ok(self.letters("áèéçëíîó@")),
            "english_repeat" => Ok(self.keep_same("@")),
            "english_th" => Ok(self.letters("þ")),
            "finnish" => Ok(self
                .letters("åäö")
            ),
            "finnish_repeat" => Ok(self
                .letters("åäö@")
            ),
            "french" | "french_qu" => {
            Ok(self
                .to_multiple(vec![
                    ('ç', "*c"), ('Ç', "*c"), ('œ', "oe"), ('á', "*'a"), ('â', "*.a"), ('è', "*,e"),
                    ('ê', "*.e"), ('ì', "*.i"), ('í', "*'i"), ('î', "*.i"), ('ò', "*,o"), ('ó', "*'o"),
                    ('ô', "*.o"), ('ù', "*,u"), ('ú', "*'u"), ('û', "*.u"), ('Á', "*'a"), ('Â', "*.a"),
                    ('È', "*,e"), ('Ê', "*.e"), ('Ì', "*,i"), ('Í', "*'i"), ('Î', "*.i"), ('Ò', "*,o"),
                    ('Ó', "*'o"), ('Ô', "*.o"), ('Ù', "*,u"), ('Ú', "*'u"), ('Û', "*.u"), ('ä', "*'a"),
                    ('ë', "*'e"), ('ï', "*'i"), ('ö', "*'o"), ('ü', "*'u"), ('Ä', "*'a"), ('Ë', "*'e"),
                    ('Ï', "*'i"), ('Ö', "*'o"), ('Ü', "*'u")
                ])
                .letters("éà"))
            },
            "german" => Ok(self.letters("äöüß")),
            "spanish" => {
            Ok(self
                .to_multiple(vec![
                    ('á', "*a"), ('é', "*e"), ('í', "*i"), ('ó', "*o"), ('ú', "*u"), ('ü', "*y"),
                    ('Á', "*a"), ('É', "*e"), ('Í', "*i"), ('Ó', "*o"), ('Ú', "*u"), ('Ü', "*y"),
                    ('ñ', "*n"), ('Ñ', "*n")    
                ]))
            },
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
            .letters("aď")
            .build();
        
        assert_eq!(translator.translate("ŽAaØ ď"), "*zaa  ď");
    }
}