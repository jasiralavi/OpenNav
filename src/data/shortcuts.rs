use std::collections::{BTreeMap, HashSet};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use super::{
    browser_repository::Browser,
    store::{SearchEngine, Store},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Bookmark {
    pub name: String,
    pub keyword: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LauncherSettings {
    pub chrome_profile_shortcut: String,
    pub browser_shortcuts: BTreeMap<String, String>,
    pub bookmarks: Vec<Bookmark>,
}

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            chrome_profile_shortcut: "cc".into(),
            browser_shortcuts: BTreeMap::new(),
            bookmarks: Vec::new(),
        }
    }
}

pub fn normalize_shortcut(value: &str) -> Result<String> {
    let value = value.trim().to_ascii_lowercase();
    if value.len() != 2 || !value.bytes().all(|c| c.is_ascii_lowercase()) {
        bail!("Shortcuts must contain exactly two letters (a–z).");
    }
    Ok(value)
}

impl LauncherSettings {
    pub fn load(store: &Store) -> Result<Self> {
        match store.get_setting("launcher_settings")? {
            Some(value) => serde_json::from_str(&value).context("Unable to read launcher settings"),
            None => Ok(Self::default()),
        }
    }

    pub fn normalized(&self, engines: &[SearchEngine]) -> Result<Self> {
        let mut result = self.clone();
        let mut used: HashSet<String> = engines
            .iter()
            .map(|e| e.keyword.to_ascii_lowercase())
            .collect();
        result.chrome_profile_shortcut = normalize_shortcut(&result.chrome_profile_shortcut)?;
        if !used.insert(result.chrome_profile_shortcut.clone()) {
            bail!("The Chrome-profile shortcut conflicts with a search-engine alias.");
        }
        for shortcut in result.browser_shortcuts.values_mut() {
            *shortcut = normalize_shortcut(shortcut)?;
            if !used.insert(shortcut.clone()) {
                bail!("Shortcut ‘{shortcut}’ is already used by another browser, Chrome profiles, or a search engine.");
            }
        }
        let mut keywords = HashSet::new();
        for bookmark in &mut result.bookmarks {
            bookmark.name = bookmark.name.trim().into();
            bookmark.keyword = bookmark
                .keyword
                .trim()
                .trim_start_matches('.')
                .to_ascii_lowercase();
            bookmark.url = bookmark.url.trim().into();
            if bookmark.name.is_empty()
                || bookmark.keyword.is_empty()
                || !bookmark
                    .keyword
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
            {
                bail!("Bookmarks need a name and a keyword containing letters, numbers, hyphens or underscores.");
            }
            if !keywords.insert(bookmark.keyword.clone()) {
                bail!("Bookmark keyword ‘.{}’ is already used.", bookmark.keyword);
            }
            let url = url::Url::parse(&bookmark.url)
                .context("Enter a complete bookmark URL, including https://")?;
            if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
                bail!("Bookmark URLs must start with http:// or https:// and include a host.");
            }
        }
        Ok(result)
    }

    pub fn save(&self, store: &Store) -> Result<Self> {
        let normalized = self.normalized(&store.list_engines()?)?;
        store.set_setting("launcher_settings", &serde_json::to_string(&normalized)?)?;
        Ok(normalized)
    }

    /// Assign stable, unused shortcuts without changing existing user choices or aliases.
    pub fn ensure_browsers(
        &mut self,
        browsers: &[Browser],
        engines: &[SearchEngine],
    ) -> Result<()> {
        let mut used: HashSet<String> = engines
            .iter()
            .map(|e| e.keyword.to_ascii_lowercase())
            .collect();
        used.extend(self.browser_shortcuts.values().cloned());
        // An existing search alias always wins over the initial profile default.
        if used.contains(&self.chrome_profile_shortcut) && self.browser_shortcuts.is_empty() {
            self.chrome_profile_shortcut = available_shortcut("cp", &used)?;
        }
        used.insert(self.chrome_profile_shortcut.clone());
        let mut sorted = browsers.iter().collect::<Vec<_>>();
        sorted.sort_by(|a, b| a.id.cmp(&b.id));
        for browser in sorted {
            if self.browser_shortcuts.contains_key(&browser.id) {
                continue;
            }
            let name = format!("{} {}", browser.id, browser.name).to_ascii_lowercase();
            let preferred = if name.contains("firefox") {
                "fx"
            } else if name.contains("edge") {
                "me"
            } else if name.contains("google-chrome") {
                "gc"
            } else if name.contains("chromium") {
                "cr"
            } else if name.contains("brave") {
                "bv"
            } else if name.contains("vivaldi") {
                "vi"
            } else if name.contains("opera") {
                "op"
            } else {
                "wb"
            };
            let shortcut = available_shortcut(preferred, &used)?;
            used.insert(shortcut.clone());
            self.browser_shortcuts.insert(browser.id.clone(), shortcut);
        }
        Ok(())
    }

    pub fn browser_for_shortcut(&self, shortcut: &str) -> Option<&str> {
        self.browser_shortcuts
            .iter()
            .find(|(_, value)| value.eq_ignore_ascii_case(shortcut))
            .map(|(id, _)| id.as_str())
    }

    pub fn reserves(&self, keyword: &str) -> bool {
        self.chrome_profile_shortcut.eq_ignore_ascii_case(keyword)
            || self.browser_for_shortcut(keyword).is_some()
            || keyword.starts_with('.')
    }
}

fn available_shortcut(preferred: &str, used: &HashSet<String>) -> Result<String> {
    if !used.contains(preferred) {
        return Ok(preferred.into());
    }
    for a in b'a'..=b'z' {
        for b in b'a'..=b'z' {
            let candidate = format!("{}{}", a as char, b as char);
            if !used.contains(&candidate) {
                return Ok(candidate);
            }
        }
    }
    bail!("All two-letter shortcuts are in use.")
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn engine(keyword: &str) -> SearchEngine {
        SearchEngine {
            keyword: keyword.into(),
            name: keyword.into(),
            url: "https://example.com/?q={}".into(),
            icon_path: None,
        }
    }

    #[test]
    fn profile_default_normalization_and_collisions() {
        let mut config = LauncherSettings::default();
        assert_eq!(config.chrome_profile_shortcut, "cc");
        config.chrome_profile_shortcut = " CP ".into();
        assert_eq!(
            config.normalized(&[]).unwrap().chrome_profile_shortcut,
            "cp"
        );
        assert!(config.normalized(&[engine("cp")]).is_err());
        for invalid in ["", "x", "abc", "éx", "c1", "c-"] {
            assert!(normalize_shortcut(invalid).is_err());
        }
        config
            .browser_shortcuts
            .insert("chrome.desktop".into(), "cp".into());
        assert!(config.normalized(&[]).is_err());
    }
    #[test]
    fn settings_round_trip_and_failed_save_preserve_data() {
        let store = Store::in_memory();
        let mut config = LauncherSettings::load(&store).unwrap();
        config.chrome_profile_shortcut = " PP ".into();
        config = config.save(&store).unwrap();
        assert_eq!(
            LauncherSettings::load(&store)
                .unwrap()
                .chrome_profile_shortcut,
            "pp"
        );
        config.chrome_profile_shortcut = "gh".into();
        assert!(config.save(&store).is_err());
        assert_eq!(
            LauncherSettings::load(&store)
                .unwrap()
                .chrome_profile_shortcut,
            "pp"
        );
        assert!(store.add_engine(&engine("PP")).is_err());
        assert!(store.add_engine(&engine("gh")).is_err());
        assert!(store.get_engine_by_keyword("gh").unwrap().is_some());
    }

    #[test]
    fn bookmarks_validate_persist_edit_and_delete() {
        let store = Store::in_memory();
        let mut config = LauncherSettings::default();
        config.bookmarks.push(Bookmark {
            name: " Dashboard ".into(),
            keyword: " .DP ".into(),
            url: "https://example.com/?a=1&b=2".into(),
        });
        config = config.save(&store).unwrap();
        assert_eq!(config.bookmarks[0].keyword, "dp");
        assert_eq!(
            LauncherSettings::load(&store).unwrap().bookmarks,
            config.bookmarks
        );
        config.bookmarks.push(config.bookmarks[0].clone());
        assert!(config.save(&store).is_err());
        config.bookmarks.pop();
        for url in ["example.com", "javascript:alert(1)", "file:///tmp/foo"] {
            let mut invalid = config.clone();
            invalid.bookmarks[0].url = url.into();
            assert!(invalid.save(&store).is_err());
        }
        config.bookmarks[0].keyword = "work".into();
        config.save(&store).unwrap();
        assert_eq!(
            LauncherSettings::load(&store).unwrap().bookmarks[0].keyword,
            "work"
        );
        config.bookmarks.clear();
        config.save(&store).unwrap();
        assert!(LauncherSettings::load(&store).unwrap().bookmarks.is_empty());
    }
}
