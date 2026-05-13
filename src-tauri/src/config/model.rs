use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub schema_version: u32,
    pub hotkey: String,
    #[serde(default = "default_paste_pin_hotkey")]
    pub paste_pin_hotkey: String,
    pub default_save_path: PathBuf,
    pub image_format: ImageFormat,
    pub jpeg_quality: u8,
}

fn default_paste_pin_hotkey() -> String {
    "Ctrl+Shift+V".to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Png,
    Jpeg,
}

impl ImageFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_serde_roundtrip() {
        let cfg = Config {
            schema_version: 1,
            hotkey: "Ctrl+Shift+S".into(),
            paste_pin_hotkey: "Ctrl+Shift+V".into(),
            default_save_path: PathBuf::from("C:/temp"),
            image_format: ImageFormat::Png,
            jpeg_quality: 90,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn legacy_config_missing_paste_pin_hotkey_uses_default() {
        let json = r#"{
            "schema_version": 1,
            "hotkey": "Ctrl+Shift+S",
            "default_save_path": "C:/temp",
            "image_format": "png",
            "jpeg_quality": 90
        }"#;
        let cfg: Config = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.paste_pin_hotkey, "Ctrl+Shift+V");
    }

    #[test]
    fn image_format_parses_lowercase() {
        let json = r#""jpeg""#;
        assert_eq!(
            serde_json::from_str::<ImageFormat>(json).unwrap(),
            ImageFormat::Jpeg
        );
    }
}
