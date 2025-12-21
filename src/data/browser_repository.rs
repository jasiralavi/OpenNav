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
    if let Some(app) = gtk4::gio::DesktopAppInfo::new(browser_id) {
        let launch_context = gtk4::gio::AppLaunchContext::new();
        // The list of files is empty because we are opening a URI, which is handled via URIs list usually
        // But AppInfo::launch_uris looks appropriate.
        let uris = vec![url];
        app.launch_uris(&uris, Some(&launch_context))?;
        Ok(())
    } else {
        Err(format!("Browser {} not found", browser_id).into())
    }
}
