use serde_json::Value;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ChromeProfile {
    pub name: String,
    pub directory: String,
}

fn chrome_user_data_dir() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let candidates = [
        home.join(".config/google-chrome"),
        home.join(".config/google-chrome-beta"),
        home.join(".config/google-chrome-unstable"),
    ];

    candidates
        .into_iter()
        .find(|path| path.join("Local State").is_file())
}

pub fn get_chrome_profiles() -> Vec<ChromeProfile> {
    let Some(user_data_dir) = chrome_user_data_dir() else {
        return Vec::new();
    };

    let Ok(contents) = fs::read_to_string(user_data_dir.join("Local State")) else {
        return Vec::new();
    };

    let Ok(json) = serde_json::from_str::<Value>(&contents) else {
        return Vec::new();
    };

    let Some(info_cache) = json
        .get("profile")
        .and_then(|profile| profile.get("info_cache"))
        .and_then(Value::as_object)
    else {
        return Vec::new();
    };

    let mut profiles: Vec<ChromeProfile> = info_cache
        .iter()
        .filter_map(|(directory, info)| {
            if !user_data_dir.join(directory).is_dir() {
                return None;
            }

            let name = info
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(directory)
                .trim();

            Some(ChromeProfile {
                name: if name.is_empty() {
                    directory.clone()
                } else {
                    name.to_string()
                },
                directory: directory.clone(),
            })
        })
        .collect();

    profiles.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    profiles
}
