use serde::{Deserialize, Serialize};
use std::io::Result;
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
    pub custom_ldflags: Option<String>,
}

macro_rules! makefile_template {
    () => {
        "
CC=clang
CFLAGS=-std={} {} {} {}
CSRC=$(shell find src -type f -name '*.c')
COBJ=$(patsubst src/%.c,obj/%.c.o,$(CSRC))

CXX=clang++
CXXFLAGS=-std={} {} {} {}
CXXSRC=$(shell find src -type f -name '*.cpp')
CXXOBJ=$(patsubst src/%.cpp,obj/%.cpp.o,$(CXXSRC))

LDFLAGS={} {}

BINARY=bin/{}
SOURCES=$(CSRC) $(CXXSRC)
OBJECTS=$(COBJ) $(CXXOBJ)
HEADERS=$(shell find include -name '*.h*')

.PHONY: all

all: $(BINARY)
\t@printf 'Build successful!\\n'

$(BINARY): $(COBJ) $(CXXOBJ)
\t@mkdir -p $(shell dirname $@)
\t@printf 'Linking project executable.\\n'
\t@$(CXX) $(OBJECTS) -o $@ $(LDFLAGS)

obj/%.c.o: src/%.c $(HEADERS)
\t@mkdir -p $(shell dirname $@)
\t@printf 'Building C object %s.\\n' $@
\t@$(CC) $(CFLAGS) -c $< -o $@

obj/%.cpp.o: src/%.cpp $(HEADERS)
\t@mkdir -p $(shell dirname $@)
\t@printf 'Building C++ object %s.\\n' $@
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

    pub fn generate_makefile(&self) -> String {
        let common_cflags = "-Wall -Wextra -pedantic -Wshadow -Wdouble-promotion -Wformat=2 -Wconversion -Iinclude -Isrc";
        let (library_cflags, library_ldflags) = build_library_flags(&self.external_libraries);

        let custom_cflags = if self.custom_cflags.is_some() {
            self.custom_cflags.clone().unwrap()
        } else {
            String::new()
        };

        let custom_ldflags = if self.custom_ldflags.is_some() {
            self.custom_ldflags.clone().unwrap()
        } else {
            String::new()
        };

        let result = format!(
            makefile_template!(),
            self.c_standard,
            common_cflags,
            library_cflags,
            custom_cflags,
            self.cpp_standard,
            common_cflags,
            library_cflags,
            custom_cflags,
            library_ldflags,
            custom_ldflags,
            self.name
        );

        result
    }
}

fn call_pkg_config(name: &str, mode: &str) -> String {
    let result = Command::new("pkg-config")
        .arg(name)
        .arg(mode)
        .output()
        .unwrap()
        .stdout;
    let mut result = std::str::from_utf8(&result).unwrap().to_string();
    result.pop();
    result
}

fn build_library_flags(libraries: &Option<Vec<Library>>) -> (String, String) {
    let mut library_cflags = String::new();
    let mut library_ldflags = String::new();

    if let Some(libraries) = libraries {
        libraries.iter().for_each(|library| {
            match library {
                Library::PkgConfig { name } => {
                    library_cflags.push_str(&call_pkg_config(&name, "--cflags"));
                    library_ldflags.push_str(&call_pkg_config(&name, "--libs"));
                }
                Library::Manual { cflags, ldflags } => {
                    library_cflags.push_str(cflags);
                    library_ldflags.push_str(ldflags);
                }
            }

            library_cflags.push_str(" ");
            library_ldflags.push_str(" ");
        });
    }

    (library_cflags, library_ldflags)
}
