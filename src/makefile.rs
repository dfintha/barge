use crate::project::{Library, Project, ProjectType};
use crate::result::{BargeError, Result};
use std::process::Command;

#[derive(Debug, Clone, Copy)]
pub(crate) enum BuildMode {
    Debug,
    Release,
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
HEADERS=$(shell find include -name '*.h*')

GREEN=`tput setaf 2``tput bold`
BLUE=`tput setaf 4``tput bold`
RESET=`tput sgr0`
DIM=`tput dim`

.PHONY: all

all: $(BINARY)

$(BINARY): $(COBJ) $(CXXOBJ) $(ASMOBJ)
\t@mkdir -p $(shell dirname $@)
\t@printf '%sLinking executable %s%s\\n' $(GREEN) $@ $(RESET)
\t{}
\t@printf '%sBuilt target %s%s\\n' $(BLUE) $(NAME) $(RESET)

obj/{}/%.s.o: src/%.s
\t@mkdir -p $(shell dirname $@)
\t@printf '%s%sBuilding assembly object %s.%s\\n' $(GREEN) $(DIM) $@ $(RESET)
\t@$(ASM) $(ASMFLAGS) $< -o $@

obj/{}/%.c.o: src/%.c $(HEADERS)
\t@mkdir -p $(shell dirname $@)
\t@printf '%s%sBuilding C object %s.%s\\n' $(GREEN) $(DIM) $@ $(RESET)
\t@$(CC) $(CFLAGS) -c $< -o $@

obj/{}/%.cpp.o: src/%.cpp $(HEADERS)
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

pub(crate) fn generate_build_makefile(project: &Project, build_mode: BuildMode) -> Result<String> {
    let common_cflags = "-Wall -Wextra -pedantic \
                         -Wshadow -Wdouble-promotion -Wformat=2 -Wconversion \
                         -Iinclude -Isrc";

    let (library_cflags, library_ldflags) = build_library_flags(&project.external_libraries)?;

    let (mode_string, mode_cflags, mode_ldflags) = match build_mode {
        BuildMode::Debug => ("debug", "-Og", "-ggdb"),
        BuildMode::Release => ("release", "-DNDEBUG -O2 -ffast-math", "-s"),
    };

    let custom_cflags = if project.custom_cflags.is_some() {
        project
            .custom_cflags
            .clone()
            .ok_or(BargeError::NoneOption)?
    } else {
        String::new()
    };

    let custom_cxxflags = if project.custom_cxxflags.is_some() {
        project
            .custom_cflags
            .clone()
            .ok_or(BargeError::NoneOption)?
    } else {
        String::new()
    };

    let custom_ldflags = if project.custom_ldflags.is_some() {
        project
            .custom_ldflags
            .clone()
            .ok_or(BargeError::NoneOption)?
    } else {
        String::new()
    };

    let pic_flag = if project.project_type != ProjectType::Binary {
        "-fPIC"
    } else {
        ""
    };

    let cflags = String::from("-std=")
        + &project.c_standard
        + " "
        + common_cflags
        + " "
        + &library_cflags
        + " "
        + mode_cflags
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
        + mode_cflags
        + " "
        + &custom_cxxflags
        + pic_flag;

    let ldflags = library_ldflags + " " + &custom_ldflags + " " + mode_ldflags;

    let name = match project.project_type {
        ProjectType::Binary => project.name.clone(),
        ProjectType::SharedLibrary => "lib".to_string() + &project.name + ".so",
        ProjectType::StaticLibrary => "lib".to_string() + &project.name + ".a",
    };

    let link_command = match project.project_type {
        ProjectType::Binary => "@$(CXX) $(OBJECTS) -o $@ $(LDFLAGS)",
        ProjectType::SharedLibrary => "@$(CXX) -shared $(OBJECTS) -o $@ $(LDFLAGS)",
        ProjectType::StaticLibrary => "@ar rcs $@ $(OBJECTS)",
    };

    let result = format!(
        build_makefile_template!(),
        mode_string,
        cflags,
        mode_string,
        cxxflags,
        mode_string,
        ldflags,
        name,
        mode_string,
        link_command,
        mode_string,
        mode_string,
        mode_string
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
                    library_cflags.push_str(&call_pkg_config(&name, "--cflags")?);
                    library_ldflags.push_str(&call_pkg_config(&name, "--libs")?);
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
