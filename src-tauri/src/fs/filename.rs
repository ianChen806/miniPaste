use chrono::{DateTime, Local};

pub fn screenshot_filename(ts: DateTime<Local>, ext: &str) -> String {
    format!("screenshot-{}.{}", ts.format("%Y%m%d-%H%M%S"), ext)
}

pub fn now_filename(ext: &str) -> String {
    screenshot_filename(Local::now(), ext)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn screenshot_filename_matches_expected_format() {
        let ts = chrono::Local
            .with_ymd_and_hms(2026, 5, 12, 14, 30, 45)
            .unwrap();
        assert_eq!(screenshot_filename(ts, "png"), "screenshot-20260512-143045.png");
        assert_eq!(screenshot_filename(ts, "jpg"), "screenshot-20260512-143045.jpg");
    }
}
