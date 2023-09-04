use crate::makefile::BuildTarget;
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
            execute_clang_source(path, name, "clang", "-std=c11", env)?;
        }
        ScriptKind::CppSource => {
            execute_clang_source(path, name, "clang++", "-std=c++17", env)?;
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

fn execute_clang_source(
    path: &str,
    name: &str,
    compiler: &str,
    std_flag: &str,
    env: ScriptEnvironment,
) -> Result<()> {
    let target = format!("bin/{}", name);
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
