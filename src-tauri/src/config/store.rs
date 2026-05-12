use super::defaults::default_config;
use super::model::{Config, CURRENT_SCHEMA_VERSION};
use std::path::{Path, PathBuf};

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub fn load_or_init(path: &Path) -> Result<Config, ConfigError> {
    if !path.exists() {
        let cfg = default_config();
        save(path, &cfg)?;
        return Ok(cfg);
    }
    let raw = std::fs::read_to_string(path)?;
    match serde_json::from_str::<Config>(&raw) {
        Ok(cfg) if cfg.schema_version <= CURRENT_SCHEMA_VERSION => Ok(cfg),
        Ok(_) | Err(_) => {
            // Corrupt or future-versioned → backup + defaults
            let backup_path = path.with_file_name("config.broken.json");
            let _ = std::fs::copy(path, &backup_path);
            let cfg = default_config();
            save(path, &cfg)?;
            Ok(cfg)
        }
    }
}

pub fn save(path: &Path, cfg: &Config) -> Result<(), ConfigError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(cfg)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn config_path(app_config_dir: PathBuf) -> PathBuf {
    app_config_dir.join("config.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_missing_returns_defaults_and_creates_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        let cfg = load_or_init(&path).unwrap();
        assert_eq!(cfg.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(path.exists());
    }

    #[test]
    fn load_corrupt_backs_up_and_returns_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, "{not valid json").unwrap();
        let cfg = load_or_init(&path).unwrap();
        assert_eq!(cfg.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(dir.path().join("config.broken.json").exists());
    }

    #[test]
    fn save_then_load_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut cfg = default_config();
        cfg.hotkey = "Ctrl+Alt+P".into();
        save(&path, &cfg).unwrap();
        let back = load_or_init(&path).unwrap();
        assert_eq!(back.hotkey, "Ctrl+Alt+P");
    }

    #[test]
    fn future_schema_version_returns_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(
            &path,
            r#"{"schema_version":99,"hotkey":"X","default_save_path":".","image_format":"png","jpeg_quality":90}"#,
        )
        .unwrap();
        let cfg = load_or_init(&path).unwrap();
        assert_eq!(cfg.schema_version, CURRENT_SCHEMA_VERSION);
    }
}
