use crate::config;
use std::{collections::BTreeMap, fs};

#[derive(Default)]
pub struct Catalog {
    languages: BTreeMap<String, BTreeMap<String, String>>,
}

impl Catalog {
    pub fn load() -> Self {
        let mut catalog = Self::default();
        catalog.insert_json("en", include_str!("../locales/en.json"));
        catalog.insert_json("de", include_str!("../locales/de.json"));
        catalog.insert_json("ru", include_str!("../locales/ru.json"));
        let extra_dir = config::config_dir().join("locales");
        if let Ok(entries) = fs::read_dir(extra_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|value| value.to_str()) != Some("json") {
                    continue;
                }
                let Some(code) = path.file_stem().and_then(|value| value.to_str()) else {
                    continue;
                };
                if valid_locale_code(code) {
                    if let Ok(contents) = fs::read_to_string(&path) {
                        catalog.insert_json(code, &contents);
                    }
                }
            }
        }
        catalog
    }

    fn insert_json(&mut self, code: &str, json: &str) {
        if let Ok(values) = serde_json::from_str::<BTreeMap<String, String>>(json) {
            self.languages.insert(code.to_owned(), values);
        }
    }

    pub fn text(&self, locale: &str, key: &str) -> String {
        self.languages
            .get(locale)
            .and_then(|values| values.get(key))
            .or_else(|| self.languages.get("en").and_then(|values| values.get(key)))
            .cloned()
            .unwrap_or_else(|| key.to_owned())
    }

    pub fn languages(&self) -> Vec<(String, String)> {
        self.languages
            .iter()
            .map(|(code, entries)| {
                (
                    code.clone(),
                    entries
                        .get("language_name")
                        .cloned()
                        .unwrap_or_else(|| code.clone()),
                )
            })
            .collect()
    }
}

fn valid_locale_code(code: &str) -> bool {
    !code.is_empty()
        && code.len() <= 16
        && code.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_languages_have_english_fallback() {
        let catalog = Catalog::load();
        assert_eq!(catalog.text("de", "start"), "Starten");
        assert_eq!(catalog.text("untranslated", "start"), "Start");
    }

    #[test]
    fn locale_code_validation_prevents_path_components() {
        assert!(valid_locale_code("pt-BR"));
        assert!(!valid_locale_code("../evil"));
        assert!(!valid_locale_code(""));
    }
}
