use std::path::{Path, PathBuf};

#[derive(thiserror::Error, Debug)]
pub enum SaveError {
    #[error("directory does not exist: {0}")]
    DirMissing(PathBuf),
    #[error("directory not writable: {0}")]
    NotWritable(PathBuf),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn validate_writable_dir(path: &Path) -> Result<(), SaveError> {
    if !path.exists() {
        return Err(SaveError::DirMissing(path.to_path_buf()));
    }
    let probe = path.join(".minipaste-write-probe");
    match std::fs::write(&probe, b"") {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            Ok(())
        }
        Err(_) => Err(SaveError::NotWritable(path.to_path_buf())),
    }
}

pub fn write_image_file(path: &Path, bytes: &[u8]) -> Result<(), SaveError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn validate_writable_dir_passes_on_existing_dir() {
        let dir = tempdir().unwrap();
        assert!(validate_writable_dir(dir.path()).is_ok());
    }

    #[test]
    fn validate_writable_dir_fails_on_missing() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("nope");
        assert!(matches!(
            validate_writable_dir(&missing),
            Err(SaveError::DirMissing(_))
        ));
    }

    #[test]
    fn write_image_creates_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a.png");
        write_image_file(&path, b"fake-bytes").unwrap();
        assert!(path.exists());
        assert_eq!(std::fs::read(&path).unwrap(), b"fake-bytes");
    }
}
