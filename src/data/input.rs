use super::{browser_repository::Browser, shortcuts::LauncherSettings, store::SearchEngine};
use anyhow::{bail, Result};

#[derive(Debug, PartialEq)]
pub struct LaunchRequest {
    pub browser_id: String,
    pub target: String,
}

/// Remove a trailing browser selector before interpreting aliases or bookmarks.
pub fn resolve_request(
    input: &str,
    selected_browser: &str,
    browsers: &[Browser],
    config: &LauncherSettings,
    engines: &[SearchEngine],
    default_engine: &str,
) -> Result<LaunchRequest> {
    let mut text = input.trim();
    let mut browser_id = selected_browser;
    let last = text.split_whitespace().last().unwrap_or_default();
    if last.len() == 3
        && last.starts_with('-')
        && last[1..].bytes().all(|c| c.is_ascii_alphabetic())
    {
        browser_id = config.browser_for_shortcut(&last[1..]).ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown browser selector ‘{last}’. Check Settings → Browser Shortcuts."
            )
        })?;
        text = text[..text.len() - last.len()].trim_end();
    }
    if !browsers.iter().any(|b| b.id == browser_id) {
        bail!("The selected browser is no longer installed.");
    }
    let target = if let Some(keyword) = text.strip_prefix('.') {
        config
            .bookmarks
            .iter()
            .find(|b| b.keyword.eq_ignore_ascii_case(keyword))
            .ok_or_else(|| {
                anyhow::anyhow!("Unknown bookmark ‘{text}’. Add it in Settings → Bookmarks.")
            })?
            .url
            .clone()
    } else {
        resolve_target(text, engines, default_engine)
    };
    Ok(LaunchRequest {
        browser_id: browser_id.into(),
        target,
    })
}

pub fn resolve_target(input: &str, engines: &[SearchEngine], default_engine: &str) -> String {
    let input = input.trim();
    if input.is_empty() {
        return String::new();
    }
    if let Some((alias, query)) = input.split_once(char::is_whitespace) {
        if let Some(engine) = engines.iter().find(|e| e.keyword == alias) {
            return search_url(&engine.url, query.trim_start());
        }
    }
    if input.contains("://") {
        return input.into();
    }
    if !input.chars().any(char::is_whitespace) && input.contains('.') {
        return format!("https://{input}");
    }
    let keyword = match default_engine {
        "Google" => "g",
        "DuckDuckGo" => "d",
        "Bing" => "b",
        "Brave" => "br",
        "Ecosia" => "e",
        k => k,
    };
    let template = engines
        .iter()
        .find(|e| e.keyword == keyword)
        .map(|e| e.url.as_str())
        .unwrap_or("https://www.google.com/search?q={}");
    search_url(template, input)
}

fn search_url(template: &str, query: &str) -> String {
    let encoded: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
    template.replace("{}", &encoded).replace("%s", &encoded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::shortcuts::Bookmark;

    fn fixtures() -> (Vec<Browser>, LauncherSettings, Vec<SearchEngine>) {
        let browsers = ["Firefox", "Microsoft Edge", "Brave"]
            .iter()
            .map(|name| Browser {
                name: name.to_string(),
                id: format!("{name}.desktop"),
                command: String::new(),
                icon: String::new(),
                is_pinned: false,
            })
            .collect::<Vec<_>>();
        let engines = ["g", "br"]
            .iter()
            .map(|kw| SearchEngine {
                keyword: kw.to_string(),
                name: kw.to_string(),
                url: format!("https://{kw}.example/search?q={{}}"),
                icon_path: None,
            })
            .collect::<Vec<_>>();
        let mut config = LauncherSettings::default();
        config.ensure_browsers(&browsers, &engines).unwrap();
        (browsers, config, engines)
    }

    #[test]
    fn browser_defaults_are_unique_and_preserve_aliases() {
        let (browsers, mut config, engines) = fixtures();
        assert_eq!(config.browser_shortcuts["Firefox.desktop"], "fx");
        assert_eq!(config.browser_shortcuts["Microsoft Edge.desktop"], "me");
        assert_ne!(config.browser_shortcuts["Brave.desktop"], "br");
        config
            .browser_shortcuts
            .insert("Firefox.desktop".into(), "ff".into());
        config.ensure_browsers(&browsers, &engines).unwrap();
        assert_eq!(config.browser_shortcuts["Firefox.desktop"], "ff");
        config
            .browser_shortcuts
            .insert("Brave.desktop".into(), "FF".into());
        assert!(config.normalized(&engines).is_err());
    }

    #[test]
    fn browser_selectors_compose_with_search_aliases_and_urls() {
        let (browsers, config, engines) = fixtures();
        for (input, browser, target) in [
            (
                "g rust gtk -fx",
                "Firefox.desktop",
                "https://g.example/search?q=rust+gtk",
            ),
            (
                "br rust gtk -me",
                "Microsoft Edge.desktop",
                "https://br.example/search?q=rust+gtk",
            ),
            (
                "  g\trust & gtk  -FX  ",
                "Firefox.desktop",
                "https://g.example/search?q=rust+%26+gtk",
            ),
            ("example.com -fx", "Firefox.desktop", "https://example.com"),
            (
                "https://example.com/?a=1&b=2 -me",
                "Microsoft Edge.desktop",
                "https://example.com/?a=1&b=2",
            ),
            ("-fx", "Firefox.desktop", ""),
            (
                "g rust gtk",
                "Brave.desktop",
                "https://g.example/search?q=rust+gtk",
            ),
        ] {
            let r =
                resolve_request(input, "Brave.desktop", &browsers, &config, &engines, "g").unwrap();
            assert_eq!(
                r,
                LaunchRequest {
                    browser_id: browser.into(),
                    target: target.into()
                }
            );
        }
        assert!(resolve_request(
            "g rust -zz",
            "Brave.desktop",
            &browsers,
            &config,
            &engines,
            "g"
        )
        .is_err());
        assert!(resolve_request(
            "g rust -fx",
            "Brave.desktop",
            &browsers[1..],
            &config,
            &engines,
            "g"
        )
        .is_err());
    }

    #[test]
    fn bookmarks_compose_with_selectors() {
        let (browsers, mut config, engines) = fixtures();
        config.bookmarks.push(Bookmark {
            name: "Dashboard".into(),
            keyword: "dp".into(),
            url: "https://example.com/dashboard?a=1&b=2".into(),
        });
        for (input, browser) in [
            (".dp", "Brave.desktop"),
            (".dp -fx", "Firefox.desktop"),
            (".DP -me", "Microsoft Edge.desktop"),
        ] {
            let r =
                resolve_request(input, "Brave.desktop", &browsers, &config, &engines, "g").unwrap();
            assert_eq!(r.browser_id, browser);
            assert_eq!(r.target, config.bookmarks[0].url);
        }
        assert!(resolve_request(
            ".missing -fx",
            "Brave.desktop",
            &browsers,
            &config,
            &engines,
            "g"
        )
        .is_err());
    }

    #[test]
    fn search_templates_encode_queries_and_preserve_ampersands() {
        let engines = vec![SearchEngine {
            keyword: "rz".into(),
            name: "RhymeZone".into(),
            url: "https://www.rhymezone.com/r/rhyme.cgi?Word=%s&typeofrhyme=perfect".into(),
            icon_path: None,
        }];
        assert_eq!(
            resolve_target("rz cats & dogs+#", &engines, "g"),
            "https://www.rhymezone.com/r/rhyme.cgi?Word=cats+%26+dogs%2B%23&typeofrhyme=perfect"
        );
    }
}
