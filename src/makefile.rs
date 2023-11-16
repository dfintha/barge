use crate::output::NO_COLOR;
use crate::project::{
    collect_source_files, Library, Project, ProjectType, Toolset, DEFAULT_CPP_STANDARD,
    DEFAULT_C_STANDARD, DEFAULT_FORTRAN_STANDARD,
};
use crate::result::{BargeError, Result};
use serde::Deserialize;
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
ASMOBJ=$(patsubst src/%.s,build/{}/obj/%.s.o,$(ASMSRC))

CC={}
CFLAGS={}
CSRC=$(shell find src -type f -name '*.c')
COBJ=$(patsubst src/%.c,build/{}/obj/%.c.o,$(CSRC))

CXX={}
CXXFLAGS={}
CXXSRC=$(shell find src -type f -name '*.cpp')
CXXOBJ=$(patsubst src/%.cpp,build/{}/obj/%.cpp.o,$(CXXSRC))

FORTRAN={}
FORTRANFLAGS={}
FORTRANSRC=$(shell find src -type f -name '*.f90')
FORTRANOBJ=$(patsubst src/%.f90,build/{}/obj/%.f90.o,$(FORTRANSRC))

LD={}
LDFLAGS={}

NAME={}
BINARY=build/{}/$(NAME)
SOURCES=$(CSRC) $(CXXSRC) $(ASMSRC) $(FORTRANSRC)
OBJECTS=$(COBJ) $(CXXOBJ) $(ASMOBJ) $(FORTRANOBJ)

{}

.PHONY: all

all: $(BINARY)

{}
{}

$(BINARY): $(OBJECTS)
\t@mkdir -p $(shell dirname $@)
\t@printf '%sLinking executable %s%s\\n' $(GREEN) $@ $(RESET)
\t{}
\t@printf '%sBuilt target %s%s\\n' $(BLUE) $(NAME) $(RESET)

build/{}/obj/%.s.o: src/%.s
\t@mkdir -p $(shell dirname $@)
\t@printf '%s%sBuilding assembly object %s.%s\\n' $(GREEN) $(DIM) $@ $(RESET)
\t@$(ASM) $(ASMFLAGS) $< -o $@

build/{}/obj/%.c.o: src/%.c
\t@mkdir -p $(shell dirname $@)
\t@printf '%s%sBuilding C object %s.%s\\n' $(GREEN) $(DIM) $@ $(RESET)
\t@$(CC) $(CFLAGS) -c $< -o $@

build/{}/obj/%.cpp.o: src/%.cpp
\t@mkdir -p $(shell dirname $@)
\t@printf '%s%sBuilding C++ object %s.%s\\n' $(GREEN) $(DIM) $@ $(RESET)
\t@$(CXX) $(CXXFLAGS) -c $< -o $@

build/{}/obj/%.f90.o: src/%.f90
\t@mkdir -p $(shell dirname $@)
\t@printf '%s%sBuilding FORTRAN object %s.%s\\n' $(GREEN) $(DIM) $@ $(RESET)
\t@$(FORTRAN) $(FORTRANFLAGS) -Jbuild/{} -c $< -o $@
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

pub(crate) fn generate_build_makefile(project: &Project, target: BuildTarget) -> Result<String> {
    let common_cflags = "-Wall -Wextra -Wpedantic -Wshadow -Wconversion \
                         -Wdouble-promotion -Wformat=2 -Iinclude -Isrc";

    let (library_cflags, library_ldflags) = build_library_flags(&project.external_libraries)?;

    let (target_cflags, target_ldflags) = match target {
        BuildTarget::Debug => ("-Og -g -fsanitize=undefined -fsanitize-trap", "-ggdb"),
        BuildTarget::Release => ("-DNDEBUG -O2 -ffast-math", "-s"),
    };

    let toolset = if let Some(toolset) = &project.toolset {
        toolset
    } else {
        &Toolset::Llvm
    };

    let (c_compiler, cpp_compiler, fortran_compiler, linker, _) = get_toolset_executables(toolset);

    let custom_cflags = if let Some(flags) = &project.custom_cflags {
        flags.clone()
    } else {
        String::new()
    };

    let custom_cxxflags = if let Some(flags) = &project.custom_cxxflags {
        flags.clone()
    } else {
        String::new()
    };

    let custom_ldflags = if let Some(flags) = &project.custom_ldflags {
        flags.clone()
    } else {
        String::new()
    };

    let custom_fortranflags = if let Some(flags) = &project.custom_fortranflags {
        flags.clone()
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

    let c_std = if let Some(c_standard) = &project.c_standard {
        c_standard
    } else {
        DEFAULT_C_STANDARD
    };

    let cpp_std = if let Some(cpp_standard) = &project.cpp_standard {
        cpp_standard
    } else {
        DEFAULT_CPP_STANDARD
    };

    let fortran_std = if let Some(fortran_standard) = &project.fortran_standard {
        fortran_standard
    } else {
        DEFAULT_FORTRAN_STANDARD
    };

    let cflags = String::from("-std=")
        + c_std
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
        + cpp_std
        + " "
        + common_cflags
        + " "
        + &library_cflags
        + " "
        + target_cflags
        + " "
        + &custom_cxxflags
        + pic_flag;

    let fortranflags = String::from("-std=") + fortran_std + " " + &custom_fortranflags;

    let libgfortran = collect_source_files(false)?
        .iter()
        .any(|source| source.ends_with(".f90"));
    let libgfortran = if libgfortran { "-lgfortran" } else { "" };

    let ldflags = target_ldflags.to_owned()
        + " "
        + &library_ldflags
        + " "
        + &custom_ldflags
        + " "
        + libgfortran;

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
        build_makefile_template!(),
        target.to_string(),
        c_compiler,
        cflags,
        target.to_string(),
        cpp_compiler,
        cxxflags,
        target.to_string(),
        fortran_compiler,
        fortranflags,
        target.to_string(),
        linker,
        ldflags,
        name,
        target.to_string(),
        colorization,
        c_dependencies,
        cpp_dependencies,
        link_command,
        target.to_string(),
        target.to_string(),
        target.to_string(),
        target.to_string(),
        target.to_string()
    );

    Ok(result)
}

pub(crate) fn generate_analyze_makefile(project: &Project) -> Result<String> {
    let c_std = if let Some(c_standard) = &project.c_standard {
        c_standard
    } else {
        DEFAULT_C_STANDARD
    };

    let cpp_std = if let Some(cpp_standard) = &project.cpp_standard {
        cpp_standard
    } else {
        DEFAULT_CPP_STANDARD
    };

    Ok(format!(analyze_makefile_template!(), c_std, cpp_std))
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
                format!("build/{}/obj/{}.o", target.to_string(), name)
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

    Ok(dependencies.join(""))
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

fn get_toolset_executables(
    toolset: &Toolset,
) -> (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
) {
    match toolset {
        Toolset::Gnu => ("gcc", "g++", "gfortran", "ld", "gdb"),
        Toolset::Llvm => ("clang", "clang++", "flang", "lld", "lldb"),
    }
}
