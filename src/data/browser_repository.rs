use gtk4::gio::AppInfo;
use gtk4::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Browser {
    pub name: String,
    pub command: String,
    pub icon: String,
    pub id: String, // desktop file id
    #[serde(default)]
    pub is_pinned: bool,
}

pub fn get_installed_browsers() -> Vec<Browser> {
    let mut browsers = Vec::new();
    let apps = AppInfo::recommended_for_type("x-scheme-handler/http");
    let apps_https = AppInfo::recommended_for_type("x-scheme-handler/https");
    let mut seen_keys = std::collections::HashSet::new();

    for app in apps.into_iter().chain(apps_https.into_iter()) {
        if let Ok(app_info) = app.downcast::<gtk4::gio::DesktopAppInfo>() {
            if let Some(id) = app_info.id() {
                let id_str = id.to_string();
                let name = app_info.name().to_string();
                let command = app_info.commandline().map(|s| s.display().to_string()).unwrap_or_default();
                let cmd_clean = command.split_whitespace().next().unwrap_or("").to_string();
                let key = format!("{}|{}", name, cmd_clean);

                if seen_keys.contains(&key) {
                    continue;
                }
                seen_keys.insert(key);

                let icon_str = if let Some(icon) = app_info.icon() {
                    icon.to_string().map(|g| g.to_string()).unwrap_or_else(|| "web-browser".to_string())
                } else {
                    "web-browser".to_string()
                };

                browsers.push(Browser {
                    name,
                    command,
                    icon: icon_str,
                    id: id_str,
                    is_pinned: false,
                });
            }
        }
    }

    browsers.retain(|b| b.id != "com.opennav.app" && b.id != "com.opennav.app.desktop");
    browsers.sort_by(|a, b| a.name.cmp(&b.name));
    browsers
}

pub fn is_google_chrome(browser: &Browser) -> bool {
    let id = browser.id.to_lowercase();
    let name = browser.name.to_lowercase();
    id.contains("google-chrome") || name.contains("google chrome")
}

fn final_target(url: &str) -> Option<String> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }

    Some(if url.contains("://") {
        url.to_string()
    } else if url.contains(' ') || !url.contains('.') {
        let query = url.replace(' ', "+");
        format!("https://www.google.com/search?q={}", query)
    } else {
        format!("https://{}", url)
    })
}

fn command_from_app(
    browser_id: &str,
) -> Result<(gtk4::gio::DesktopAppInfo, Vec<String>), Box<dyn std::error::Error>> {
    let app = gtk4::gio::DesktopAppInfo::new(browser_id)
        .ok_or_else(|| format!("Browser {} not found", browser_id))?;

    let cmd_line = app.commandline().ok_or("Browser command line not available")?;
    let cmd_str = cmd_line.to_string_lossy().into_owned();
    let mut args = shlex::split(&cmd_str).ok_or("Unable to parse browser command line")?;
    args.retain(|arg| !arg.starts_with('%'));

    if args.is_empty() {
        return Err("Browser command line is empty".into());
    }

    Ok((app, args))
}

pub fn launch_browser(browser_id: &str, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(app) = gtk4::gio::DesktopAppInfo::new(browser_id) {
        if url.trim().is_empty() {
            let launch_context = gtk4::gio::AppLaunchContext::new();
            app.launch(&[], Some(&launch_context))?;
            return Ok(());
        }
    }

    if let Ok((_app, args)) = command_from_app(browser_id) {
        let mut command = std::process::Command::new(&args[0]);
        for arg in args.iter().skip(1) {
            command.arg(arg);
        }
        if let Some(target) = final_target(url) {
            command.arg(target);
        }
        command.spawn()?;
        return Ok(());
    }

    if let Some(app) = gtk4::gio::DesktopAppInfo::new(browser_id) {
        let launch_context = gtk4::gio::AppLaunchContext::new();
        let uris = vec![url];
        app.launch_uris(&uris, Some(&launch_context))?;
        Ok(())
    } else {
        Err(format!("Browser {} not found", browser_id).into())
    }
}

pub fn launch_chrome_profile(
    browser_id: &str,
    profile_directory: &str,
    url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let (_app, args) = command_from_app(browser_id)?;
    let mut command = std::process::Command::new(&args[0]);

    for arg in args.iter().skip(1) {
        command.arg(arg);
    }

    command.arg(format!("--profile-directory={}", profile_directory));

    if let Some(target) = final_target(url) {
        command.arg(target);
    }

    command.spawn()?;
    Ok(())
}
