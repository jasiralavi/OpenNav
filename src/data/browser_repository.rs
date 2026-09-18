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
    // recommended_for_type returns Vec<AppInfo> directly (or similar list)
    let apps = AppInfo::recommended_for_type("x-scheme-handler/http");
    
    // Also try https to catch edge cases
    let apps_https = AppInfo::recommended_for_type("x-scheme-handler/https");
    

    let mut seen_keys = std::collections::HashSet::new();
    
    for app in apps.into_iter().chain(apps_https.into_iter()) {
         // AppInfo is a struct/wrapper, Cast trait needed.
         if let Ok(app_info) = app.downcast::<gtk4::gio::DesktopAppInfo>() {
             if let Some(id) = app_info.id() {
                 let id_str = id.to_string();
                 let name = app_info.name().to_string();
                 let command = app_info.commandline().map(|s| s.display().to_string()).unwrap_or_default();
                 
                 // Deduplicate by (Name, Command Executable)
                 // This avoids merging different Flatpaks (same "flatpak" executable, different Name)
                 // while still merging identical entries (same Name, same Executable).
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

                 let b = Browser {
                     name,
                     command,
                     icon: icon_str.clone(),
                     id: id_str,
                     is_pinned: false,
                 };
                 browsers.push(b);
             }
         }
    }
    
    // Filter out our own app if detected
    browsers.retain(|b| b.id != "com.opennav.app" && b.id != "com.opennav.app.desktop");

    // Sort alphabetically by default
    browsers.sort_by(|a, b| a.name.cmp(&b.name));
    browsers
}

pub fn launch_browser(browser_id: &str, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Trim input
    let url = url.trim();

    if let Some(app) = gtk4::gio::DesktopAppInfo::new(browser_id) {
        // CASE 1: Empty URL -> just launch the app
        if url.is_empty() {
            let launch_context = gtk4::gio::AppLaunchContext::new();
            app.launch(&[], Some(&launch_context))?;
            return Ok(());
        }

        // CASE 2: Non-empty URL -> Try raw command line to support "default search" and proper CLI behavior
        if let Some(cmd_line) = app.commandline() {
            let cmd_str = cmd_line.to_string_lossy().into_owned();
            // Split using shlex to handle quotes correctly
            if let Some(mut args) = shlex::split(&cmd_str) {
                // Filter out %u, %U, %f, %F parameters
                args.retain(|arg| !arg.starts_with('%'));

                if let Some(binary) = args.first() {
                    let mut command = std::process::Command::new(binary);
                    // Add remaining args (e.g. "run", "org.mozilla.firefox" for flatpaks)
                    for arg in args.iter().skip(1) {
                        command.arg(arg);
                    }
                    
                    if let Some(target) = chrome_profile_target(url) {
                        command.arg(target);
                    }

                    // Detach process
                    let _ = command.spawn().map_err(|e| format!("Failed to spawn command: {}", e))?;
                    return Ok(());
                }
            }
        }
        
        // Fallback: Use launch_uris if raw command extraction fails (should rarely happen)
        // Note: launch_uris requires valid generic URIs, so "search query" might fail here.
        let launch_context = gtk4::gio::AppLaunchContext::new();
        let uris = vec![url];
        app.launch_uris(&uris, Some(&launch_context))?;
        Ok(())
    } else {
        Err(format!("Browser {} not found", browser_id).into())
    }
}


pub fn is_google_chrome(browser: &Browser) -> bool {
    let id = browser.id.to_lowercase();
    let name = browser.name.to_lowercase();
    id.contains("google-chrome") || name.contains("google chrome")
}

fn chrome_profile_target(input: &str) -> Option<String> {
    if input.trim().is_empty() { return None; }
    let store = crate::data::store::Store::new().ok();
    let engines = store.as_ref().and_then(|s| s.list_engines().ok()).unwrap_or_default();
    let default = store.as_ref().and_then(|s| s.get_setting("search_engine").ok().flatten()).unwrap_or_else(|| "g".into());
    Some(crate::data::input::resolve_target(input, &engines, &default))
}

pub fn launch_chrome_profile(
    browser_id: &str,
    profile_directory: &str,
    url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = gtk4::gio::DesktopAppInfo::new(browser_id)
        .ok_or_else(|| format!("Browser {} not found", browser_id))?;

    let cmd_line = app
        .commandline()
        .ok_or("Chrome command line not available")?;

    let cmd_str = cmd_line.to_string_lossy().into_owned();
    let mut args = shlex::split(&cmd_str).ok_or("Unable to parse Chrome command line")?;
    args.retain(|arg| !arg.starts_with('%'));

    if args.is_empty() {
        return Err("Chrome command line is empty".into());
    }

    let mut command = std::process::Command::new(&args[0]);

    for arg in args.iter().skip(1) {
        command.arg(arg);
    }

    command.arg(format!("--profile-directory={}", profile_directory));

    if let Some(target) = chrome_profile_target(url) {
        command.arg(target);
    }

    command.spawn()?;
    Ok(())
}
