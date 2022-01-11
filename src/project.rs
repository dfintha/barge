use crate::result::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum Library {
    PkgConfig { name: String },
    Manual { cflags: String, ldflags: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Project {
    pub name: String,
    pub version: String,
    pub c_standard: String,
    pub cpp_standard: String,
    pub external_libraries: Option<Vec<Library>>,
    pub custom_cflags: Option<String>,
    pub custom_cxxflags: Option<String>,
    pub custom_ldflags: Option<String>,
    pub custom_makeopts: Option<String>,
}

impl Project {
    pub fn load(path: &str) -> Result<Project> {
        let json = std::fs::read_to_string(path)?;
        let project: Project = serde_json::from_str(&json)?;
        Ok(project)
    }
}
