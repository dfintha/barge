# barge

## Overview

`barge` is a very simple tool written in Rust, which manages building projects,
which contain Assembly, C, and C++ source files, with its main goal being
simplicity.

Each project translates to a single library (static or shared) or executable.
Projects have and `src` and an `include` directory at their root. which contain
the source and header files respectively. Subdirectories within these
directories are supported.

Source and header files shall have appropriate file extensions based on their
type: `.c` for C source files, `.cpp` for C++ source files, `.s` for Assembly
source files, `.f90` for FORTRAN source files, .h` for C header files, and
`.hpp` for C++ header files.

`barge` supports the [`NO_COLOR`](https://no-color.org/) environment variable:
if it is set, no output will be colorized using ANSI terminal escape codes.

**Please note that the development of this software is in a very early stage.
As such, changes to the project file format and/or usage can happen
frequently.**

## Requirements

Internally, `barge` uses the following external software, which are required
for proper functionality.

- `coreutils (cat, wc)`: Used internally to count source code lines.
- `findutils (find)`: Used internally to collect project files.
- `make`: Used internally to perform various tasks on the project.
- `nasm`: Used to compile Assembly source files (if present).
- `clang (clang, clang++, clang-tidy, clang-format)`: Used to compile C/C++
  source files, to link the executable, to run static analysis on the code,
  to automatically format the code, and to compile the dependency tree of C/C++
  object files.
- `git`: Used to initialize a `git`˙repository on project creation.
- `doxygen`: Used to generate HTML documentation for projects.
- `gfortran`: Used to compile FORTRAN source files (if present).
- `bash`: Used for pre- and post-build shell scripts (if present).
- `python`: Used for pre- and post-build Python 3 scripts (if present).
- `perl`: Used for pre- and post-build Perl scripts (if present).

On Arch Linux, the following command installs the packages of all the required
dependencies.

`pacman -S coreutils findutils make nasm clang git doxygen`

## Subcommands

`barge` supports the following subcommands.

- `init <NAME>`: Creates a new project with a simple `"Hello, world!"` program
  in C++, and initializes a `git` repository in a directory with the same name.
- `build [TARGET]`, `b`: Builds the project executable for the given build
  target.
  Since this process uses GNU `make` internally, some messages may be displayed
  by its execution.
- `clean`: Deletes the build artifacts of the project (the built executable and
  the object files).
- `rebuild [TARGET]`: Equivalent to subsequently invoking `clean` and `build`.
- `run [TARGET]`, `r`: Builds and executes the project executable.
- `lines`: Displays the amount of lines of source code for the whole project.
- `analyze`: Performs static analysis for the C/C++ source files in the project.
- `format`, `fmt` : Formats the source files in-place using `clang-format`.
- `doc` : Generates HTML documentation for the project using `doxygen`. This
  requires a `Doxyfile` to be present at the project root.

The `build`, `rebuild`, and `run` subcommands have an optional argument, which
represents the configuration (target) of the build. The currently supported
targets are `debug` and `release`. If none is specified, `debug` is selected by
default.

In `debug` configuration, the resulting file contains its debug symbols, and is
optimized for debugging, while in `release` configuration, the symbols are
stripped, and the file is optimized for fast execution.

## The project file

The user can specify the settings to their project by changing `barge.json` at
the project root. This file contains a single configuration object with the
following fields.

- **`name` (string)**:
  The name of the project.
- **`authors` (list of strings)**:
  The authors of the project in `git` committer format.
- **`description` (string)**:
  A short descriptions about the project.
- **`project_type` (string)**:
  The type of the project. Can be either `executable`, `shared_library`, or
  `static_library`.
- **`version` (string)**:
  The version of the project.
- **`c_standard` (string)**:
  The C standard used for the C source files, in a format like "c99".
- **`cpp_standard` (string)**:
  The C++ standard used for the C source files, in a format like "c++14".
- **`fortran_standard` (string)**:
  The FORTRAN standard used for the FORTRAN source files, in a format like
  "f95".
- **`external_libraries` (list of objects, optional)**:
  The list of external libraries to link with. This is a list of objects, which
  are represented in one of the following ways.
  - Using pkg-config: `{ type: "pkg_config", name: "LIBRARY_NAME" }``
  - Manually specifying flags: `{ type: "manual", "cflags": "LIBRARY_CFLAGS",
    ldflags: "LIBRARY_LDFLAGS"}`
- **`custom_cflags` (string, optional)**:
  Adds the flags specified here to the C source file compilation command line.
- **`custom_cxxflags` (string, optional)**:
  Adds the flags specified here to the C++ source file compilation command line.
- **`custom_fortranflags` (string, optional)**:
  Adds the flags specified here to the FORTRAN source file compilation command
  line.
- **`custom_ldflags` (string, optional)**:
  Adds the flags specified here to the executable linking command line.
- **`custom_makeopts` (string, optional)**:
  Adds the flags specified here to the GNU make command line. If none given,
  the default makeopts will only specify the amount of parallel jobs. This is
  the minimum of the logical cores and the amount of free memory divided by 2
  GiB.
- **`format_style` (string, optional)**:
  The style in which clang-format formats the project sources. If none given,
  the default is Google. The supported format styles are the ones supported by
  `clang-format`. If `file` is given, `clang-format` will look for a
  `.clang-format` file in parent directories relative to the given source file.
- **`pre_build_step` (string, optional)**:
  A script or C/C++ source file to execute before starting a build.
- **`post_build_step` (string, optional)**:
  A script or C/C++ source file to execute after a successful build.


### Specific project file, which contains all the optional fields

```json
{
    "name": "example",
    "authors": ["Somebody <somebody@example.org>"],
    "description": "An awesome example project.",
    "project_type": "executable",
    "version": "0.1.0",
    "c_standard": "c99",
    "cpp_standard": "c++14",
    "fortran_standard": "f95",
    "external_libraries": [
        {
            "type": "pkg_config",
            "name": "sdl"
        },
        {
            "type": "manual",
            "cflags": "",
            "ldflags": "-lpthread"
        }
    ],
    "custom_cflags": "-DNDEBUG",
    "custom_cxxflags": "-DNDEBUG",
    "custom_fortranflags": "",
    "custom_ldflags": "-ggdb",
    "custom_makeopts": "-j2",
    "format_style": "Google",
    "pre_build_step": "prebuild.py",
    "post_build_step": "postbuild.cpp"
}
```

### Minimal project file, which contains no optional fields

```json
{
    "name": "example",
    "authors": ["Somebody <somebody@example.org>"],
    "description": "An awesome example project.",
    "project_type": "executable",
    "version": "0.1.0",
    "c_standard": "c99",
    "cpp_standard": "c++14",
    "fortran_standard": "f95"
}
```

## Pre-build and post-build scripts

Executables for `pre_build_step` and `post_build_step` support the following
file types, and the interpreter or compiler is chosen based on the file
extension.

- `bash` script (`.sh`)
- Python 3 script (`.py`)
- Perl script (`.pl`)
- C source file (`.c`)
- C++ source file (`.cpp`)

Obviously, the `bash`, `python3`, and `perl` interpreters must be present for
their respective scripts to work. C/C++ build steps are compiled using the
C11/C++17 standards.

During their execution, these scripts/binaries have the following environment
variables set.

- `BARGE_BUILD_TARGET`: The build target, either `debug` or `release`.
- `BARGE_PROJECT_NAME`: Name of the project.
- `BARGE_PROJECT_DESCRIPTION`: Short description of the project.
- `BARGE_PROJECT_AUTHORS`: Comma-separated list of the authors of the project.
- `BARGE_PROJECT_VERSION`: The version of the project.
- `BARGE_OBJECTS_DIR`: The directory where object files reside.
- `BARGE_BINARY_DIR`: The directory where the compiled binary resides.
- `NO_COLOR`: If set when `barge` was executed, is also set in the scripts.
