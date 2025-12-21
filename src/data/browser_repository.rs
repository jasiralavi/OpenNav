use gtk4::gio::{AppInfo, DesktopAppInfo};
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
                    // 1. If it has a protocol (://), it's a URL.
                    // 2. If it has dots NO spaces (example.com), treat as domain -> prepend https://
                    // 3. Otherwise (spaces, no dots), treat as SEARCH -> https://google.com/search?q=...
                    
                    let final_arg = if url.contains("://") {
                        url.to_string()
                    } else if url.contains(' ') || !url.contains('.') {
                        // Treat as Search
                        // TODO: Ideally configurable, defaulting to Google
                        let query = url.replace(" ", "+");
                        format!("https://www.google.com/search?q={}", query)
                    } else {
                        // Treat as Domain (e.g. "example.com", "localhost:3000")
                        format!("https://{}", url)
                    };

                    // Append user input
                    command.arg(final_arg);
                    
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
