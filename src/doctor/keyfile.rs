//! Minimal reader for the glib keyfile (ini) format used by `*.portal` and
//! `portals.conf` files.
//!
//! Only the subset needed by `doctor` is supported: groups, `key=value` pairs,
//! comments and semicolon separated lists.

use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct KeyFile {
    groups: HashMap<String, HashMap<String, String>>,
}

impl KeyFile {
    pub fn parse(content: &str) -> Self {
        let mut groups: HashMap<String, HashMap<String, String>> = HashMap::new();
        let mut current = String::new();

        for line in content.lines() {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            if let Some(group) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                current = group.trim().to_owned();
                groups.entry(current.clone()).or_default();
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                groups
                    .entry(current.clone())
                    .or_default()
                    .insert(key.trim().to_owned(), value.trim().to_owned());
            }
        }

        Self { groups }
    }

    pub fn get(&self, group: &str, key: &str) -> Option<&str> {
        self.groups.get(group)?.get(key).map(String::as_str)
    }

    /// A semicolon separated list, as used by both file formats.
    pub fn get_list(&self, group: &str, key: &str) -> Option<Vec<String>> {
        Some(
            self.get(group, key)?
                .split(';')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_owned)
                .collect(),
        )
    }

    pub fn keys(&self, group: &str) -> Vec<&str> {
        self.groups
            .get(group)
            .map(|entries| entries.keys().map(String::as_str).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::KeyFile;

    #[test]
    fn parses_portal_file() {
        let file = KeyFile::parse(
            "# a comment\n[portal]\nDBusName=org.example.Portal\nInterfaces=org.a;org.b;\n",
        );

        assert_eq!(file.get("portal", "DBusName"), Some("org.example.Portal"));
        assert_eq!(
            file.get_list("portal", "Interfaces"),
            Some(vec!["org.a".to_owned(), "org.b".to_owned()])
        );
        assert_eq!(file.get("portal", "Missing"), None);
    }

    #[test]
    fn parses_portals_conf() {
        let file = KeyFile::parse(
            "[preferred]\ndefault = gtk;gnome\norg.freedesktop.impl.portal.FileChooser=termfilepickers\n",
        );

        assert_eq!(
            file.get_list("preferred", "default"),
            Some(vec!["gtk".to_owned(), "gnome".to_owned()])
        );
        assert_eq!(
            file.get_list("preferred", "org.freedesktop.impl.portal.FileChooser"),
            Some(vec!["termfilepickers".to_owned()])
        );
    }

    #[test]
    fn ignores_keys_outside_known_groups() {
        let file = KeyFile::parse("stray=value\n[preferred]\ndefault=gtk\n");

        assert_eq!(file.get("preferred", "stray"), None);
        assert_eq!(file.get("", "stray"), Some("value"));
    }
}
