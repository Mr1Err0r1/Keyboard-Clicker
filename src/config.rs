use crate::model::AppSettings;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

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

pub fn export_to(path: &Path, settings: &AppSettings) -> io::Result<()> {
    settings
        .validate()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    let bytes = serde_json::to_vec_pretty(settings)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    fs::write(path, bytes)
}

pub fn import_from(path: &Path) -> io::Result<AppSettings> {
    let contents = fs::read_to_string(path)?;
    let settings: AppSettings = serde_json::from_str(&contents)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    settings
        .validate()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_path_is_inside_the_application_config_folder() {
        assert!(config_path().ends_with("KeyboardClicker/settings.json"));
    }

    #[test]
    fn settings_can_be_exported_and_imported_without_loss() {
        let path =
            std::env::temp_dir().join(format!("keyboard-clicker-{}.json", std::process::id()));
        let expected = AppSettings::default();
        export_to(&path, &expected).unwrap();
        let imported = import_from(&path).unwrap();
        let _ = fs::remove_file(path);
        assert_eq!(imported, expected);
    }
}
