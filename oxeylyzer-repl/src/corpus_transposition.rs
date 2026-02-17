use glob::glob;
use oxeylyzer_core::corpus_cleaner::CorpusCleaner;
use serde::Deserialize;

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Default)]
struct Multiple {
    #[serde(default)]
    uppercase_versions: bool,
    list: Vec<[String; 2]>,
}

#[derive(Debug, Clone, Deserialize, Default)]
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

#[derive(Debug, Clone, Deserialize)]
struct CorpusConfigLoad {
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

// TODO: adapt for cleaner
#[derive(Debug, Clone, Default)]
pub struct CorpusConfig {
    // TODO: add cycle detection
    inherits: Vec<String>,
    letters_to_lowercase: String,
    punct_unshifted: OneToOne,
    keep: String,
    to_multiple: Vec<(char, String)>,
    one_to_one: OneToOne,
}

impl CorpusConfig {
    fn new(loaded: CorpusConfigLoad) -> Self {
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

    pub fn load(language: &str, preferred_folder: Option<&str>) -> Result<Self, String> {
        let loaded = CorpusConfigLoad::new(language, preferred_folder)?;

        Ok(Self::new(loaded))
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
        let letters_to_lowercase = self.letters_to_lowercase + &rhs.letters_to_lowercase;
        let punct_unshifted = self.punct_unshifted + rhs.punct_unshifted;
        let keep = self.keep + &rhs.keep;
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
            .with_chars(config.letters_to_lowercase.chars())
            .with_exact_mappings(config.keep.chars())
            .with_char_mappings(
                config
                    .one_to_one
                    .from
                    .chars()
                    .zip(config.one_to_one.to.chars()),
            )
            .with_uppercase_mappings(
                config
                    .punct_unshifted
                    .to
                    .chars()
                    .zip(config.punct_unshifted.from.chars()),
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
    fn parse_config() {
        let config = r#"
            inherits = ["dofsmie"]
            
            letters_to_lowercase = "yeah"
            keep = "keep this"
            
            [multiple]
            list = [
              ["…", "..."],
            ]
            
            [punct_unshifted]
            from = "from"
            to = "to"
            
            [one_to_one]
            from = "from"
            to = "to"
        "#;

        let loaded = toml::from_str::<CorpusConfigLoad>(config).unwrap();

        println!("{loaded:#?}");

        let config = CorpusConfig::new(loaded);

        println!("\n--------------------------\n{config:#?}");
    }
}
