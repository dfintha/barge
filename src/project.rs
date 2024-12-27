use crate::makefile::{generate_analyze_makefile, generate_build_makefile, BuildTarget};
use crate::result::{BargeError, Result};
use crate::scripts::{execute_script, BuildScriptKind, ScriptEnvironment};
use crate::utilities::attempt_remove_directory;
use crate::{color_eprintln, color_println, BLUE, GREEN, NO_COLOR, RED};
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;

pub const DEFAULT_C_STANDARD: &str = "c11";
pub const DEFAULT_CPP_STANDARD: &str = "c++17";
pub const DEFAULT_FORTRAN_STANDARD: &str = "f2003";
pub const DEFAULT_COBOL_STANDARD: &str = "cobol2014";
pub const DEFAULT_TOOLSET: &Toolset = &Toolset::Llvm;
pub const DEFAULT_CUSTOM_CFLAGS: &str = "";
pub const DEFAULT_CUSTOM_CXXFLAGS: &str = "";
pub const DEFAULT_CUSTOM_FORTRANFLAGS: &str = "";
pub const DEFAULT_CUSTOM_COBOLFLAGS: &str = "";
pub const DEFAULT_CUSTOM_LDFLAGS: &str = "";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Toolset {
    Gnu,
    Llvm,
}

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

#[derive(Debug, PartialEq)]
pub(crate) enum CollectSourceFilesMode {
    All,
    CCppSourcesOnly,
    LinkerScriptsOnly,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Project {
    pub name: String,
    pub authors: Vec<String>,
    pub description: String,
    pub project_type: ProjectType,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toolset: Option<Toolset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub c_standard: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpp_standard: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fortran_standard: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cobol_standard: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_libraries: Option<Vec<Library>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_cflags: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_cxxflags: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_fortranflags: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_cobolflags: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_ldflags: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_makeopts: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_build_steps: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_build_steps: Option<Vec<String>>,
}

impl Project {
    pub(crate) fn new(name: &str, project_type: ProjectType) -> Result<Project> {
        Ok(Project {
            name: name.to_string(),
            authors: vec![get_git_user()?],
            description: String::from(""),
            project_type,
            version: String::from("0.1.0"),
            toolset: None,
            c_standard: None,
            cpp_standard: None,
            fortran_standard: None,
            cobol_standard: None,
            external_libraries: None,
            custom_cflags: None,
            custom_cxxflags: None,
            custom_fortranflags: None,
            custom_cobolflags: None,
            custom_ldflags: None,
            custom_makeopts: None,
            format_style: None,
            pre_build_steps: None,
            post_build_steps: None,
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
        let start_timestamp = Local::now();

        let makeopts = if let Some(makeopts) = &self.custom_makeopts {
            makeopts.split(' ').map(|str| str.to_string()).collect()
        } else {
            generate_default_makeopts()?
        };

        let (commit_hash, branch) = get_git_project_info()?;

        if let Some(pre_build_steps) = &self.pre_build_steps {
            for step in pre_build_steps {
                execute_script(
                    step,
                    "prebuild",
                    ScriptEnvironment {
                        target,
                        name: &self.name,
                        version: &self.version,
                        authors: self.authors.join(", "),
                        description: &self.description,
                        git_commit_hash: commit_hash.clone(),
                        git_branch: branch.clone(),
                        build_timestamp: start_timestamp,
                        kind: BuildScriptKind::PreBuildStep,
                        toolset: self.toolset.unwrap_or(*DEFAULT_TOOLSET),
                    },
                )?;
            }
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
            if let Some(post_build_steps) = &self.post_build_steps {
                for step in post_build_steps {
                    execute_script(
                        step,
                        "postbuild",
                        ScriptEnvironment {
                            target,
                            name: &self.name,
                            version: &self.version,
                            authors: self.authors.join(", "),
                            description: &self.description,
                            git_commit_hash: commit_hash.clone(),
                            git_branch: branch.clone(),
                            build_timestamp: start_timestamp,
                            kind: BuildScriptKind::PostBuildStep,
                            toolset: self.toolset.unwrap_or(*DEFAULT_TOOLSET),
                        },
                    )?;
                }
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
        let path = format!("build/{}", target);
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

    pub(crate) fn run(&self, target: BuildTarget, arguments: Vec<String>) -> Result<()> {
        if self.project_type != ProjectType::Executable {
            color_eprintln!("Only binary projects can be run");
            return Ok(());
        }

        self.build(target)?;

        let path = String::from("build/") + &target.to_string() + "/" + &self.name;
        color_println!(BLUE, "Running executable {}", &path);
        Command::new(&path).args(arguments).spawn()?.wait()?;
        Ok(())
    }

    pub(crate) fn debug(&self, target: BuildTarget, arguments: Vec<String>) -> Result<()> {
        if self.project_type != ProjectType::Executable {
            color_eprintln!("Only binary projects can be run");
            return Ok(());
        }

        self.build(target)?;

        let toolset = if let Some(toolset) = &self.toolset {
            toolset
        } else {
            DEFAULT_TOOLSET
        };
        let debugger = get_debugger(toolset);

        let path = String::from("build/") + &target.to_string() + "/" + &self.name;
        color_println!(BLUE, "Running executable {} in the debugger", &path);

        if toolset == &Toolset::Gnu {
            Command::new(debugger)
                .arg("--args")
                .arg(&path)
                .args(arguments)
                .spawn()?
                .wait()?;
        } else {
            Command::new(debugger)
                .arg(&path)
                .arg("--")
                .args(arguments)
                .spawn()?
                .wait()?;
        }

        Ok(())
    }

    pub(crate) fn format(&self) -> Result<()> {
        let sources = collect_source_files(CollectSourceFilesMode::CCppSourcesOnly)?;
        let style_arg = if let Some(format_style) = &self.format_style {
            "--style=".to_string() + format_style
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

    let processor_cores = system.cpus().len() as u64;
    let free_memory_in_kb = system.total_memory() - system.used_memory();
    let free_2g_memory = free_memory_in_kb / (2 * 1024 * 1024);
    let parallel_jobs = std::cmp::max(1, std::cmp::min(processor_cores, free_2g_memory));

    Ok(vec![format!("-j{}", parallel_jobs)])
}

pub(crate) fn collect_source_files(mode: CollectSourceFilesMode) -> Result<Vec<String>> {
    let arguments = match mode {
        CollectSourceFilesMode::All => {
            vec![
                "-name", "*.f90", // FORTRAN Source
                "-o", "-name", "*.cob", // Cobol Source
                "-o", "-name", "*.s", // Assembly Source
                "-o", "-name", "*.ld", // Linker Script
                "-o", "-name", "*.c", // C Source
                "-o", "-name", "*.cpp", // C++ Source
                "-o", "-name", "*.h", // C Header
                "-o", "-name", "*.hpp", // C++ Header
            ]
        }
        CollectSourceFilesMode::CCppSourcesOnly => {
            vec![
                "-name", "*.c", // C Source
                "-o", "-name", "*.cpp", // C++ Source
                "-o", "-name", "*.h", // C Header
                "-o", "-name", "*.hpp", // C++ Header
            ]
        }
        CollectSourceFilesMode::LinkerScriptsOnly => {
            vec!["-name", "*.ld"] // Linker Script
        }
    };

    let find_src = Command::new("find")
        .arg("src")
        .args(vec!["-type", "f"])
        .args(arguments)
        .output()?
        .stdout;

    let mut found: Vec<_> = std::str::from_utf8(&find_src)?.split('\n').collect();
    found.retain(|str| !str.is_empty());
    Ok(found.iter().map(|s| s.to_string()).collect())
}

pub(crate) fn get_toolset_executables(
    toolset: &Toolset,
) -> (&'static str, &'static str, &'static str) {
    match toolset {
        Toolset::Gnu => ("gcc", "g++", "gfortran"),
        Toolset::Llvm => ("clang", "clang++", "gfortran"),
    }
}

fn get_git_user() -> Result<String> {
    Ok(format!(
        "{} <{}>",
        get_git_config_field("user.name")?.trim_end(),
        get_git_config_field("user.email")?.trim_end(),
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

fn get_git_project_info() -> Result<(Option<String>, Option<String>)> {
    let commit_hash = Command::new("git").args(["rev-parse", "HEAD"]).output()?;
    let commit_hash = if commit_hash.status.success() {
        Some(std::str::from_utf8(&commit_hash.stdout)?.to_string())
    } else {
        None
    };

    let branch = Command::new("git")
        .args(["branch", "--show-current"])
        .output()?;
    let branch = if branch.status.success() {
        Some(std::str::from_utf8(&branch.stdout)?.to_string())
    } else {
        None
    };

    Ok((commit_hash, branch))
}

fn get_debugger(toolset: &Toolset) -> &'static str {
    match toolset {
        Toolset::Gnu => "gdb",
        Toolset::Llvm => "lldb",
    }
}
