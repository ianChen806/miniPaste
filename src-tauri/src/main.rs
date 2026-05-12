#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let app_data = dirs::config_dir()
        .expect("config dir not available")
        .join("minipaste");
    let log_dir = app_data.join("logs");
    let _guard = minipaste::logging::init(log_dir.clone());
    minipaste::logging::install_panic_handler(log_dir);
    minipaste::run()
}
