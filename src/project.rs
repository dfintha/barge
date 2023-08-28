use crate::result::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Library {
    PkgConfig { name: String },
    Manual { cflags: String, ldflags: String },
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectType {
    Executable,
    SharedLibrary,
    StaticLibrary,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Project {
    pub name: String,
    pub project_type: ProjectType,
    pub version: String,
    pub c_standard: String,
    pub cpp_standard: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_libraries: Option<Vec<Library>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_cflags: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_cxxflags: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_ldflags: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_makeopts: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format_style: Option<String>,
}

impl Project {
    pub fn new(name: &str) -> Result<Project> {
        Ok(Project {
            name: name.to_string(),
            project_type: ProjectType::Executable,
            version: String::from("0.1.0"),
            c_standard: String::from("c11"),
            cpp_standard: String::from("c++17"),
            external_libraries: None,
            custom_cflags: None,
            custom_cxxflags: None,
            custom_ldflags: None,
            custom_makeopts: None,
            format_style: None,
        })
    }

    pub fn load(path: &str) -> Result<Project> {
        let json = std::fs::read_to_string(path)?;
        let project: Project = serde_json::from_str(&json)?;
        Ok(project)
    }
}
