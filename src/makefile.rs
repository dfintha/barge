use crate::output::NO_COLOR;
use crate::project::{
    collect_source_files, CollectSourceFilesMode, Library, Project, ProjectType,
    DEFAULT_COBOL_STANDARD, DEFAULT_CPP_STANDARD, DEFAULT_CUSTOM_CFLAGS, DEFAULT_CUSTOM_COBOLFLAGS,
    DEFAULT_CUSTOM_CXXFLAGS, DEFAULT_CUSTOM_FORTRANFLAGS, DEFAULT_CUSTOM_LDFLAGS,
    DEFAULT_C_STANDARD, DEFAULT_FORTRAN_STANDARD,
};
use crate::result::{BargeError, Result};
use serde::Deserialize;
use std::fmt::Display;
use std::process::Command;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
pub(crate) enum BuildTarget {
    Debug,
    Release,
}

impl Display for BuildTarget {
    fn fmt(
        &self,
        formatter: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        match self {
            BuildTarget::Debug => write!(formatter, "debug"),
            BuildTarget::Release => write!(formatter, "release"),
        }
    }
}

impl TryFrom<&str> for BuildTarget {
    type Error = BargeError;

    fn try_from(string: &str) -> Result<BuildTarget> {
        if string == "debug" {
            Ok(BuildTarget::Debug)
        } else if string == "release" {
            Ok(BuildTarget::Release)
        } else {
            Err(BargeError::InvalidValue("Invalid target specified"))
        }
    }
}

macro_rules! get_field_or_default {
    ($field:expr, $default:ident) => {
        if let Some(field) = &$field {
            field
        } else {
            $default
        }
    };
}

fn get_cobol_ldflags() -> Result<String> {
    let result = Command::new("cob-config").arg("--libs").output()?.stdout;
    Ok(String::from_utf8(result)?)
}

pub(crate) fn generate_build_makefile(project: &Project, target: BuildTarget) -> Result<String> {
    let common_cflags = "-Wall -Wextra -Wpedantic -Wshadow -Wconversion \
                         -Wdouble-promotion -Wformat=2 -Iinclude -Isrc";

    let (library_cflags, library_ldflags) = build_library_flags(&project.external_libraries)?;

    let (target_cflags, target_ldflags) = match target {
        BuildTarget::Debug => ("-Og -g -fsanitize=undefined -fsanitize-trap", "-ggdb"),
        BuildTarget::Release => ("-DNDEBUG -O2 -ffast-math", "-s"),
    };

    let c_std = get_field_or_default!(project.c_standard, DEFAULT_C_STANDARD);
    let cpp_std = get_field_or_default!(project.cpp_standard, DEFAULT_CPP_STANDARD);
    let fortran_std = get_field_or_default!(project.fortran_standard, DEFAULT_FORTRAN_STANDARD);
    let cobol_std = get_field_or_default!(project.cobol_standard, DEFAULT_COBOL_STANDARD);
    let custom_cflags = get_field_or_default!(project.custom_cflags, DEFAULT_CUSTOM_CFLAGS);
    let custom_cxxflags = get_field_or_default!(project.custom_cxxflags, DEFAULT_CUSTOM_CXXFLAGS);
    let custom_ldflags = get_field_or_default!(project.custom_ldflags, DEFAULT_CUSTOM_LDFLAGS);
    let custom_fortranflags =
        get_field_or_default!(project.custom_fortranflags, DEFAULT_CUSTOM_FORTRANFLAGS);
    let custom_cobolflags =
        get_field_or_default!(project.custom_cobolflags, DEFAULT_CUSTOM_COBOLFLAGS);

    let pic_flag = if project.project_type != ProjectType::Executable {
        "-fPIC"
    } else {
        ""
    };

    let c_dependencies = get_dependencies_for_project(target, "c")?;
    let cpp_dependencies = get_dependencies_for_project(target, "cpp")?;

    let cflags = String::from("-std=")
        + c_std
        + " "
        + common_cflags
        + " "
        + &library_cflags
        + " "
        + target_cflags
        + " "
        + custom_cflags
        + pic_flag;

    let cxxflags = String::from("-std=")
        + cpp_std
        + " "
        + common_cflags
        + " "
        + &library_cflags
        + " "
        + target_cflags
        + " "
        + custom_cxxflags
        + pic_flag;

    let fortranflags = String::from("-std=") + fortran_std + " " + custom_fortranflags;

    let has_fortran_sources = collect_source_files(CollectSourceFilesMode::All)?
        .iter()
        .any(|source| source.ends_with(".f90"));
    let fortran_ldflags = if has_fortran_sources {
        "-lgfortran"
    } else {
        ""
    };

    let cobolflags = String::from("-std=") + cobol_std + " " + custom_cobolflags;

    let has_cobol_sources = collect_source_files(CollectSourceFilesMode::All)?
        .iter()
        .any(|source| source.ends_with(".cob"));
    let cobol_ldflags = if has_cobol_sources {
        get_cobol_ldflags()?
    } else {
        String::new()
    };

    let ldscriptflags = collect_source_files(CollectSourceFilesMode::LinkerScriptsOnly)?
        .iter()
        .map(|f| format!("-T {}", f))
        .collect::<Vec<_>>()
        .join(" ");

    let ldflags = format!(
        "{} {} {} {} {} {}",
        target_ldflags,
        library_ldflags,
        custom_ldflags,
        fortran_ldflags,
        cobol_ldflags.trim(),
        ldscriptflags
    );

    let name = match project.project_type {
        ProjectType::Executable => project.name.clone(),
        ProjectType::SharedLibrary => "lib".to_string() + &project.name + ".so",
        ProjectType::StaticLibrary => "lib".to_string() + &project.name + ".a",
    };

    let link_command = match project.project_type {
        ProjectType::Executable => "@$(LD) $(OBJECTS) -o $@ $(LDFLAGS)",
        ProjectType::SharedLibrary => "@$(LD) -shared $(OBJECTS) -o $@ $(LDFLAGS)",
        ProjectType::StaticLibrary => "@ar rcs $@ $(OBJECTS)",
    };

    let colorization = if *NO_COLOR {
        "
GREEN=''
BLUE=''
RESET=''
DIM=''
        "
    } else {
        "
GREEN=`tput setaf 2``tput bold`
BLUE=`tput setaf 4``tput bold`
RESET=`tput sgr0`
DIM=`tput dim`
        "
    };

    let result = format!(
        include_str!("template-makefile-build.in"),
        target.to_string(),
        project.get_assembler()?,
        project.get_c_compiler()?,
        cflags,
        project.get_cpp_compiler()?,
        cxxflags,
        project.get_fortran_compiler()?,
        fortranflags,
        cobolflags,
        project.get_linker()?,
        ldflags,
        name,
        colorization,
        c_dependencies,
        cpp_dependencies,
        link_command
    );

    Ok(result)
}

pub(crate) fn generate_analyze_makefile(project: &Project) -> Result<String> {
    let c_std = get_field_or_default!(project.c_standard, DEFAULT_C_STANDARD);
    let cpp_std = get_field_or_default!(project.cpp_standard, DEFAULT_CPP_STANDARD);
    Ok(format!(
        include_str!("template-makefile-analyze.in"),
        c_std, cpp_std
    ))
}

fn get_dependencies_for_project(target: BuildTarget, extension: &str) -> Result<String> {
    let sources = Command::new("find")
        .arg("src")
        .args(vec!["-type", "f"])
        .args(vec!["-name", format!("*.{}", extension).as_str()])
        .output()?
        .stdout;
    let mut sources: Vec<&str> = std::str::from_utf8(&sources)?.split('\n').collect();
    sources.retain(|source| !source.is_empty());

    let dependencies: Vec<_> = sources
        .iter()
        .map(|file| {
            let object = if let Some(name) = file.strip_prefix("src/") {
                format!("build/{}/obj/{}.o", target, name)
            } else {
                String::from("")
            };

            Command::new("clang++")
                .arg("-MM")
                .arg("-MT")
                .arg(&object)
                .arg("-Iinclude")
                .arg("-Isrc")
                .arg(file)
                .output()
        })
        .filter_map(|result| result.ok())
        .map(|result| String::from_utf8(result.stdout))
        .filter_map(|result| result.ok())
        .collect::<Vec<_>>();

    Ok(dependencies.join("").trim_end().to_string())
}

fn call_pkg_config(name: &str, mode: &str) -> Result<String> {
    let result = Command::new("pkg-config")
        .arg(name)
        .arg(mode)
        .output()?
        .stdout;
    let mut result = std::str::from_utf8(&result)?.to_string();
    result.pop();
    Ok(result)
}

fn build_library_flags(libraries: &Option<Vec<Library>>) -> Result<(String, String)> {
    let mut library_cflags = String::new();
    let mut library_ldflags = String::new();

    if let Some(libraries) = libraries {
        for library in libraries {
            match library {
                Library::PkgConfig { name } => {
                    library_cflags.push_str(&call_pkg_config(name, "--cflags")?);
                    library_ldflags.push_str(&call_pkg_config(name, "--libs")?);
                }
                Library::Manual { cflags, ldflags } => {
                    library_cflags.push_str(cflags);
                    library_ldflags.push_str(ldflags);
                }
            }

            library_cflags.push(' ');
            library_ldflags.push(' ');
        }
    }

    Ok((library_cflags, library_ldflags))
}
