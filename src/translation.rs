use std::collections::HashMap;

pub struct Translator {
    table: HashMap<char, String>,
    is_empty: bool,
    multiple_val: f64
}

impl Translator {
    pub fn new() -> TranslatorBuilder {
        TranslatorBuilder {
            table: HashMap::new()
        }
    }

    pub fn translate(&self, s: impl ToString) -> String {
        let mut res: String;
        let s = s.to_string();

        if self.is_empty {
            return s.to_string();
        } else if self.multiple_val == 0.0 {
            res = String::with_capacity(s.len()); 
        } else {
            let f64_len = s.len() as f64;
            let length = f64_len + f64_len / (0.025 * self.multiple_val);
            res = String::with_capacity(length as usize);
        }

        for c in s.chars() {
            res.push_str(self.table.get(&c).unwrap_or(&String::from(c)));
        }

        res.shrink_to_fit();
        res
	}
}

pub struct TranslatorBuilder {
    table: HashMap<char, String>
}

impl TranslatorBuilder {
    pub fn to_nothing(&mut self, to_nothing: &str) -> &mut Self {
        for c in to_nothing.chars() {
            self.table.insert(c, "".to_string());
        }
        self
    }

    pub fn to_space(&mut self, to_string: &str) -> &mut Self {
        for c in to_string.chars() {
            self.table.insert(c, " ".to_string());
        }
        self
    }

    pub fn to_same(&mut self, from: &str, to: char) -> &mut Self {
        let replace = String::from(to);
        for c in from.chars() {
            self.table.insert(c, replace.clone());
        }
        self
    }

    pub fn to_another(&mut self, from: &str, to: &str) -> &mut Self {
        for (s, d) in from.chars().zip(to.chars()) {
            self.table.insert(s, String::from(d));
        }
        self
    }

    pub fn to_multiple(&mut self, trans: Vec<(char, &str)>) -> &mut Self {
        for (s, d) in trans {
            self.table.insert(s, d.to_string());
        }
        self
    }

    pub fn one_multiple(&mut self, from: char, to: &str) -> &mut Self {
        self.table.insert(from, to.to_string());
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
        self.to_another("!@#$%^&*()", "123456789")
    }

    pub fn ascii_lower(&mut self) -> &mut Self {
        self
            .punct_lower()
            .letters_lower()
            .number_symbols_lower()
    }

    pub fn hide_numbers(&mut self) -> &mut Self {
        self.to_space("1234567890")
    }

    pub fn default_formatting(&mut self) -> &mut Self {
        self
            .ascii_lower()
            .hide_numbers()
            .to_space("!@#$%^&*()")
            .to_another(
                "\r\t\n«´»ÀÁÂÄÇÈÉÊËÌÍÎÏÑÒÓÔÖÙÚÛÜàáâäçèéêëìíîïñòóôöùúûü÷‘“”’–ʹ͵",
                     "   '''                                            /''''-''"
            )
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
            multiple_val: self.check_multiple_val(),
            table: std::mem::take(&mut self.table)
        }
    }
}