use std::{
    collections::HashSet,
    convert::Infallible,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use oxeylyzer_core::corpus_cleaner::CorpusCleaner;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, serde_conv};

serde_conv!(
    StringAsCharArray,
    Vec<char>,
    |chars: &[char]| String::from_iter(chars),
    |string: String| -> Result<_, Infallible> { Ok(string.chars().collect::<Vec<_>>()) }
);

serde_conv!(
    MultipleAsVec,
    Vec<(char, String)>,
    |_: &[(char, String)]| {
        unimplemented!("Serializing a Multiple struct is currently unsupported");
        #[allow(unused)]
        Multiple::default()
    },
    |multiple: Multiple| -> Result<_, Infallible> {
        let vec = multiple
            .list
            .into_iter()
            .map(|(from, to)| {
                if multiple.uppercase_versions && from.to_uppercase().count() == 1 {
                    let upper = from.to_uppercase().next().unwrap();
                    vec![(from, to.clone()), (upper, to)]
                } else {
                    vec![(from, to)]
                }
            })
            .flatten()
            .collect::<Vec<_>>();

        Ok(vec)
    }
);

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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

// TODO: adapt for cleaner
#[serde_as]
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CorpusConfig {
    inherits: Vec<String>,
    #[serde_as(as = "StringAsCharArray")]
    letters_to_lowercase: Vec<char>,
    #[serde_as(as = "StringAsCharArray")]
    keep: Vec<char>,
    #[serde_as(as = "MultipleAsVec")]
    multiple: Vec<(char, String)>,
    one_to_one: OneToOne,
    punct_unshifted: OneToOne,
    repeat_key: bool,
    #[serde(skip)]
    inherits_visited: HashSet<String>,
}

impl CorpusConfig {
    pub fn load(language: &str, preferred_folder: Option<&str>) -> Result<Self, String> {
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

    pub fn all<P: AsRef<Path>>(base_path: P) -> Vec<(String, Self)> {
        let path = base_path.as_ref().join("static/corpus_configs");
        let pattern = format!("{}/*/*.toml", path.display());

        glob::glob(&pattern)
            .unwrap()
            .flatten()
            .filter(|pb| pb.is_file())
            .flat_map(|pb| {
                File::open(&pb).map(|file| {
                    let lang = pb
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .trim_end_matches(".toml")
                        .to_string();

                    (lang, file)
                })
            })
            .flat_map(|(lang, mut f)| {
                let mut buf = String::new();
                f.read_to_string(&mut buf).map(|_| (lang, buf))
            })
            .flat_map(|(lang, contents)| toml::from_str(&contents).map(|config| (lang, config)))
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
}

impl std::ops::Add<CorpusConfig> for CorpusConfig {
    type Output = CorpusConfig;

    fn add(self, rhs: CorpusConfig) -> Self::Output {
        let inherits = self.inherits.into_iter().chain(rhs.inherits).collect();
        let multiple = self.multiple.into_iter().chain(rhs.multiple).collect();
        let letters_to_lowercase = self
            .letters_to_lowercase
            .into_iter()
            .chain(rhs.letters_to_lowercase)
            .collect();
        let keep = self.keep.into_iter().chain(rhs.keep).collect();
        let punct_unshifted = self.punct_unshifted + rhs.punct_unshifted;
        let one_to_one = self.one_to_one + rhs.one_to_one;
        let repeat_key = self.repeat_key || rhs.repeat_key;
        let inherits_visited = self
            .inherits_visited
            .into_iter()
            .chain(rhs.inherits_visited)
            .collect();

        CorpusConfig {
            inherits,
            letters_to_lowercase,
            punct_unshifted,
            keep,
            multiple,
            one_to_one,
            repeat_key,
            inherits_visited,
        }
    }
}

impl From<CorpusConfig> for CorpusCleaner {
    fn from(mut config: CorpusConfig) -> Self {
        for inherits in config.inherits.clone() {
            if !config.inherits_visited.contains(&inherits)
                && let Ok(new) = CorpusConfig::load(&inherits, None)
            {
                config.inherits_visited.insert(inherits);
                config = config + new;
            }
        }

        // TODO: add proper dead key mapping, custom shift and repeat char
        CorpusCleaner::builder()
            .with_chars(config.letters_to_lowercase)
            .with_exact_mappings(config.keep)
            .with_char_mappings(config.one_to_one.from.into_iter().zip(config.one_to_one.to))
            .with_uppercase_mappings(
                config
                    .punct_unshifted
                    .to
                    .into_iter()
                    .zip(config.punct_unshifted.from),
            )
            .with_mappings(
                config
                    .multiple
                    .into_iter()
                    .map(|(c, s)| (c, s.chars().collect())),
            )
            .repeat_key(config.repeat_key)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn inherits() {
        let config1 = r#"inherits = ["dofsmie"]"#;
        let config2 = r#"inherits = ["yeah"]"#;

        let config1 = toml::from_str::<CorpusConfig>(config1).unwrap();
        let config2 = toml::from_str::<CorpusConfig>(config2).unwrap();
        assert_eq!(config1.inherits, vec!["dofsmie".to_string()]);
        assert_eq!(config2.inherits, vec!["yeah"]);

        let config = config1 + config2;
        assert_eq!(config.inherits, vec!["dofsmie", "yeah"]);
    }

    #[test]
    fn letters_to_lowercase() {
        let config1 = r#"letters_to_lowercase = "dofsmie""#;
        let config2 = r#"letters_to_lowercase = "yeah""#;

        let config1 = toml::from_str::<CorpusConfig>(config1).unwrap();
        let config2 = toml::from_str::<CorpusConfig>(config2).unwrap();
        assert_eq!(
            config1.letters_to_lowercase,
            "dofsmie".chars().collect::<Vec<_>>()
        );
        assert_eq!(
            config2.letters_to_lowercase,
            "yeah".chars().collect::<Vec<_>>()
        );

        let config = config1 + config2;
        assert_eq!(
            config.letters_to_lowercase,
            concat!("dofsmie", "yeah").chars().collect::<Vec<_>>()
        );
    }

    #[test]
    fn keep() {
        let config1 = r#"keep = "dofsmie""#;
        let config2 = r#"keep = "yeah""#;

        let config1 = toml::from_str::<CorpusConfig>(config1).unwrap();
        let config2 = toml::from_str::<CorpusConfig>(config2).unwrap();
        assert_eq!(config1.keep, "dofsmie".chars().collect::<Vec<_>>());
        assert_eq!(config2.keep, "yeah".chars().collect::<Vec<_>>());

        let config = config1 + config2;
        assert_eq!(
            config.keep,
            concat!("dofsmie", "yeah").chars().collect::<Vec<_>>()
        );
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

        let config1 = toml::from_str::<CorpusConfig>(config1).unwrap();
        let config2 = toml::from_str::<CorpusConfig>(config2).unwrap();
        assert_eq!(config1.multiple, vec![('a', "dofsmie".to_string()),]);
        assert_eq!(
            config2.multiple,
            vec![('b', "yeah".to_string()), ('B', "yeah".to_string()),]
        );

        let config = config1 + config2;
        assert_eq!(
            config.multiple,
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

        let config1 = toml::from_str::<CorpusConfig>(config1).unwrap();
        let config2 = toml::from_str::<CorpusConfig>(config2).unwrap();
        assert_eq!(
            config1.one_to_one,
            OneToOne {
                from: "yeah".chars().collect::<Vec<_>>(),
                to: "dofs".chars().collect::<Vec<_>>()
            }
        );
        assert_eq!(
            config2.one_to_one,
            OneToOne {
                from: "nah".chars().collect::<Vec<_>>(),
                to: "lol".chars().collect::<Vec<_>>()
            }
        );

        let config = config1 + config2;
        assert_eq!(
            config.one_to_one,
            OneToOne {
                from: concat!("yeah", "nah").chars().collect::<Vec<_>>(),
                to: concat!("dofs", "lol").chars().collect::<Vec<_>>()
            }
        );
    }

    #[test]
    fn existing_file_validity() {
        for (lang, config) in CorpusConfig::all(concat!(std::env!("CARGO_MANIFEST_DIR"), "/..")) {
            config.keep.into_iter().for_each(|c| {
                assert_eq!(
                    c.to_uppercase().to_string(),
                    c.to_lowercase().to_string(),
                    "Corpus config for lang {lang} has keep rule for {c} which has an uppercase"
                );
            });

            let multiple_map = config.multiple.into_iter().collect::<HashMap<_, _>>();

            multiple_map.iter().for_each(|(c, str)| {
                let lower = c.to_lowercase().collect::<Vec<char>>();
                let upper = c.to_uppercase().collect::<Vec<char>>();

                if lower != upper && lower.len() == 1 && upper.len() == 1 {
                    let (lower, upper) = (lower[0], upper[0]);

                    let lower_to = multiple_map.get(&lower);
                    let upper_to = multiple_map.get(&upper);

                    assert!(
                        lower_to.is_some(),
                        concat!(
                            "multiple mapping for language {} has character '{}' mapped to ",
                            "\"{}\", but no such mapping exists for the lowercase variant {}",
                        ),
                        lang,
                        upper,
                        str,
                        lower
                    );

                    assert!(
                        upper_to.is_some(),
                        concat!(
                            "multiple mapping for language {} has character '{}' mapped to ",
                            "\"{}\", but no such mapping exists for the uppercase variant {}.\n",
                            "Did you mean to enable `uppercase_versions = true`?"
                        ),
                        lang,
                        lower,
                        str,
                        upper
                    );
                }
            });
        }
    }
}
