use crate::makefile::{generate_analyze_makefile, generate_build_makefile, BuildTarget};
use crate::result::{BargeError, Result};
use crate::scripts::{execute_script, ScriptEnvironment};
use crate::utilities::attempt_remove_directory;
use crate::{color_eprintln, color_println, BLUE, GREEN, NO_COLOR, RED};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;
use sysinfo::SystemExt;

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
    pub authors: Vec<String>,
    pub description: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_build_step: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_build_step: Option<String>,
}

impl Project {
    pub(crate) fn new(name: &str, project_type: ProjectType) -> Result<Project> {
        Ok(Project {
            name: name.to_string(),
            authors: vec![get_git_user()?],
            description: String::from(""),
            project_type,
            version: String::from("0.1.0"),
            c_standard: String::from("c11"),
            cpp_standard: String::from("c++17"),
            external_libraries: None,
            custom_cflags: None,
            custom_cxxflags: None,
            custom_ldflags: None,
            custom_makeopts: None,
            format_style: None,
            pre_build_step: None,
            post_build_step: None,
        })
    }

    pub(crate) fn load(path: &str) -> Result<Project> {
        let json = std::fs::read_to_string(path)?;
        let project: Project = serde_json::from_str(&json)?;
        Ok(project)
    }

    pub(crate) fn build(&self, target: BuildTarget) -> Result<()> {
        color_println!(
            BLUE,
            "Building project with {} configuration",
            target.to_string()
        );
        let start_time = Instant::now();

        let makeopts = if let Some(makeopts) = &self.custom_makeopts {
            makeopts.split(' ').map(|str| str.to_string()).collect()
        } else {
            generate_default_makeopts()?
        };

        if let Some(pre_build_step) = &self.pre_build_step {
            execute_script(
                pre_build_step,
                "prebuild",
                ScriptEnvironment {
                    target,
                    name: &self.name,
                    version: &self.version,
                    authors: self.authors.join(", "),
                    description: &self.description,
                },
            )?;
        }

        let mut make = Command::new("make")
            .arg("-s")
            .arg("-f")
            .arg("-")
            .arg("all")
            .args(makeopts)
            .stdin(Stdio::piped())
            .spawn()?;

        let makefile = generate_build_makefile(self, target)?;
        make.stdin
            .as_mut()
            .ok_or(BargeError::NoneOption("Could not interact with make"))?
            .write_all(makefile.as_bytes())?;
        let status = make.wait()?.success();

        if status {
            if let Some(post_build_step) = &self.post_build_step {
                execute_script(
                    post_build_step,
                    "postbuild",
                    ScriptEnvironment {
                        target,
                        name: &self.name,
                        version: &self.version,
                        authors: self.authors.join(", "),
                        description: &self.description,
                    },
                )?;
            }

            let finish_time = Instant::now();
            let build_duration = finish_time - start_time;
            color_println!(
                BLUE,
                "Build finished in {:.2} seconds",
                build_duration.as_secs_f64()
            );

            Ok(())
        } else {
            color_eprintln!("Build failed");
            Err(BargeError::FailedOperation(
                "One or more dependencies failed to build",
            ))
        }
    }

    pub(crate) fn rebuild(&self, target: BuildTarget) -> Result<()> {
        color_println!(BLUE, "{}", "Removing relevant build artifacts");
        let path = format!("build/{}", target.to_string());
        attempt_remove_directory(&path)?;
        self.build(target)
    }

    pub(crate) fn analyze(&self) -> Result<()> {
        color_println!(BLUE, "Running static analysis on project");

        let mut make = Command::new("make")
            .arg("-s")
            .arg("-f")
            .arg("-")
            .arg("analyze")
            .stdin(Stdio::piped())
            .spawn()?;

        let makefile = generate_analyze_makefile(self)?;

        make.stdin
            .as_mut()
            .ok_or(BargeError::NoneOption("Could not interact with make"))?
            .write_all(makefile.as_bytes())?;
        make.wait()?;

        Ok(())
    }

    pub(crate) fn run(&self, target: BuildTarget) -> Result<()> {
        if self.project_type != ProjectType::Executable {
            color_eprintln!("Only binary projects can be run");
            return Ok(());
        }

        self.build(target)?;

        let path = String::from("bin/") + &target.to_string() + "/" + &self.name;
        color_println!(BLUE, "Running executable {}", &path);
        Command::new(&path).spawn()?.wait()?;
        Ok(())
    }

    pub(crate) fn format(&self) -> Result<()> {
        let sources = collect_source_files()?;
        let style_arg = if let Some(format_style) = &self.format_style {
            "--style=".to_string() + &format_style
        } else {
            "--style=Google".to_string()
        };

        Command::new("clang-format")
            .arg("-i")
            .arg(style_arg)
            .args(sources)
            .spawn()?
            .wait()?;

        color_println!(BLUE, "The project source files were formatted");
        Ok(())
    }

    pub(crate) fn document(&self) -> Result<()> {
        color_println!(BLUE, "Generating project documentation");
        if !Path::new("Doxyfile").exists() {
            return Err(BargeError::FailedOperation(
                "Doxyfile is missing from the project directory",
            ));
        }

        let doxygen = Command::new("doxygen")
            .arg("Doxyfile")
            .env("BARGE_PROJECT_NAME", &self.name)
            .env("BARGE_PROJECT_VERSION", &self.version)
            .spawn()?
            .wait()?;
        if doxygen.success() {
            color_println!(GREEN, "Project documentation successfully generated");
            Ok(())
        } else {
            Err(BargeError::FailedOperation(
                "Failed to generate documentation using doxygen",
            ))
        }
    }
}

fn generate_default_makeopts() -> Result<Vec<String>> {
    let mut system = sysinfo::System::new_all();
    system.refresh_all();

    let processor_cores = system.processors().len() as u64;
    let free_memory_in_kb = system.total_memory() - system.used_memory();
    let free_2g_memory = free_memory_in_kb / (2 * 1024 * 1024);
    let parallel_jobs = std::cmp::max(1, std::cmp::min(processor_cores, free_2g_memory));

    Ok(vec![format!("-j{}", parallel_jobs)])
}

pub(crate) fn collect_source_files() -> Result<Vec<String>> {
    let find_src = Command::new("find")
        .arg("src")
        .args(vec!["-type", "f"])
        .args(vec!["-name", "*.c"])
        .args(vec!["-o", "-name", "*.cpp"])
        .args(vec!["-o", "-name", "*.s"])
        .args(vec!["-o", "-name", "*.h"])
        .args(vec!["-o", "-name", "*.hpp"])
        .output()?
        .stdout;

    let mut find_src: Vec<_> = std::str::from_utf8(&find_src)?.split('\n').collect();

    let find_include = Command::new("find")
        .arg("include")
        .args(vec!["-type", "f"])
        .args(vec!["-name", "*.h"])
        .args(vec!["-o", "-name", "*.hpp"])
        .output()?
        .stdout;

    let mut found: Vec<_> = std::str::from_utf8(&find_include)?.split('\n').collect();
    found.append(&mut find_src);
    found.retain(|str| !str.is_empty());

    Ok(found.iter().map(|s| s.to_string()).collect())
}

fn get_git_user() -> Result<String> {
    Ok(format!(
        "{} <{}>",
        get_git_config_field("user.name")?,
        get_git_config_field("user.email")?,
    ))
}

fn get_git_config_field(field: &str) -> Result<String> {
    let result = Command::new("git")
        .arg("config")
        .arg("--get")
        .arg(field)
        .output()?
        .stdout;
    Ok(std::str::from_utf8(&result)?.to_string())
}
