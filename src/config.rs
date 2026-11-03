use crate::model::AppSettings;
use std::{fs, io, path::PathBuf};

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("KeyboardClicker")
}

pub fn config_path() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn load() -> AppSettings {
    let path = config_path();
    fs::read_to_string(path)
        .ok()
        .and_then(|data| serde_json::from_str::<AppSettings>(&data).ok())
        .unwrap_or_default()
}

pub fn save(settings: &AppSettings) -> io::Result<()> {
    let dir = config_dir();
    fs::create_dir_all(&dir)?;
    let destination = dir.join("settings.json");
    let temporary = dir.join("settings.json.tmp");
    let bytes = serde_json::to_vec_pretty(settings)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    fs::write(&temporary, bytes)?;
    fs::rename(temporary, destination)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_path_is_inside_the_application_config_folder() {
        assert!(config_path().ends_with("KeyboardClicker/settings.json"));
    }
}
