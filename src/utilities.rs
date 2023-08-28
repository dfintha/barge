use crate::result::Result;
use std::path::Path;

pub(crate) fn attempt_remove_directory(path: &str) -> Result<()> {
    let path = Path::new(&path);
    if path.exists() {
        std::fs::remove_dir_all(path)?;
    }
    Ok(())
}
