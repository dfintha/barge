use crate::makefile::BuildTarget;
use crate::project::{get_toolset_executables, Toolset};
use crate::result::{BargeError, Result};
use crate::NO_COLOR;
use std::collections::HashMap;
use std::process::Command;

enum ScriptKind {
    ShellScript,
    PythonScript,
    PerlScript,
    CSource,
    CppSource,
}

pub(crate) struct ScriptEnvironment<'a> {
    pub target: BuildTarget,
    pub name: &'a String,
    pub version: &'a String,
    pub authors: String,
    pub description: &'a String,
    pub toolset: Toolset,
}

impl TryFrom<&str> for ScriptKind {
    type Error = BargeError;

    fn try_from(extension: &str) -> Result<ScriptKind> {
        if extension == "sh" {
            Ok(ScriptKind::ShellScript)
        } else if extension == "py" {
            Ok(ScriptKind::PythonScript)
        } else if extension == "pl" {
            Ok(ScriptKind::PerlScript)
        } else if extension == "c" {
            Ok(ScriptKind::CSource)
        } else if extension == "cpp" {
            Ok(ScriptKind::CppSource)
        } else {
            Err(BargeError::InvalidValue("Invalid file type"))
        }
    }
}

pub(crate) fn execute_script(path: &str, name: &str, env: ScriptEnvironment) -> Result<()> {
    let kind = ScriptKind::try_from(get_file_extension(path)?)?;

    let (cc, cxx, _) = get_toolset_executables(&env.toolset);

    match kind {
        ScriptKind::ShellScript => {
            execute_script_plain(path, "bash", env)?;
        }
        ScriptKind::PythonScript => {
            execute_script_env(path, "python3", env)?;
        }
        ScriptKind::PerlScript => {
            execute_script_plain(path, "perl", env)?;
        }
        ScriptKind::CSource => {
            execute_c_cpp_source(path, name, cc, "-std=c11", env)?;
        }
        ScriptKind::CppSource => {
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
    if !std::path::Path::new("bin").exists() {
        std::fs::create_dir("bin")?;
    }

    let target = format!("bin/{}", name);

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
    if *NO_COLOR {
        result.insert(String::from("NO_COLOR"), String::from("1"));
    }
    result
}
