use super::model::{Config, ImageFormat, CURRENT_SCHEMA_VERSION};
use std::path::PathBuf;

pub fn default_config() -> Config {
    Config {
        schema_version: CURRENT_SCHEMA_VERSION,
        hotkey: "Ctrl+Shift+S".to_string(),
        paste_pin_hotkey: "Ctrl+Shift+V".to_string(),
        default_save_path: dirs::picture_dir().unwrap_or_else(|| PathBuf::from(".")),
        image_format: ImageFormat::Png,
        jpeg_quality: 90,
    }
}
