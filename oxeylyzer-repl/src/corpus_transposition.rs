use glob::glob;
use oxeylyzer_core::translation::*;
use serde::Deserialize;

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

#[derive(Deserialize, Default)]
struct Multiple {
    #[serde(default)]
    uppercase_versions: bool,
    list: Vec<[String; 2]>,
}

#[derive(Deserialize, Default)]
struct OneToOne {
    pub from: String,
    to: String,
}

impl std::ops::Add for OneToOne {
    type Output = OneToOne;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.from.push_str(&rhs.from);
        self.to.push_str(&rhs.to);

        self
    }
}

#[derive(Deserialize)]
struct CorpusConfigLoad {
    // source: Option<String>,
    // #[serde(from = "OneOrMany<_>")]
    #[serde(default)]
    inherits: Vec<String>,
    #[serde(default)]
    letters_to_lowercase: String,
    #[serde(default)]
    keep: String,
    #[serde(default)]
    multiple: Multiple,
    #[serde(default)]
    one_to_one: OneToOne,
    #[serde(default)]
    punct_unshifted: OneToOne,
}

impl CorpusConfigLoad {
    fn check_for_language(language: &str) -> Result<PathBuf, String> {
        let try_find_path = glob("static/corpus_configs/*/*.toml")
            .unwrap()
            .flatten()
            .find(|stem| stem.file_stem().unwrap_or_else(|| std::ffi::OsStr::new("")) == language);
        if let Some(path) = try_find_path {
            let res = path
                .as_path()
                .parent()
                .unwrap()
                .components()
                .next_back()
                .unwrap()
                .as_os_str();
            Ok(PathBuf::from(res))
        } else {
            Err("Could not find a fitting config".to_string())
        }
    }

    pub fn new(language: &str, preferred_folder: Option<&str>) -> Result<Self, String> {
        let preferred_folder = if let Some(folder) = preferred_folder {
            Ok(PathBuf::from(folder))
        } else {
            Self::check_for_language(language)
        };

        if let Ok(preferred_folder) = preferred_folder {
            let file_name = format!("{language}.toml");
            let path = PathBuf::from("static")
                .join("corpus_configs")
                .join(preferred_folder)
                .join(file_name);

            let mut f = File::open(path)
                .map_err(|_| "Couldn't open file because it does not exist.".to_string())?;

            let mut buf = String::new();
            f.read_to_string(&mut buf)
                .map_err(|_| "Toml contains non-utf8 characters, aborting...".to_string())?;

            toml::from_str(buf.as_str()).map_err(|_| {
                "Toml contains invalid elements. Check the readme for what is allowed.".to_string()
            })
        } else {
            Err("No config file found!".to_string())
        }
    }
}

pub struct CorpusConfig {
    // source_language: String,
    inherits: Vec<String>,
    letters_to_lowercase: String,
    punct_unshifted: OneToOne,
    keep: String,
    to_multiple: Vec<(char, String)>,
    one_to_one: OneToOne,
}

impl CorpusConfig {
    pub fn new(language: &str, preferred_folder: Option<&str>) -> Result<Self, String> {
        let loaded = CorpusConfigLoad::new(language, preferred_folder)?;
        // let inherits = match loaded.inherits {
        //     Some(Single(v)) => vec![v],
        //     Some(Multiple(l)) => l,
        //     None => Vec::new()
        // };
        Ok(Self {
            // source_language: loaded.source.unwrap_or_else(|| language.to_string()),
            inherits: loaded.inherits,
            letters_to_lowercase: loaded.letters_to_lowercase,
            punct_unshifted: loaded.punct_unshifted,
            keep: loaded.keep,
            to_multiple: Self::get_to_multiple(loaded.multiple),
            one_to_one: loaded.one_to_one,
        })
    }

    fn get_to_multiple(multiple: Multiple) -> Vec<(char, String)> {
        let mut res = Vec::new();
        if multiple.uppercase_versions {
            for [from, to] in multiple.list {
                if from.chars().count() == 1 {
                    let c = from.chars().next().unwrap();
                    res.push((c, to.clone()));

                    let mut upper = c.to_uppercase();
                    if upper.clone().count() == 1 {
                        let upper_c = upper.next().unwrap();
                        res.push((upper_c, to))
                    }
                }
            }
        }
        res
    }

    pub fn all() -> Vec<(String, Self)> {
        glob("static/text/*")
            .unwrap()
            .flatten()
            .filter(|pb| pb.is_dir())
            .flat_map(|pb| pb.file_name().unwrap().to_os_string().into_string())
            .map(|l| (l.clone(), Self::new(&l, None)))
            .flat_map(|(l, c)| c.ok().map(|cc| (l, cc)))
            .collect::<Vec<_>>()
    }

    pub fn new_translator(language: &str, preferred_folder: Option<&str>) -> Translator {
        match Self::new(language, preferred_folder) {
            Ok(config) => config.translator(),
            Err(error) => {
                println!("{error}\nUsing a raw translator instead.");
                Self::raw_translator()
            }
        }
    }

    pub fn translator(self) -> Translator {
        let mut res = Translator::new()
            .letters_to_lowercase(&self.letters_to_lowercase)
            .keep(&self.keep)
            .one_to_one(&self.one_to_one.from, &self.one_to_one.to)
            .custom_unshift(&self.punct_unshifted.from, &self.punct_unshifted.to)
            .to_multiple_string(&self.to_multiple)
            .build();

        for inherits in self.inherits {
            if let Ok(new) = Self::new(&inherits, None) {
                res = res + new.translator();
            }
        }
        res
    }

    pub fn raw_translator() -> Translator {
        Translator::raw(true)
    }
}
