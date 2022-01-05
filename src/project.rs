use crate::result::{BargeError, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum Library {
    PkgConfig { name: String },
    Manual { cflags: String, ldflags: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Project {
    pub name: String,
    pub c_standard: String,
    pub cpp_standard: String,
    pub external_libraries: Option<Vec<Library>>,
    pub custom_cflags: Option<String>,
    pub custom_cxxflags: Option<String>,
    pub custom_ldflags: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum BuildMode {
    Debug,
    Release,
}

macro_rules! makefile_template {
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

LDFLAGS=-no-pie {}

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
\t@$(CXX) $(OBJECTS) -o $@ $(LDFLAGS)
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

impl Project {
    pub fn load(path: &str) -> Result<Project> {
        let json = std::fs::read_to_string(path)?;
        let project: Project = serde_json::from_str(&json)?;
        Ok(project)
    }

    pub fn generate_makefile(&self, build_mode: BuildMode) -> Result<String> {
        let common_cflags = "-Wall -Wextra -pedantic \
                             -Wshadow -Wdouble-promotion -Wformat=2 -Wconversion \
                             -Iinclude -Isrc";

        let (library_cflags, library_ldflags) = build_library_flags(&self.external_libraries)?;

        let (mode_string, mode_cflags, mode_ldflags) = match build_mode {
            BuildMode::Debug => ("debug", "-Og", "-ggdb"),
            BuildMode::Release => ("release", "-DNDEBUG -O2 -ffast-math", "-s"),
        };

        let custom_cflags = if self.custom_cflags.is_some() {
            self.custom_cflags.clone().ok_or(BargeError::NoneOption)?
        } else {
            String::new()
        };

        let custom_cxxflags = if self.custom_cxxflags.is_some() {
            self.custom_cflags.clone().ok_or(BargeError::NoneOption)?
        } else {
            String::new()
        };

        let custom_ldflags = if self.custom_ldflags.is_some() {
            self.custom_ldflags.clone().ok_or(BargeError::NoneOption)?
        } else {
            String::new()
        };

        let cflags = String::from("-std=")
            + &self.c_standard
            + " "
            + common_cflags
            + " "
            + &library_cflags
            + " "
            + mode_cflags
            + " "
            + &custom_cflags;

        let cxxflags = String::from("-std=")
            + &self.cpp_standard
            + " "
            + common_cflags
            + " "
            + &library_cflags
            + " "
            + mode_cflags
            + " "
            + &custom_cxxflags;

        let ldflags = library_ldflags + " " + &custom_ldflags + " " + mode_ldflags;

        let result = format!(
            makefile_template!(),
            mode_string,
            cflags,
            mode_string,
            cxxflags,
            mode_string,
            ldflags,
            self.name,
            mode_string,
            mode_string,
            mode_string,
            mode_string
        );

        Ok(result)
    }
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

            library_cflags.push_str(" ");
            library_ldflags.push_str(" ");
        }
    }

    Ok((library_cflags, library_ldflags))
}
