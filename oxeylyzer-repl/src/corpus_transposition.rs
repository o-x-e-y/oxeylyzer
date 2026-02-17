use std::{convert::Infallible, fs::File, io::Read, path::PathBuf};

use oxeylyzer_core::corpus_cleaner::CorpusCleaner;
use serde::Deserialize;
use serde_with::{serde_as, serde_conv};

serde_conv!(
    StringAsCharArray,
    Vec<char>,
    |trigram: &[char]| String::from_iter(trigram),
    |value: String| -> Result<_, Infallible> { Ok(value.chars().collect::<Vec<_>>()) }
);

#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
struct Multiple {
    #[serde(default)]
    uppercase_versions: bool,
    list: Vec<(char, String)>,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
struct OneToOne {
    #[serde_as(as = "StringAsCharArray")]
    pub from: Vec<char>,
    #[serde_as(as = "StringAsCharArray")]
    to: Vec<char>,
}

impl std::ops::Add for OneToOne {
    type Output = OneToOne;

    fn add(self, rhs: Self) -> Self::Output {
        let from = self.from.into_iter().chain(rhs.from).collect();
        let to = self.to.into_iter().chain(rhs.to).collect();

        Self { from, to }
    }
}

#[serde_as]
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct CorpusConfigLoad {
    inherits: Vec<String>,
    #[serde_as(as = "StringAsCharArray")]
    letters_to_lowercase: Vec<char>,
    #[serde_as(as = "StringAsCharArray")]
    keep: Vec<char>,
    multiple: Multiple,
    one_to_one: OneToOne,
    punct_unshifted: OneToOne,
}

impl CorpusConfigLoad {
    fn check_for_language(language: &str) -> Result<PathBuf, String> {
        let try_find_path = glob::glob("static/corpus_configs/*/*.toml")
            .unwrap()
            .flatten()
            .find(|stem| stem.file_stem().unwrap_or_default() == language);

        if let Some(path) = try_find_path {
            let res = path
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

    fn load(language: &str, preferred_folder: Option<&str>) -> Result<Self, String> {
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
                .map_err(|e| format!("Couldn't open file because it does not exist: {e}"))?;

            let mut buf = String::new();
            f.read_to_string(&mut buf)
                .map_err(|e| format!("Toml contains non-utf8 characters, aborting... {e}"))?;

            toml::from_str(buf.as_str()).map_err(|e| {
                format!("Toml contains invalid elements. Check the readme for what is allowed: {e}")
            })
        } else {
            Err("No config file found!".to_string())
        }
    }
}

impl From<CorpusConfigLoad> for CorpusConfig {
    fn from(loaded: CorpusConfigLoad) -> Self {
        Self {
            // source_language: loaded.source.unwrap_or_else(|| language.to_string()),
            inherits: loaded.inherits,
            letters_to_lowercase: loaded.letters_to_lowercase,
            punct_unshifted: loaded.punct_unshifted,
            keep: loaded.keep,
            to_multiple: Self::get_to_multiple(loaded.multiple),
            one_to_one: loaded.one_to_one,
        }
    }
}

// TODO: adapt for cleaner
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(from = "CorpusConfigLoad")]
pub struct CorpusConfig {
    // TODO: add cycle detection
    inherits: Vec<String>,
    letters_to_lowercase: Vec<char>,
    punct_unshifted: OneToOne,
    keep: Vec<char>,
    to_multiple: Vec<(char, String)>,
    one_to_one: OneToOne,
}

impl CorpusConfig {
    fn new(loaded: CorpusConfigLoad) -> Self {
        loaded.into()
    }

    pub fn load(language: &str, preferred_folder: Option<&str>) -> Result<Self, String> {
        let loaded = CorpusConfigLoad::load(language, preferred_folder)?;

        Ok(Self::new(loaded))
    }

    fn get_to_multiple(multiple: Multiple) -> Vec<(char, String)> {
        let mut res = Vec::new();

        for (from, to) in multiple.list {
            res.push((from, to.clone()));

            if multiple.uppercase_versions {
                let mut upper = from.to_uppercase();
                if upper.clone().count() == 1 {
                    let upper_c = upper.next().unwrap();
                    res.push((upper_c, to))
                }
            }
        }

        res
    }

    pub fn all() -> Vec<(String, Self)> {
        glob::glob("static/text/*")
            .unwrap()
            .flatten()
            .filter(|pb| pb.is_dir())
            .flat_map(|pb| pb.file_name().unwrap().to_os_string().into_string())
            .map(|l| (l.clone(), Self::load(&l, None)))
            .flat_map(|(l, c)| c.ok().map(|cc| (l, cc)))
            .collect::<Vec<_>>()
    }

    pub fn new_translator(language: &str, preferred_folder: Option<&str>) -> CorpusCleaner {
        match Self::load(language, preferred_folder) {
            Ok(config) => config.into(),
            Err(error) => {
                println!("{error}\nUsing a raw translator instead.");
                CorpusCleaner::raw()
            }
        }
    }
}

impl std::ops::Add<CorpusConfig> for CorpusConfig {
    type Output = CorpusConfig;

    fn add(self, rhs: CorpusConfig) -> Self::Output {
        let inherits = self.inherits.into_iter().chain(rhs.inherits).collect();
        let to_multiple = self
            .to_multiple
            .into_iter()
            .chain(rhs.to_multiple)
            .collect();
        let letters_to_lowercase = self.letters_to_lowercase.into_iter().chain(rhs.letters_to_lowercase).collect();
        let keep = self.keep.into_iter().chain(rhs.keep).collect();
        let punct_unshifted = self.punct_unshifted + rhs.punct_unshifted;
        let one_to_one = self.one_to_one + rhs.one_to_one;

        CorpusConfig {
            inherits,
            letters_to_lowercase,
            punct_unshifted,
            keep,
            to_multiple,
            one_to_one,
        }
    }
}

impl From<CorpusConfig> for CorpusCleaner {
    fn from(mut config: CorpusConfig) -> Self {
        for inherits in config.inherits.clone() {
            if let Ok(new) = CorpusConfig::load(&inherits, None) {
                config = config + new;
            }
        }

        // TODO: add proper dead key mapping, custom shift and repeat char
        CorpusCleaner::builder()
            .with_chars(config.letters_to_lowercase)
            .with_exact_mappings(config.keep)
            .with_char_mappings(
                config
                    .one_to_one
                    .from
                    .into_iter()
                    .zip(config.one_to_one.to),
            )
            .with_uppercase_mappings(
                config
                    .punct_unshifted
                    .to
                    .into_iter()
                    .zip(config.punct_unshifted.from),
            )
            .with_mappings(
                config
                    .to_multiple
                    .into_iter()
                    .map(|(c, s)| (c, s.chars().collect())),
            )
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inherits() {
        let config1 = r#"inherits = ["dofsmie"]"#;
        let config2 = r#"inherits = ["yeah"]"#;

        let loaded1 = toml::from_str::<CorpusConfigLoad>(config1).unwrap();
        let loaded2 = toml::from_str::<CorpusConfigLoad>(config2).unwrap();
        assert_eq!(loaded1.inherits, vec!["dofsmie"]);
        assert_eq!(loaded2.inherits, vec!["yeah"]);

        let config1 = CorpusConfig::new(loaded1);
        let config2 = CorpusConfig::new(loaded2);

        let config = config1 + config2;
        assert_eq!(config.inherits, vec!["dofsmie", "yeah"]);
    }

    #[test]
    fn letters_to_lowercase() {
        let config1 = r#"letters_to_lowercase = "dofsmie""#;
        let config2 = r#"letters_to_lowercase = "yeah""#;

        let loaded1 = toml::from_str::<CorpusConfigLoad>(config1).unwrap();
        let loaded2 = toml::from_str::<CorpusConfigLoad>(config2).unwrap();
        assert_eq!(loaded1.letters_to_lowercase, "dofsmie".chars().collect::<Vec<_>>());
        assert_eq!(loaded2.letters_to_lowercase, "yeah".chars().collect::<Vec<_>>());

        let config1 = CorpusConfig::new(loaded1);
        let config2 = CorpusConfig::new(loaded2);

        let config = config1 + config2;
        assert_eq!(config.letters_to_lowercase, concat!("dofsmie", "yeah").chars().collect::<Vec<_>>());
    }

    #[test]
    fn keep() {
        let config1 = r#"keep = "dofsmie""#;
        let config2 = r#"keep = "yeah""#;

        let loaded1 = toml::from_str::<CorpusConfigLoad>(config1).unwrap();
        let loaded2 = toml::from_str::<CorpusConfigLoad>(config2).unwrap();
        assert_eq!(loaded1.keep, "dofsmie".chars().collect::<Vec<_>>());
        assert_eq!(loaded2.keep, "yeah".chars().collect::<Vec<_>>());

        let config1 = CorpusConfig::new(loaded1);
        let config2 = CorpusConfig::new(loaded2);

        let config = config1 + config2;
        assert_eq!(config.keep, concat!("dofsmie", "yeah").chars().collect::<Vec<_>>());
    }

    #[test]
    fn multiple() {
        let config1 = r#"
            [multiple]
            list = [
              ["a", "dofsmie"],
            ]
        "#;
        let config2 = r#"
            [multiple]
            uppercase_versions = true
            list = [
              ["b", "yeah"],
            ]
        "#;

        let loaded1 = toml::from_str::<CorpusConfigLoad>(config1).unwrap();
        let loaded2 = toml::from_str::<CorpusConfigLoad>(config2).unwrap();
        assert_eq!(
            loaded1.multiple,
            Multiple {
                uppercase_versions: false,
                list: vec![('a', "dofsmie".to_string())]
            }
        );
        assert_eq!(
            loaded2.multiple,
            Multiple {
                uppercase_versions: true,
                list: vec![('b', "yeah".to_string())]
            }
        );

        let config1 = CorpusConfig::new(loaded1);
        let config2 = CorpusConfig::new(loaded2);

        let config = config1 + config2;
        assert_eq!(
            config.to_multiple,
            vec![
                ('a', "dofsmie".to_string()),
                ('b', "yeah".to_string()),
                ('B', "yeah".to_string()),
            ]
        );
    }

    #[test]
    fn one_to_one() {
        let config1 = r#"
            [one_to_one]
            from = "yeah"
            to =   "dofs"
        "#;
        let config2 = r#"
            [one_to_one]
            from = "nah"
            to =   "lol"
        "#;

        let loaded1 = toml::from_str::<CorpusConfigLoad>(config1).unwrap();
        let loaded2 = toml::from_str::<CorpusConfigLoad>(config2).unwrap();
        assert_eq!(
            loaded1.one_to_one,
            OneToOne {
                from: "yeah".chars().collect::<Vec<_>>(),
                to: "dofs".chars().collect::<Vec<_>>()
            }
        );
        assert_eq!(
            loaded2.one_to_one,
            OneToOne {
                from: "nah".chars().collect::<Vec<_>>(),
                to: "lol".chars().collect::<Vec<_>>()
            }
        );

        let config1 = CorpusConfig::new(loaded1);
        let config2 = CorpusConfig::new(loaded2);

        let config = config1 + config2;
        assert_eq!(
            config.one_to_one,
            OneToOne {
                from: concat!("yeah", "nah").chars().collect::<Vec<_>>(),
                to: concat!("dofs", "lol").chars().collect::<Vec<_>>()
            }
        );
    }
}
