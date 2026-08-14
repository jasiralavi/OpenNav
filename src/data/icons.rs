use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

pub fn fetch_favicon(domain_or_url: &str) -> anyhow::Result<String> {
    // Extract domain if it is a full url
    let domain = if domain_or_url.contains("://") {
        url::Url::parse(domain_or_url)
            .ok()
            .and_then(|u| u.host_str().map(|s| s.to_string()))
            .unwrap_or(domain_or_url.to_string())
    } else {
        domain_or_url.to_string()
    };

    let icon_dir = dirs::data_dir()
        .unwrap_or(PathBuf::from("."))
        .join("opennav")
        .join("icons");
        
    std::fs::create_dir_all(&icon_dir)?;
    
    // Hash domain to get safe filename
    let filename = format!("{}.png", domain);
    let file_path = icon_dir.join(&filename);
    
    // Check if exists to avoid refetching every time (cache)
    if file_path.exists() {
        return Ok(file_path.to_string_lossy().to_string());
    }

    let url = format!("https://www.google.com/s2/favicons?domain={}&sz=64", domain);
    
    // Blocking request (run in thread if needed by caller)
    let resp = reqwest::blocking::get(url)?;
    let bytes = resp.bytes()?;
    
    let mut file = File::create(&file_path)?;
    file.write_all(&bytes)?;
    
    Ok(file_path.to_string_lossy().to_string())
}

pub fn fetch_missing_icons() {
    // Spawns a thread to fetch icons for engines that lack them
    std::thread::spawn(|| {
        if let Ok(store) = crate::data::store::Store::new() {
            if let Ok(engines) = store.list_engines() {
                for mut engine in engines {
                     if engine.icon_path.is_none() {
                         // Attempt fetch
                         let domain = if engine.url.contains("://") {
                             engine.url.clone()
                         } else {
                             // Fallbacks for known ones if URL is weird? 
                             engine.url.clone()
                         };
                         
                         if let Ok(path) = fetch_favicon(&domain) {
                             engine.icon_path = Some(path);
                             let _ = store.add_engine(&engine); // Update DB
                         }
                     }
                }
            }
        }
    });
}
