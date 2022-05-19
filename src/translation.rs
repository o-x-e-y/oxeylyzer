use std::borrow::Cow;
use anyhow::Result;

const USIZE_BOUND: usize = 0x110000;
const SUPPORTED_CHARS: usize = 8500;

pub struct Translator {
    pub table: Vec<Cow<'static, str>>,
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
        let mut table: Vec<Cow<'static, str>> = Vec::with_capacity(SUPPORTED_CHARS);
        for _ in 0..SUPPORTED_CHARS {
            table.push(Cow::from(" "));
        }
        TranslatorBuilder {
            table: table.try_into().unwrap()
        }
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

        for c in s.chars() {
            let n = c as usize;
            if n < SUPPORTED_CHARS {
                res.push_str(self.table[c as usize].as_ref());
            } else {
                res.push(' ');
            }
        }

        res.shrink_to_fit();
        Cow::from(res)
	}
}

pub struct TranslatorBuilder {
    table: Vec<Cow<'static, str>>
}

impl TranslatorBuilder {
    pub fn to_nothing(&mut self, to_nothing: &str) -> &mut Self {
        for c in to_nothing.chars() {
            self.table[c as usize] = Cow::from("");
        }
        self
    }

    pub fn to_space(&mut self, to_string: &str) -> &mut Self {
        for c in to_string.chars() {
            self.table[c as usize] = Cow::from(" ");
        }
        self
    }

    pub fn to_one(&mut self, from: &str, to: char) -> &mut Self {
        for c in from.chars() {
            self.table[c as usize] = Cow::from(to.to_string());
        }
        self
    }

    pub fn keep_same(&mut self, keep: &str) -> &mut Self {
        for c in keep.chars() {
            self.table[c as usize] = Cow::from(String::from(c));
        }
        self
    }

    pub fn to_another(&mut self, from: &str, to: &str) -> &mut Self {
        assert_eq!(from.chars().count(), to.chars().count());

        for (s, d) in from.chars().zip(to.chars()) {
            self.table[s as usize] = Cow::from(d.to_string());
        }
        self
    }

    pub fn one_multiple(&mut self, from: char, to: &'static str) -> &mut Self {
        self.table[from as usize] = Cow::from(to);
        self
    }

    pub fn to_multiple(&mut self, trans: Vec<(char, &'static str)>) -> &mut Self {
        for (s, d) in trans {
            self.table[s as usize] = Cow::from(d);
        }
        self
    }

    pub fn letters(&mut self, letters: &str) -> &mut Self {
        for letter in letters.chars() {
            self.table[letter as usize] = Cow::from(letter.to_string());

            let upper_string = String::from_iter(letter.to_uppercase());

            let new_upper = upper_string.chars().next().unwrap();
            self.table[new_upper as usize] = Cow::from(letter.to_string());
        }
        self
    }

    pub fn punct_lower(&mut self) -> &mut Self {
        self.to_another("{}?+_|\"<>:~", "[]/=-\\',.;`")
    }

    pub fn letters_lower(&mut self) -> &mut Self {
        self.to_another("ABCDEFGHIJKLMNOPQRSTUVWXYZ",
                          "abcdefghijklmnopqrstuvwxyz")
    }

    pub fn number_symbols_lower(&mut self) -> &mut Self {
        self.to_another("!@#$%^&*()", "1234567890")
    }

    pub fn ascii_lower(&mut self) -> &mut Self {
        self
            .punct_lower()
            .letters_lower()
    }

    pub fn keep_numbers(&mut self) -> &mut Self {
        self.keep_same("1234567890")
    }

    pub fn keep_default(&mut self) -> &mut Self {
        self.keep_same("abcdefghijklmnopqrstuvwxyz.,';[]/=-\\`")
    }

    pub fn default_formatting(&mut self) -> &mut Self {
        self
            .keep_default()
            .ascii_lower()
            .one_multiple('…', "...")
            .to_another("«´»÷‘“”’–ʹ͵","'''/''''-''")
    }

    pub fn language(&mut self, language: &str) -> Result<&mut Self> {
        self.default_formatting();
        let language = language.to_lowercase();
        if language == "english" || language == "toki_pona" {
            Ok(self)
        } else if language == "albanian" {
            Ok(self
                .letters("çë"))
        } else if language == "bokmal" || language == "nynorsk" {
            Ok(self
                .letters("åøæ"))
        } else if language == "czech" {
            Ok(self
                .to_multiple(vec![
                    ('á', "*a"), ('č', "*c"), ('ď', "*d"), ('ě', "*e"), ('é', "*x"), ('í', "*i"),
                    ('ň', "*n"), ('ó', "*o"), ('ř', "*r"), ('š', "*s"), ('ť', "*t"), ('ů', "*u"),
                    ('ú', "*b"), ('ý', "*y"), ('ž', "*z"), ('Á', "*a"), ('Č', "*c"), ('Ď', "*d"),
                    ('Ě', "*e"), ('É', "*x"), ('Í', "*i"), ('Ň', "*n"), ('Ó', "*o"), ('Ř', "*r"),
                    ('Š', "*s"), ('Ť', "*t"), ('Ů', "*u"), ('Ú', "*b"), ('Ý', "*y"), ('Ž', "*z")
                ])
                .letters("øáíě"))
        } else if language == "dutch" {
            Ok(self
                .letters("áèéçëíîó"))
        } else if language == "german" {
            Ok(self
                .letters("äöüß"))
        } else if language == "spanish" {
            Ok(self
                .letters("ñ")
                .to_multiple(vec![
                    ('á', "*a"), ('é', "*e"), ('í', "*i"), ('ó', "*o"), ('ú', "*u"), ('ü', "*y"),
                    ('Á', "*a"), ('É', "*e"), ('Í', "*i"), ('Ó', "*o"), ('Ú', "*u"), ('Ü', "*y")
                ]))
        } else {
            Err(anyhow::format_err!("This language is not available. You'll have to make your own formatter, sorry!"))
        }
    }

    pub fn language_or_default(&mut self, language: &str) -> &mut Self {
        if self.language(language).is_ok() {
            self
        } else {
            self.default_formatting()
        }
    }

    fn check_multiple_val(&self) -> f64 {
        // assume a 2.5% occurence of every 1 -> n translation to be safe
        // subtract from total length with a factor of 0.1 in case of a 1 -> 0 translation

        let mut res = 0.0;
        for trans in self.table.iter() {
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
    fn test_multiple() {
        let translator = Translator::new()
            .to_multiple(vec![('Ž', "*z")])
            .letters("aď")
            .build();
        
        assert_eq!(translator.translate("ŽAaØ ď"), "*zaa  ď");
    }
}