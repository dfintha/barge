use crate::project::{Library, Project, ProjectType};
use crate::result::{BargeError, Result};
use serde::Deserialize;
use std::convert::TryFrom;
use std::process::Command;
use std::string::ToString;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
pub(crate) enum BuildTarget {
    Debug,
    Release,
}

impl ToString for BuildTarget {
    fn to_string(&self) -> String {
        match self {
            BuildTarget::Debug => String::from("debug"),
            BuildTarget::Release => String::from("release"),
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

macro_rules! build_makefile_template {
    () => {
        "
ASM=nasm
ASMFLAGS=-f elf64
ASMSRC=$(shell find src -type f -name '*.s')
ASMOBJ=$(patsubst src/%.s,obj/{}/%.s.o,$(ASMSRC))

CC=clang
CFLAGS={}
CSRC=$(shell find src -type f -name '*.c')
COBJ=$(patsubst src/%.c,obj/{}/%.c.o,$(CSRC))

CXX=clang++
CXXFLAGS={}
CXXSRC=$(shell find src -type f -name '*.cpp')
CXXOBJ=$(patsubst src/%.cpp,obj/{}/%.cpp.o,$(CXXSRC))

LDFLAGS={}

NAME={}
BINARY=bin/{}/$(NAME)
SOURCES=$(CSRC) $(CXXSRC) $(ASMSRC)
OBJECTS=$(COBJ) $(CXXOBJ) $(ASMOBJ)

{}

.PHONY: all

all: $(BINARY)

{}
{}

$(BINARY): $(COBJ) $(CXXOBJ) $(ASMOBJ)
\t@mkdir -p $(shell dirname $@)
\t@printf '%sLinking executable %s%s\\n' $(GREEN) $@ $(RESET)
\t{}
\t@printf '%sBuilt target %s%s\\n' $(BLUE) $(NAME) $(RESET)

obj/{}/%.s.o: src/%.s
\t@mkdir -p $(shell dirname $@)
\t@printf '%s%sBuilding assembly object %s.%s\\n' $(GREEN) $(DIM) $@ $(RESET)
\t@$(ASM) $(ASMFLAGS) $< -o $@

obj/{}/%.c.o: src/%.c
\t@mkdir -p $(shell dirname $@)
\t@printf '%s%sBuilding C object %s.%s\\n' $(GREEN) $(DIM) $@ $(RESET)
\t@$(CC) $(CFLAGS) -c $< -o $@

obj/{}/%.cpp.o: src/%.cpp
\t@mkdir -p $(shell dirname $@)
\t@printf '%s%sBuilding C++ object %s.%s\\n' $(GREEN) $(DIM) $@ $(RESET)
\t@$(CXX) $(CXXFLAGS) -c $< -o $@
"
    };
}

macro_rules! analyze_makefile_template {
    () => {
        "
CSRC=$(shell find src -type f -name '*.c')
CXXSRC=$(shell find src -type f -name '*.cpp')
PFLAGS=-Iinclude -Isrc
WFLAGS=-Wall -Wextra -pedantic -Wshadow -Wdouble-promotion -Wformat=2 -Wconversion
FLAGS=$(PFLAGS) $(WFLAGS)
.PHONY: analyze
analyze: $(CSRC) $(CXXSRC)
\t@[ \"$(CSRC)\" != \"\" ] && clang-tidy $(CSRC) -- -std={} $(FLAGS) || true
\t@[ \"$(CXXSRC)\" != \"\" ] && clang-tidy $(CXXSRC) -- -std={} $(FLAGS) || true
"
    };
}

fn get_dependencies_for_project(target: BuildTarget, extension: &str) -> Result<String> {
    let sources = Command::new("find")
        .arg("src")
        .args(vec!["-type", "f"])
        .args(vec!["-name", format!("*.{}", extension).as_str()])
        .output()?
        .stdout;
    let sources: Vec<_> = std::str::from_utf8(&sources)?.split('\n').collect();
    let dependencies = Command::new("clang++")
        .arg("-MM")
        .arg("-Iinclude")
        .args(sources)
        .output()?
        .stdout;
    let dependencies: Vec<_> = std::str::from_utf8(&dependencies)?
        .split('\n')
        .collect::<Vec<_>>()
        .iter()
        .map(|dependency| {
            if dependency.starts_with(' ') || dependency.is_empty() {
                dependency.to_string()
            } else {
                let obj_extension = format!(".{}.o:", extension);
                format!("obj/{}/{}", target.to_string(), dependency)
                    .replace(".o:", obj_extension.as_str())
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        .into();
    Ok(std::str::from_utf8(&dependencies)?.to_string())
}

pub(crate) fn generate_build_makefile(project: &Project, target: BuildTarget) -> Result<String> {
    let common_cflags = "-Wall -Wextra -Wpedantic -Wshadow -Wconversion \
                         -Wdouble-promotion -Wformat=2 -Iinclude -Isrc";

    let (library_cflags, library_ldflags) = build_library_flags(&project.external_libraries)?;

    let (target_cflags, target_ldflags) = match target {
        BuildTarget::Debug => ("-Og -g -fsanitize=undefined -fsanitize-trap", "-ggdb"),
        BuildTarget::Release => ("-DNDEBUG -O2 -ffast-math", "-s"),
    };

    let custom_cflags = if project.custom_cflags.is_some() {
        project.custom_cflags.clone().ok_or(BargeError::NoneOption(
            "Nonexistent optional value reported as existent",
        ))?
    } else {
        String::new()
    };

    let custom_cxxflags = if project.custom_cxxflags.is_some() {
        project.custom_cflags.clone().ok_or(BargeError::NoneOption(
            "Nonexistent optional value reported as existent",
        ))?
    } else {
        String::new()
    };

    let custom_ldflags = if project.custom_ldflags.is_some() {
        project
            .custom_ldflags
            .clone()
            .ok_or(BargeError::NoneOption(
                "Nonexistent optional value reported as existent",
            ))?
    } else {
        String::new()
    };

    let pic_flag = if project.project_type != ProjectType::Executable {
        "-fPIC"
    } else {
        ""
    };

    let c_dependencies = get_dependencies_for_project(target, "c")?;
    let cpp_dependencies = get_dependencies_for_project(target, "cpp")?;

    let cflags = String::from("-std=")
        + &project.c_standard
        + " "
        + common_cflags
        + " "
        + &library_cflags
        + " "
        + target_cflags
        + " "
        + &custom_cflags
        + pic_flag;

    let cxxflags = String::from("-std=")
        + &project.cpp_standard
        + " "
        + common_cflags
        + " "
        + &library_cflags
        + " "
        + target_cflags
        + " "
        + &custom_cxxflags
        + pic_flag;

    let ldflags = target_ldflags.to_owned() + " " + &library_ldflags + " " + &custom_ldflags;

    let name = match project.project_type {
        ProjectType::Executable => project.name.clone(),
        ProjectType::SharedLibrary => "lib".to_string() + &project.name + ".so",
        ProjectType::StaticLibrary => "lib".to_string() + &project.name + ".a",
    };

    let link_command = match project.project_type {
        ProjectType::Executable => "@$(CXX) $(OBJECTS) -o $@ $(LDFLAGS)",
        ProjectType::SharedLibrary => "@$(CXX) -shared $(OBJECTS) -o $@ $(LDFLAGS)",
        ProjectType::StaticLibrary => "@ar rcs $@ $(OBJECTS)",
    };

    let colorization = if std::env::var("NO_COLOR").is_ok() {
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
        build_makefile_template!(),
        target.to_string(),
        cflags,
        target.to_string(),
        cxxflags,
        target.to_string(),
        ldflags,
        name,
        target.to_string(),
        colorization,
        c_dependencies,
        cpp_dependencies,
        link_command,
        target.to_string(),
        target.to_string(),
        target.to_string()
    );

    Ok(result)
}

pub(crate) fn generate_analyze_makefile(project: &Project) -> Result<String> {
    Ok(format!(
        analyze_makefile_template!(),
        project.c_standard, project.cpp_standard
    ))
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
