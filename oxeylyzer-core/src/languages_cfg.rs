use ahash::AHashMap as HashMap;
use std::io::Read;

pub fn read_cfg() -> HashMap<String, String> {
    let mut res = HashMap::default();

    if let Ok(mut f) = std::fs::File::open("languages_default.cfg") {
        let mut file_contents = String::new();
        f.read_to_string(&mut file_contents).unwrap();

        for line in file_contents.lines() {
            match parse_line(line) {
                Ok(parsed) => {
                    for lang in parsed.languages {
                        res.insert(lang, parsed.chars.clone());
                    }
                }
                Err(error_msg) => {
                    if !error_msg.is_empty() {
                        println!("{error_msg}")
                    }
                }
            }
        }
    } else {
        println!(
            "No cfg file found! Make sure to have a 'languages_default.cfg' in your root folder"
        );
    }
    res
}

struct LangsChars {
    languages: Vec<String>,
    chars: String,
}

fn parse_line(line: &str) -> Result<LangsChars, String> {
    let line_content = line.split('#').collect::<Vec<&str>>();
    if !line_content.is_empty() && !line_content[0].is_empty() {
        let split_langs_chars = line_content[0].split(':').collect::<Vec<&str>>();
        if split_langs_chars.len() == 2 {
            let langs = split_langs_chars[0]
                .trim()
                .split(',')
                .map(|s| s.trim().to_owned())
                .collect::<Vec<String>>();
            let chars = split_langs_chars[1].trim();
            if !langs.is_empty() {
                let cc = chars.chars().count();
                if cc == 30 {
                    Ok(LangsChars {
                        languages: langs,
                        chars: chars.to_string(),
                    })
                } else {
                    Err(format!(
                        "You specified {cc} characters instead of the required 30 for {langs:?}"
                    ))
                }
            } else {
                Err("No specified language".to_owned())
            }
        } else {
            Err("Either the characters or language is missing".to_owned())
        }
    } else {
        Err("".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cfg() {
        let map = read_cfg();
        println!("{map:#?}");
    }
}
