use crate::makefile::BuildTarget;
use crate::result::{BargeError, Result};
use crate::NO_COLOR;
use std::collections::HashMap;
use std::process::Command;

enum ScriptKind {
    ShellScript,
    PythonScript,
    CSource,
    CppSource,
}

pub(crate) struct ScriptEnvironment<'a> {
    pub target: BuildTarget,
    pub name: &'a String,
    pub version: &'a String,
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
    let interpreter = Command::new("bash")
        .arg(path)
        .envs(unpack_script_environment(env))
        .spawn()?
        .wait()?;
    if interpreter.success() {
        Ok(())
    } else {
        Err(BargeError::FailedOperation(
            "Failed to execute shell (bash) script",
        ))
    }
}

fn execute_python_script(path: &str, env: ScriptEnvironment) -> Result<()> {
    let interpreter = Command::new("env")
        .arg("-S")
        .arg("python3")
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
        .envs(unpack_script_environment(env))
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
    result.insert(
        String::from("BARGE_PROJECT_VERSION"),
        env.version.to_string(),
    );
    result.insert(String::from("BARGE_BUILD_TARGET"), env.target.to_string());
    result.insert(
        String::from("BARGE_OBJECTS_DIR"),
        format!("obj/{}", env.target.to_string()),
    );
    result.insert(
        String::from("BARGE_BINARY_DIR"),
        format!("bin/{}", env.target.to_string()),
    );
    if *NO_COLOR {
        result.insert(String::from("NO_COLOR"), String::from("1"));
    }
    result
}
