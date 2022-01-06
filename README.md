# barge

## Overview

`barge` is a very simple tool written in Rust, which manages building projects,
which contain Assembly, C, and C++ source files, with its main goal being
simplicity.

Each project translates to a single executable. Projects have and `src` and an
`include` directory at their root. which contain the source and header files
respectively. Subdirectories within these directories are supported.

Source and header files shall have appropriate file extensions based on their
type: `.c` for C source files, `.cpp` for C++ source files, `.s` for Assembly
source files, `.h` for C header files, and `.hpp` for C++ header files.

## Requirements

Internally, `barge` uses the following external software, which are required
for proper functionality.

- `coreutils (cat, wc)`: Used internally to count source code lines.
- `findutils (find)`: Used internally to collect project files.
- GNU `make`: Used internally to perform various tasks on the project.
- `nasm`: Used to compile Assembly source files.
- `clang`: Used to compile C/C++ source files, and link the executable.
- `clang-tidy`: Used for static analysis.
- `git`: Used to initialize a `git`˙repository on project creation.

On Arch Linux, the following command installs all the required packages.

`pacman -S coreutils findutils make nasm clang git`

## Subcommands

`barge` supports the following subcommands.

- `init NAME`: Creates a new project with a simple `"Hello, world!"` program
  in C++, and initializes a `git` repository in a directory with the same name.
- `build [MODE]`: Builds the project executable in the given build mode. Since
  this process uses GNU `make` internally, some messages may be displayed by
  its execution.
- `clean`: Deletes the build artifacts of the project (the built executable and
  the object files).
- `rebuild [MODE]`: Equivalent to subsequently invoking `clean` and `build`.
- `run [MODE]`: Builds and executes the project executable.
- `lines`: Displays the amount of lines of source code for the whole project.
- `analyze`: Performs static analysis for the C/C++ source files in the project.

The `build`, `rebuild`, and `run` subcommands have an optional argument, which
represents the mode of the build. The currently supported modes are `debug` and
`release`. If none is specified, `debug` is the default.

In `debug` build mode, the executable contains its debug symbols, and is
optimized for debugging, while in `release` mode, the symbols are stripped, and
the executable is optimized for fast execution.

## The project file

The user can specify the settings to their project by changing `barge.json` at
the project root. This file contains a single configuration object with the
following fields.

- name (string):
  The name of the project, and as such, the executable.
- c_standard (string):
  The C standard used for the C source files, in a format like "c99".
- cpp_standard (string):
  The C++ standard used for the C source files, in a format like "c++14".
- external_libraries (list of objects, optional):
  The list of external libraries to link with. This is a list of objects, which
  are represented in one of the following ways.
  - Using pkg-build: { type: "pkgbuild", name: "LIBRARY_NAME" }
  - Manually specifying flags: { type: "manual", "cflags": "LIBRARY_CFLAGS", ldflags: "LIBRARY_LDFLAGS"}
- custom_cflags (string, optional):
  Adds the flags specified here to the C source file compilation command line.
- custom_cxxflags (string, optional):
  Adds the flags specified here to the C++ source file compilation command line.
- custom_ldflags (string, optional):
  Adds the flags specified here to the executable linking command line.
- custom_makeopts (string, optional):
  Adds the flags specified here to the GNU make command line.

### Specific project file, which contains all the optional fields

```json
{
    "name": "example",
    "c_standard": "c99",
    "cpp_standard": "c++14",
    "external_libraries": [
        {
            "type": "pkgconfig",
            "name": "sdl"
        },
        {
            "type": "manual",
            "cflags": "",
            "ldflags": "-lpthread"
        },
    ],
    "custom_cflags": "-DNDEBUG",
    "custom_cxxflags": "-DNDEBUG",
    "custom_ldflags": "-ggdb",
    "custom_makeopts": "-j2"
}
```

### Minimal project file, which contains no optional fields

```json
{
    "name": "example",
    "c_standard": "c99",
    "cpp_standard": "c++14"
}
```
