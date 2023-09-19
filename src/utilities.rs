use crate::result::{BargeError, Result};
use std::path::{Path, PathBuf};

pub(crate) fn attempt_remove_directory(path: &str) -> Result<()> {
    let path = Path::new(&path);
    if path.exists() {
        std::fs::remove_dir_all(path)?;
    }
    Ok(())
}

pub(crate) fn look_for_project_directory() -> Result<PathBuf> {
    let mut current = std::env::current_dir()?;
    while current.parent().is_some() {
        current.push("barge.json");
        if current.exists() {
            current.pop();
            return Ok(current);
        } else {
            current.pop();
            current.pop();
        }
    }
    Err(BargeError::ProjectNotFound(
        "Project file not found before reaching filesystem root.",
    ))
}
