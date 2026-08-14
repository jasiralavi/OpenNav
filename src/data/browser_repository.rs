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
                    
                    // Smart Argument Handling
                // Smart Argument Handling
                let is_url = url.contains("://");
                let is_search = !is_url && (url.contains(' ') || !url.contains('.'));
                
                if is_search {
                    let mut final_url = String::new();
                    // Check for keyword (e.g. "g query")
                    let parts: Vec<&str> = url.splitn(2, ' ').collect();
                    let mut used_keyword = false;

                    if parts.len() > 1 {
                        let potential_keyword = parts[0];
                        let query = parts[1];
                        
                        // Try resolve by keyword
                        if let Ok(store) = crate::data::store::Store::new() {
                            if let Ok(Some(engine)) = store.get_engine_by_keyword(potential_keyword) {
                                final_url = engine.url.replace("{}", &query.replace(" ", "+"));
                                used_keyword = true;
                            }
                        }
                    }
                    
                    if !used_keyword {
                         // Use Default Engine from Settings
                         let mut engine_url = "https://www.google.com/search?q={}".to_string(); // Fallback
                         
                         if let Ok(store) = crate::data::store::Store::new() {
                             // Get setting (which should now be a KEYWORD like 'g', or legacy name 'Google')
                             let setting = store.get_setting("search_engine").ok().flatten().unwrap_or("g".to_string());
                             
                             // Try to find the engine for this setting
                             // If it matches a legacy name, map it to a keyword manually or checking logic
                             let keyword = match setting.as_str() {
                                 "Google" => "g",
                                 "DuckDuckGo" => "d",
                                 "Bing" => "b",
                                 "Brave" => "br",
                                 "Ecosia" => "e",
                                 k => k,
                             };
                             
                             if let Ok(Some(engine)) = store.get_engine_by_keyword(keyword) {
                                 engine_url = engine.url.clone();
                             } else {
                                 // Fallback if DB lookup fails (shouldn't happen with seeding)
                                 if setting == "DuckDuckGo" { engine_url = "https://duckduckgo.com/?q={}".to_string(); }
                             }
                         }
                         
                         let query_encoded = url.replace(" ", "+");
                         final_url = engine_url.replace("{}", &query_encoded);
                    }

                    command.arg(final_url);
                } else {
                     // It's a URL or Domain
                     let final_url = if is_url {
                         url.to_string()
                     } else {
                         format!("https://{}", url)
                     };
                     command.arg(final_url);
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

fn chrome_profile_target(url: &str) -> Option<String> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }

    let is_url = url.contains("://");
    let is_search = !is_url && (url.contains(' ') || !url.contains('.'));

    if !is_search {
        return Some(if is_url {
            url.to_string()
        } else {
            format!("https://{}", url)
        });
    }

    let parts: Vec<&str> = url.splitn(2, ' ').collect();

    if parts.len() > 1 {
        let potential_keyword = parts[0];
        let query = parts[1];

        if let Ok(store) = crate::data::store::Store::new() {
            if let Ok(Some(engine)) = store.get_engine_by_keyword(potential_keyword) {
                return Some(engine.url.replace("{}", &query.replace(' ', "+")));
            }
        }
    }

    let mut engine_url = "https://www.google.com/search?q={}".to_string();

    if let Ok(store) = crate::data::store::Store::new() {
        let setting = store
            .get_setting("search_engine")
            .ok()
            .flatten()
            .unwrap_or_else(|| "g".to_string());

        let keyword = match setting.as_str() {
            "Google" => "g",
            "DuckDuckGo" => "d",
            "Bing" => "b",
            "Brave" => "br",
            "Ecosia" => "e",
            k => k,
        };

        if let Ok(Some(engine)) = store.get_engine_by_keyword(keyword) {
            engine_url = engine.url;
        }
    }

    Some(engine_url.replace("{}", &url.replace(' ', "+")))
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
