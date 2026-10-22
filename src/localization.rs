use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path, sync::OnceLock};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguagePack {
    pub locale: String,
    pub name: String,
    pub messages: BTreeMap<String, String>,
}
#[derive(Clone, Debug)]
pub struct Localizer {
    packs: BTreeMap<String, LanguagePack>,
    pub diagnostics: Vec<String>,
}
fn locale_key(lang: &str) -> &str {
    match lang {
        "en-US" => "en",
        "vi-VN" => "vi",
        _ => lang,
    }
}
impl Default for Localizer {
    fn default() -> Self {
        Self::new()
    }
}
impl Localizer {
    pub fn new() -> Self {
        let directory = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("locales")));
        Self::from_directory(directory.as_deref())
    }
    pub fn from_directory(directory: Option<&Path>) -> Self {
        let mut this = Self {
            packs: BTreeMap::new(),
            diagnostics: Vec::new(),
        };
        for source in [
            include_str!("../locales/en.json"),
            include_str!("../locales/vi.json"),
        ] {
            let pack: LanguagePack =
                serde_json::from_str(source).expect("tested embedded language pack");
            this.packs.insert(pack.locale.clone(), pack);
        }
        if let Some(directory) = directory
            && let Ok(entries) = std::fs::read_dir(directory)
        {
            let mut paths: Vec<_> = entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|s| s == "json"))
                .collect();
            paths.sort();
            for path in paths {
                let result = (|| -> anyhow::Result<LanguagePack> {
                    anyhow::ensure!(
                        std::fs::metadata(&path)?.len() <= 1024 * 1024,
                        "language pack exceeds 1 MiB"
                    );
                    let pack: LanguagePack =
                        serde_json::from_str(&std::fs::read_to_string(&path)?)?;
                    anyhow::ensure!(
                        !pack.locale.is_empty()
                            && pack.locale.len() <= 32
                            && pack
                                .locale
                                .chars()
                                .all(|c| c.is_ascii_alphanumeric() || c == '-'),
                        "invalid locale code"
                    );
                    anyhow::ensure!(
                        !pack.name.trim().is_empty()
                            && pack.name.len() <= 100
                            && !pack.name.chars().any(char::is_control),
                        "invalid language name"
                    );
                    let base = &this.packs["en"].messages;
                    for (key, value) in &pack.messages {
                        anyhow::ensure!(base.contains_key(key), "unknown translation key: {key}");
                        anyhow::ensure!(
                            placeholders(value) == placeholders(&base[key]),
                            "placeholder mismatch: {key}"
                        );
                    }
                    Ok(pack)
                })();
                match result {
                    Ok(mut pack) => {
                        pack.locale = locale_key(&pack.locale).to_owned();
                        if let Some(existing) = this.packs.get_mut(&pack.locale) {
                            existing.name = pack.name;
                            existing.messages.extend(pack.messages);
                        } else {
                            this.packs.insert(pack.locale.clone(), pack);
                        }
                    }
                    Err(error) => this.diagnostics.push(format!(
                        "Language pack {}: {error}",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    )),
                }
            }
        }
        this
    }
    pub fn languages(&self) -> Vec<(String, String)> {
        self.packs
            .values()
            .map(|p| {
                (
                    match p.locale.as_str() {
                        "en" => "en-US".into(),
                        "vi" => "vi-VN".into(),
                        _ => p.locale.clone(),
                    },
                    p.name.clone(),
                )
            })
            .collect()
    }
    pub fn text(&self, lang: &str, key: &str) -> String {
        let lang = locale_key(lang);
        self.packs
            .get(lang)
            .and_then(|p| p.messages.get(key))
            .filter(|s| !s.is_empty())
            .or_else(|| self.packs["en"].messages.get(key))
            .cloned()
            .unwrap_or_else(|| key.to_owned())
    }
    pub fn format(&self, lang: &str, key: &str, values: &[(&str, &str)]) -> String {
        // Replace placeholders once, so inserted device names cannot become templates.
        let source = self.text(lang, key);
        let mut out = String::new();
        let mut rest = source.as_str();
        while let Some(start) = rest.find('{') {
            out.push_str(&rest[..start]);
            if let Some(end) = rest[start..].find('}') {
                let name = &rest[start + 1..start + end];
                if let Some((_, v)) = values.iter().find(|(k, _)| *k == name) {
                    out.push_str(v);
                } else {
                    out.push_str(&rest[start..=start + end]);
                }
                rest = &rest[start + end + 1..];
            } else {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
        out.push_str(rest);
        out
    }
    pub fn message(&self, lang: &str, english: &str) -> String {
        // Source diagnostics remain stable for exported evidence and older files.
        let alias = match english {
            "No verified current snapshot evidence" => Some("explanation.no_current"),
            "Review explicit values and device compatibility. Packet Tracer execution support is untested for this suggestion. No commands have been executed." => {
                Some("explanation.review_command")
            }
            _ => None,
        };
        if let Some(key) = alias {
            return self.text(lang, key);
        }
        self.packs["en"]
            .messages
            .iter()
            .find(|(_, value)| value.as_str() == english)
            .map(|(key, _)| self.text(lang, key))
            .unwrap_or_else(|| english.to_owned())
    }
}
fn placeholders(text: &str) -> std::collections::BTreeSet<String> {
    let mut set = std::collections::BTreeSet::new();
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        if let Some(end) = rest[start..].find('}') {
            set.insert(rest[start + 1..start + end].into());
            rest = &rest[start + end + 1..];
        } else {
            break;
        }
    }
    set
}
pub fn vietnamese(message: &str) -> String {
    static CATALOG: OnceLock<Localizer> = OnceLock::new();
    CATALOG.get_or_init(Localizer::new).message("vi", message)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embedded_keys_and_placeholders_match() {
        let en: LanguagePack = serde_json::from_str(include_str!("../locales/en.json")).unwrap();
        let vi: LanguagePack = serde_json::from_str(include_str!("../locales/vi.json")).unwrap();
        assert_eq!(
            en.messages.keys().collect::<Vec<_>>(),
            vi.messages.keys().collect::<Vec<_>>()
        );
        for (key, value) in &en.messages {
            assert_eq!(
                placeholders(value),
                placeholders(&vi.messages[key]),
                "{key}"
            );
        }
    }
    #[test]
    fn legacy_verification_copy_uses_current_bilingual_guidance() {
        let localizer = Localizer::from_directory(None);
        for language in ["en-US", "vi-VN"] {
            assert_eq!(
                localizer.message(language, "No verified current snapshot evidence"),
                localizer.text(language, "explanation.no_current")
            );
        }
    }
    #[test]
    fn extra_languages_discover_and_fall_back() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("fr.json"),
            r#"{"locale":"fr","name":"Français","messages":{"nav.overview":"Vue générale"}}"#,
        )
        .unwrap();
        let l = Localizer::from_directory(Some(dir.path()));
        assert!(l.diagnostics.is_empty());
        assert_eq!(l.text("fr", "nav.overview"), "Vue générale");
        assert_eq!(
            l.text("fr", "nav.checkpoints"),
            l.text("en", "nav.checkpoints")
        );
        assert_eq!(
            l.text("missing", "nav.overview"),
            l.text("en", "nav.overview")
        );
    }
    #[test]
    fn malformed_pack_does_not_break_startup() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("bad.json"), "broken").unwrap();
        let l = Localizer::from_directory(Some(d.path()));
        assert_eq!(l.languages().len(), 2);
        assert_eq!(l.diagnostics.len(), 1);
    }
    #[test]
    fn replacement_does_not_reinterpret_values() {
        let l = Localizer::from_directory(None);
        assert_eq!(
            l.format("en", "action.configure_device", &[("device", "R{device}")]),
            "Configure R{device}"
        );
    }
}
