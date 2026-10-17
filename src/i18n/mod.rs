//! UI and diagnostics share an exhaustive pair of dictionaries.
mod en;
mod keys;
mod ru;
pub use keys::Key;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[serde(rename = "ru")]
    Russian,
    #[default]
    #[serde(rename = "en", other)]
    English,
}
impl Language {
    pub const fn text(self, key: Key) -> &'static str {
        match self {
            Self::English => en::get(key),
            Self::Russian => ru::get(key),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Message {
    pub key: Key,
    args: Vec<String>,
}
impl Message {
    pub fn with(key: Key, args: impl IntoIterator<Item = impl ToString>) -> Self {
        Self {
            key,
            args: args.into_iter().map(|v| v.to_string()).collect(),
        }
    }
    pub fn render(&self, language: Language) -> String {
        // Process the template once: file names and library errors are opaque data.
        let mut out = String::new();
        let mut rest = language.text(self.key);
        while let Some(start) = rest.find('{') {
            out.push_str(&rest[..start]);
            rest = &rest[start..];
            if let Some(end) = rest.find('}')
                && let Ok(index) = rest[1..end].parse::<usize>()
                && let Some(arg) = self.args.get(index)
            {
                out.push_str(arg);
                rest = &rest[end + 1..];
            } else {
                out.push('{');
                rest = &rest[1..];
            }
        }
        out.push_str(rest);
        out
    }
    pub fn estimated_bytes(&self) -> usize {
        self.args.capacity() * std::mem::size_of::<String>()
            + self.args.iter().map(String::capacity).sum::<usize>()
    }
}
impl From<Key> for Message {
    fn from(key: Key) -> Self {
        Self { key, args: vec![] }
    }
}
impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.render(Language::English))
    }
}
impl std::error::Error for Message {}
impl Serialize for Message {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.render(Language::English))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dictionaries_have_matching_placeholders_and_no_empty_entries() {
        fn placeholders(s: &str) -> Vec<&str> {
            s.split('{')
                .skip(1)
                .map(|p| p.split('}').next().unwrap())
                .collect()
        }
        for &key in Key::ALL {
            let (en, ru) = (Language::English.text(key), Language::Russian.text(key));
            assert!(!en.is_empty() && !ru.is_empty(), "{key:?}");
            assert_eq!(placeholders(en), placeholders(ru), "{key:?}");
        }
    }
    #[test]
    fn substitutions_preserve_untrusted_text_in_both_languages() {
        let message = Message::with(Key::MissingObject, ["{1}", "часть {0}.model"]);
        for lang in [Language::English, Language::Russian] {
            let result = message.render(lang);
            assert!(result.contains("{1}"));
            assert!(result.contains("часть {0}.model"));
        }
    }
}
