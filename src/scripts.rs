use crate::makefile::BuildTarget;
use crate::result::{BargeError, Result};
use std::convert::TryFrom;
use std::process::Command;

enum ScriptKind {
    ShellScript,
    PythonScript,
    CSource,
    CppSource,
}

pub(crate) struct ScriptEnvironment {
    pub target: BuildTarget,
}

impl TryFrom<&str> for ScriptKind {
    type Error = BargeError;

    fn try_from(extension: &str) -> Result<ScriptKind> {
        if extension == "sh" {
            Ok(ScriptKind::ShellScript)
        } else if extension == "py" {
            Ok(ScriptKind::PythonScript)
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
            execute_shell_script(path, env)?;
        }
        ScriptKind::PythonScript => {
            execute_python_script(path, env)?;
        }
        ScriptKind::CSource => {
            execute_c_source(path, name, env)?;
        }
        ScriptKind::CppSource => {
            execute_cpp_source(path, name, env)?;
        }
    }
    Ok(())
}

fn get_file_extension(path: &str) -> Result<&str> {
    path.split('.')
        .last()
        .ok_or(BargeError::NoneOption("Failed to parse file name"))
}

fn execute_shell_script(path: &str, env: ScriptEnvironment) -> Result<()> {
    let bash = Command::new("bash")
        .arg(path)
        .env("BARGE_BUILD_TARGET", env.target.to_string().as_str())
        .spawn()?
        .wait()?;
    let status = bash.success();
    if status {
        Ok(())
    } else {
        Err(BargeError::FailedOperation(
            "Failed to execute shell script",
        ))
    }
}

fn execute_python_script(path: &str, env: ScriptEnvironment) -> Result<()> {
    let python = Command::new("env")
        .arg("-S")
        .arg("python3")
        .arg(path)
        .env("BARGE_BUILD_TARGET", env.target.to_string().as_str())
        .spawn()?
        .wait()?;
    let status = python.success();
    if status {
        Ok(())
    } else {
        Err(BargeError::FailedOperation(
            "Failed to execute Python script",
        ))
    }
}

fn execute_c_source(path: &str, name: &str, env: ScriptEnvironment) -> Result<()> {
    let target = format!("bin/{}", name);
    let cc = Command::new("clang")
        .arg("-std=c11")
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
        .env("BARGE_BUILD_TARGET", env.target.to_string().as_str())
        .spawn()?
        .wait()?;
    if step.success() {
        Ok(())
    } else {
        Err(BargeError::FailedOperation("Custom build step failed"))
    }
}

fn execute_cpp_source(path: &str, name: &str, env: ScriptEnvironment) -> Result<()> {
    let target = format!("bin/{}", name);
    let cxx = Command::new("clang++")
        .arg("-std=c++17")
        .arg(path)
        .arg("-o")
        .arg(&target)
        .spawn()?
        .wait()?;
    if !cxx.success() {
        return Err(BargeError::FailedOperation(
            "Failed to compile a custom build step binary",
        ));
    }

    let step = Command::new(&target)
        .env("BARGE_BUILD_TARGET", env.target.to_string().as_str())
        .spawn()?
        .wait()?;
    if step.success() {
        Ok(())
    } else {
        Err(BargeError::FailedOperation("Custom build step failed"))
    }
}
