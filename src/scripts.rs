use crate::makefile::BuildTarget;
use crate::project::{get_toolset_executables, Toolset};
use crate::result::{BargeError, Result};
use crate::NO_COLOR;
use chrono::{DateTime, Local};
use std::collections::HashMap;
use std::process::Command;

enum BuildScriptLanguage {
    ShellScript,
    PythonScript,
    PerlScript,
    CSource,
    CppSource,
}

pub(crate) enum BuildScriptKind {
    PreBuildStep,
    PostBuildStep,
}

pub(crate) struct ScriptEnvironment<'a> {
    pub target: BuildTarget,
    pub name: &'a String,
    pub version: &'a String,
    pub authors: String,
    pub description: &'a String,
    pub git_commit_hash: Option<String>,
    pub git_branch: Option<String>,
    pub build_timestamp: DateTime<Local>,
    pub kind: BuildScriptKind,
    pub toolset: Toolset,
}

impl TryFrom<&str> for BuildScriptLanguage {
    type Error = BargeError;

    fn try_from(extension: &str) -> Result<BuildScriptLanguage> {
        if extension == "sh" {
            Ok(BuildScriptLanguage::ShellScript)
        } else if extension == "py" {
            Ok(BuildScriptLanguage::PythonScript)
        } else if extension == "pl" {
            Ok(BuildScriptLanguage::PerlScript)
        } else if extension == "c" {
            Ok(BuildScriptLanguage::CSource)
        } else if extension == "cpp" {
            Ok(BuildScriptLanguage::CppSource)
        } else {
            Err(BargeError::InvalidValue("Invalid file type"))
        }
    }
}

pub(crate) fn execute_script(path: &str, name: &str, env: ScriptEnvironment) -> Result<()> {
    let kind = BuildScriptLanguage::try_from(get_file_extension(path)?)?;

    let (cc, cxx, _) = get_toolset_executables(&env.toolset);

    match kind {
        BuildScriptLanguage::ShellScript => {
            execute_script_plain(path, "bash", env)?;
        }
        BuildScriptLanguage::PythonScript => {
            execute_script_env(path, "python3", env)?;
        }
        BuildScriptLanguage::PerlScript => {
            execute_script_plain(path, "perl", env)?;
        }
        BuildScriptLanguage::CSource => {
            execute_c_cpp_source(path, name, cc, "-std=c11", env)?;
        }
        BuildScriptLanguage::CppSource => {
            execute_c_cpp_source(path, name, cxx, "-std=c++17", env)?;
        }
    }
    Ok(())
}

fn get_file_extension(path: &str) -> Result<&str> {
    path.split('.')
        .last()
        .ok_or(BargeError::NoneOption("Failed to parse file name"))
}

fn execute_script_plain(path: &str, interpreter: &str, env: ScriptEnvironment) -> Result<()> {
    let interpreter = Command::new(interpreter)
        .arg(path)
        .envs(unpack_script_environment(env))
        .spawn()?
        .wait()?;
    if interpreter.success() {
        Ok(())
    } else {
        Err(BargeError::FailedOperation("Failed to execute script"))
    }
}

fn execute_script_env(path: &str, interpreter: &str, env: ScriptEnvironment) -> Result<()> {
    let interpreter = Command::new("env")
        .arg("-S")
        .arg(interpreter)
        .arg(path)
        .envs(unpack_script_environment(env))
        .spawn()?
        .wait()?;
    if interpreter.success() {
        Ok(())
    } else {
        Err(BargeError::FailedOperation(
            "Failed to execute Python script",
        ))
    }
}

fn execute_c_cpp_source(
    path: &str,
    name: &str,
    compiler: &str,
    std_flag: &str,
    env: ScriptEnvironment,
) -> Result<()> {
    let subdirectory = match env.kind {
        BuildScriptKind::PreBuildStep => "prebuild",
        BuildScriptKind::PostBuildStep => "postbuild",
    };

    let directory = format!("build/{}", subdirectory);
    if !std::path::Path::new(&directory).exists() {
        std::fs::create_dir_all(directory)?;
    }

    let target = format!("build/{}/{}", subdirectory, name);
    if std::path::Path::new(&target).exists() {
        std::fs::remove_file(&target)?;
    }

    let cc = Command::new(compiler)
        .arg(std_flag)
        .arg(path)
        .arg("-o")
        .arg(&target)
        .spawn()?
        .wait()?;
    if !cc.success() {
        return Err(BargeError::FailedOperation(
            "Failed to compile a custom build step binary",
        ));
    }

    let step = Command::new(&target)
        .envs(unpack_script_environment(env))
        .spawn()?
        .wait()?;

    if step.success() {
        Ok(())
    } else {
        Err(BargeError::FailedOperation("Custom build step failed"))
    }
}

fn unpack_script_environment(env: ScriptEnvironment) -> HashMap<String, String> {
    let mut result: HashMap<String, String> = HashMap::new();
    result.insert(String::from("BARGE_PROJECT_NAME"), env.name.to_string());
    result.insert(String::from("BARGE_PROJECT_AUTHORS"), env.authors);
    result.insert(
        String::from("BARGE_PROJECT_DESCRIPTION"),
        env.description.to_string(),
    );
    result.insert(
        String::from("BARGE_PROJECT_VERSION"),
        env.version.to_string(),
    );
    result.insert(String::from("BARGE_BUILD_TARGET"), env.target.to_string());
    result.insert(
        String::from("BARGE_OBJECTS_DIR"),
        format!("build/{}/obj", env.target.to_string()),
    );
    result.insert(
        String::from("BARGE_BINARY_DIR"),
        format!("build/{}", env.target.to_string()),
    );
    result.insert(
        String::from("BARGE_GIT_COMMIT"),
        env.git_commit_hash
            .unwrap_or(String::from(""))
            .trim()
            .to_string(),
    );
    result.insert(
        String::from("BARGE_GIT_BRANCH"),
        env.git_branch
            .unwrap_or(String::from(""))
            .trim()
            .to_string(),
    );
    result.insert(
        String::from("BARGE_BUILD_START_TIMESTAMP"),
        env.build_timestamp.to_rfc3339(),
    );
    result.insert(
        String::from("BARGE_STEP_START_TIMESTAMP"),
        Local::now().to_rfc3339(),
    );
    result.insert(
        String::from("BARGE_BUILD_STEP_KIND"),
        match env.kind {
            BuildScriptKind::PreBuildStep => String::from("prebuild"),
            BuildScriptKind::PostBuildStep => String::from("postbuild"),
        },
    );
    result.insert(
        String::from("BARGE_TOOLSET"),
        match env.toolset {
            Toolset::Llvm => String::from("llvm"),
            Toolset::Gnu => String::from("gnu"),
        },
    );
    if *NO_COLOR {
        result.insert(String::from("NO_COLOR"), String::from("1"));
    }
    result
}
