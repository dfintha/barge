use crate::makefile::BuildTarget;
use crate::output::*;
use crate::project::{collect_source_files, Project, ProjectType};
use crate::result::{BargeError, Result};
use crate::utilities::{attempt_remove_directory, look_for_project_directory};
use clap::App;
use std::fs::File;
use std::io::Write;
use std::process::{Command, Stdio};

mod makefile;
mod output;
mod project;
mod result;
mod scripts;
mod utilities;

macro_rules! hello_template {
    () => {
        "#include <iostream>

int main() {
    std::cout << \"Hello, world!\" << std::endl;
    return 0;
}
"
    };
}

macro_rules! doxy_template {
    () => {
        "
#---------------------------------------------------------------------------
# Project related configuration options
#---------------------------------------------------------------------------
DOXYFILE_ENCODING      = UTF-8
PROJECT_NAME           = $(BARGE_PROJECT_NAME)
PROJECT_NUMBER         = $(BARGE_PROJECT_VERSION)
PROJECT_BRIEF          = \"\"
PROJECT_LOGO           =
OUTPUT_DIRECTORY       = build/doc
CREATE_SUBDIRS         = NO
ALLOW_UNICODE_NAMES    = NO
OUTPUT_LANGUAGE        = English
BRIEF_MEMBER_DESC      = YES
REPEAT_BRIEF           = YES
ABBREVIATE_BRIEF       = \"The $name class\" \
                         \"The $name widget\" \
                         \"The $name file\" \
                         is \
                         provides \
                         specifies \
                         contains \
                         represents \
                         a \
                         an \
                         the
ALWAYS_DETAILED_SEC    = NO
INLINE_INHERITED_MEMB  = NO
FULL_PATH_NAMES        = YES
STRIP_FROM_PATH        =
STRIP_FROM_INC_PATH    =
SHORT_NAMES            = NO
JAVADOC_AUTOBRIEF      = NO
JAVADOC_BANNER         = NO
QT_AUTOBRIEF           = NO
MULTILINE_CPP_IS_BRIEF = NO
PYTHON_DOCSTRING       = YES
INHERIT_DOCS           = YES
SEPARATE_MEMBER_PAGES  = NO
TAB_SIZE               = 4
ALIASES                =
OPTIMIZE_OUTPUT_FOR_C  = YES
OPTIMIZE_OUTPUT_JAVA   = NO
OPTIMIZE_FOR_FORTRAN   = NO
OPTIMIZE_OUTPUT_VHDL   = NO
OPTIMIZE_OUTPUT_SLICE  = NO
EXTENSION_MAPPING      =
MARKDOWN_SUPPORT       = YES
TOC_INCLUDE_HEADINGS   = 5
AUTOLINK_SUPPORT       = YES
BUILTIN_STL_SUPPORT    = NO
CPP_CLI_SUPPORT        = NO
SIP_SUPPORT            = NO
IDL_PROPERTY_SUPPORT   = YES
DISTRIBUTE_GROUP_DOC   = NO
GROUP_NESTED_COMPOUNDS = NO
SUBGROUPING            = YES
INLINE_GROUPED_CLASSES = NO
INLINE_SIMPLE_STRUCTS  = NO
TYPEDEF_HIDES_STRUCT   = NO
LOOKUP_CACHE_SIZE      = 0
NUM_PROC_THREADS       = 1
#---------------------------------------------------------------------------
# Build related configuration options
#---------------------------------------------------------------------------
EXTRACT_ALL            = YES
EXTRACT_PRIVATE        = NO
EXTRACT_PRIV_VIRTUAL   = NO
EXTRACT_PACKAGE        = NO
EXTRACT_STATIC         = NO
EXTRACT_LOCAL_CLASSES  = YES
EXTRACT_LOCAL_METHODS  = NO
EXTRACT_ANON_NSPACES   = NO
RESOLVE_UNNAMED_PARAMS = YES
HIDE_UNDOC_MEMBERS     = NO
HIDE_UNDOC_CLASSES     = NO
HIDE_FRIEND_COMPOUNDS  = NO
HIDE_IN_BODY_DOCS      = NO
INTERNAL_DOCS          = NO
CASE_SENSE_NAMES       = YES
HIDE_SCOPE_NAMES       = YES
HIDE_COMPOUND_REFERENCE= NO
SHOW_INCLUDE_FILES     = YES
SHOW_GROUPED_MEMB_INC  = NO
FORCE_LOCAL_INCLUDES   = NO
INLINE_INFO            = YES
SORT_MEMBER_DOCS       = YES
SORT_BRIEF_DOCS        = NO
SORT_MEMBERS_CTORS_1ST = NO
SORT_GROUP_NAMES       = NO
SORT_BY_SCOPE_NAME     = NO
STRICT_PROTO_MATCHING  = NO
GENERATE_TODOLIST      = YES
GENERATE_TESTLIST      = YES
GENERATE_BUGLIST       = YES
GENERATE_DEPRECATEDLIST= YES
ENABLED_SECTIONS       =
MAX_INITIALIZER_LINES  = 30
SHOW_USED_FILES        = YES
SHOW_FILES             = YES
SHOW_NAMESPACES        = YES
FILE_VERSION_FILTER    =
LAYOUT_FILE            =
CITE_BIB_FILES         =
#---------------------------------------------------------------------------
# Configuration options related to warning and progress messages
#---------------------------------------------------------------------------
QUIET                  = YES
WARNINGS               = YES
WARN_IF_UNDOCUMENTED   = YES
WARN_IF_DOC_ERROR      = YES
WARN_NO_PARAMDOC       = NO
WARN_AS_ERROR          = NO
WARN_FORMAT            = \"$file:$line: $text\"
WARN_LOGFILE           =
#---------------------------------------------------------------------------
# Configuration options related to the input files
#---------------------------------------------------------------------------
INPUT                  = .
INPUT_ENCODING         = UTF-8
FILE_PATTERNS          = *.c *.cpp *.h *.hpp
RECURSIVE              = YES
EXCLUDE                =
EXCLUDE_SYMLINKS       = NO
EXCLUDE_PATTERNS       =
EXCLUDE_SYMBOLS        =
EXAMPLE_PATH           =
EXAMPLE_PATTERNS       = *
EXAMPLE_RECURSIVE      = NO
IMAGE_PATH             =
INPUT_FILTER           =
FILTER_PATTERNS        =
FILTER_SOURCE_FILES    = NO
FILTER_SOURCE_PATTERNS =
USE_MDFILE_AS_MAINPAGE =
#---------------------------------------------------------------------------
# Configuration options related to source browsing
#---------------------------------------------------------------------------
SOURCE_BROWSER         = NO
INLINE_SOURCES         = NO
STRIP_CODE_COMMENTS    = NO
REFERENCED_BY_RELATION = NO
REFERENCES_RELATION    = NO
REFERENCES_LINK_SOURCE = YES
SOURCE_TOOLTIPS        = YES
USE_HTAGS              = NO
VERBATIM_HEADERS       = YES
#---------------------------------------------------------------------------
# Configuration options related to the alphabetical class index
#---------------------------------------------------------------------------
ALPHABETICAL_INDEX     = YES
IGNORE_PREFIX          =
#---------------------------------------------------------------------------
# Configuration options related to the HTML output
#---------------------------------------------------------------------------
GENERATE_HTML          = YES
HTML_OUTPUT            = html
HTML_FILE_EXTENSION    = .html
HTML_HEADER            =
HTML_FOOTER            =
HTML_STYLESHEET        =
HTML_EXTRA_STYLESHEET  =
HTML_EXTRA_FILES       =
HTML_COLORSTYLE        = LIGHT
HTML_COLORSTYLE_HUE    = 220
HTML_COLORSTYLE_SAT    = 150
HTML_COLORSTYLE_GAMMA  = 60
HTML_DYNAMIC_MENUS     = YES
HTML_DYNAMIC_SECTIONS  = NO
HTML_INDEX_NUM_ENTRIES = 100
GENERATE_DOCSET        = NO
DOCSET_FEEDNAME        = \"Doxygen generated docs\"
DOCSET_BUNDLE_ID       = org.doxygen.Project
DOCSET_PUBLISHER_ID    = org.doxygen.Publisher
DOCSET_PUBLISHER_NAME  = Publisher
GENERATE_HTMLHELP      = NO
CHM_FILE               =
HHC_LOCATION           =
GENERATE_CHI           = NO
CHM_INDEX_ENCODING     =
BINARY_TOC             = NO
TOC_EXPAND             = NO
GENERATE_QHP           = NO
QCH_FILE               =
QHP_NAMESPACE          = org.doxygen.Project
QHP_VIRTUAL_FOLDER     = doc
QHP_CUST_FILTER_NAME   =
QHP_CUST_FILTER_ATTRS  =
QHP_SECT_FILTER_ATTRS  =
QHG_LOCATION           =
GENERATE_ECLIPSEHELP   = NO
ECLIPSE_DOC_ID         = org.doxygen.Project
DISABLE_INDEX          = NO
GENERATE_TREEVIEW      = NO
ENUM_VALUES_PER_LINE   = 4
TREEVIEW_WIDTH         = 250
EXT_LINKS_IN_WINDOW    = NO
HTML_FORMULA_FORMAT    = png
FORMULA_FONTSIZE       = 10
FORMULA_MACROFILE      =
USE_MATHJAX            = NO
MATHJAX_FORMAT         = HTML-CSS
MATHJAX_RELPATH        =
MATHJAX_EXTENSIONS     =
MATHJAX_CODEFILE       =
SEARCHENGINE           = NO
SERVER_BASED_SEARCH    = NO
EXTERNAL_SEARCH        = NO
SEARCHENGINE_URL       =
SEARCHDATA_FILE        = searchdata.xml
EXTERNAL_SEARCH_ID     =
EXTRA_SEARCH_MAPPINGS  =
#---------------------------------------------------------------------------
# Configuration options related to other outputs
#---------------------------------------------------------------------------
GENERATE_LATEX         = NO
GENERATE_RTF           = NO
GENERATE_MAN           = NO
GENERATE_XML           = NO
GENERATE_DOCBOOK       = NO
GENERATE_AUTOGEN_DEF   = NO
GENERATE_PERLMOD       = NO
#---------------------------------------------------------------------------
# Configuration options related to the preprocessor
#---------------------------------------------------------------------------
ENABLE_PREPROCESSING   = YES
MACRO_EXPANSION        = NO
EXPAND_ONLY_PREDEF     = NO
SEARCH_INCLUDES        = YES
INCLUDE_PATH           = README.md
INCLUDE_FILE_PATTERNS  =
PREDEFINED             =
EXPAND_AS_DEFINED      =
SKIP_FUNCTION_MACROS   = YES
#---------------------------------------------------------------------------
# Configuration options related to external references
#---------------------------------------------------------------------------
TAGFILES               =
GENERATE_TAGFILE       =
ALLEXTERNALS           = NO
EXTERNAL_GROUPS        = YES
EXTERNAL_PAGES         = YES
#---------------------------------------------------------------------------
# Configuration options related to the dot tool
#---------------------------------------------------------------------------
DIA_PATH               =
HIDE_UNDOC_RELATIONS   = YES
HAVE_DOT               = NO
DOT_NUM_THREADS        = 0
DOT_FONTPATH           =
CLASS_GRAPH            = NO
COLLABORATION_GRAPH    = YES
GROUP_GRAPHS           = YES
UML_LOOK               = NO
UML_LIMIT_NUM_FIELDS   = 10
DOT_UML_DETAILS        = NO
DOT_WRAP_THRESHOLD     = 17
TEMPLATE_RELATIONS     = NO
INCLUDE_GRAPH          = YES
INCLUDED_BY_GRAPH      = YES
CALL_GRAPH             = NO
CALLER_GRAPH           = NO
GRAPHICAL_HIERARCHY    = YES
DIRECTORY_GRAPH        = YES
DOT_IMAGE_FORMAT       = png
INTERACTIVE_SVG        = NO
DOT_PATH               =
DOTFILE_DIRS           =
MSCFILE_DIRS           =
DIAFILE_DIRS           =
PLANTUML_JAR_PATH      =
PLANTUML_CFG_FILE      =
PLANTUML_INCLUDE_PATH  =
DOT_GRAPH_MAX_NODES    = 50
MAX_DOT_GRAPH_DEPTH    = 0
DOT_MULTI_TARGETS      = NO
GENERATE_LEGEND        = YES
DOT_CLEANUP            = YES
        "
    };
}

fn init(name: &str, project_type: ProjectType, json: bool) -> Result<()> {
    let path = String::from(name);

    std::fs::create_dir(path.clone())?;
    let project = Project::new(name, project_type)?;
    let mut file = File::create(path.clone() + "/barge.json")?;
    let content = serde_json::to_string_pretty(&project)?;
    file.write_all(content.as_bytes())?;
    file.write_all(b"\n")?;

    if !json {
        std::fs::create_dir(path.clone() + "/src")?;
        std::fs::create_dir(path.clone() + "/include")?;
        let mut file = File::create(path.clone() + "/src/main.cpp")?;
        file.write_all(hello_template!().as_bytes())?;
        let mut file = File::create(path.clone() + "/.gitignore")?;
        file.write_all("build/*\n".as_bytes())?;
        let mut file = File::create(path.clone() + "/Doxyfile")?;
        file.write_all(doxy_template!().as_bytes())?;
        Command::new("git").arg("init").arg(name).output()?;
        color_println!(GREEN, "Project {} successfully created", name);
    } else {
        color_println!(GREEN, "JSON file for project {} successfully created", name);
    }

    Ok(())
}

fn clean() -> Result<()> {
    color_println!(BLUE, "{}", "Removing build artifacts");
    attempt_remove_directory("build")?;
    Ok(())
}

fn lines() -> Result<()> {
    let sources = collect_source_files(false)?;

    let cat = Command::new("cat")
        .args(sources)
        .stdout(Stdio::piped())
        .spawn()?;

    let wc = Command::new("wc")
        .arg("-l")
        .stdin(Stdio::from(
            cat.stdout
                .ok_or(BargeError::NoneOption("Could not get file list"))?,
        ))
        .output()?
        .stdout;
    let mut wc = String::from(std::str::from_utf8(&wc)?);
    wc.pop();

    color_println!(BLUE, "The project contains {} lines of code", wc);
    Ok(())
}

fn in_project_folder() -> bool {
    let metadata = std::fs::metadata("barge.json");
    if let Ok(metadata) = metadata {
        metadata.is_file()
    } else {
        false
    }
}

fn parse_build_target(target: Option<&str>) -> Result<BuildTarget> {
    if let Some(target) = target {
        BuildTarget::try_from(target)
    } else {
        Ok(BuildTarget::Debug)
    }
}

fn parse_and_run_subcommands() -> Result<()> {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .version(env!("CARGO_PKG_VERSION"))
        .about("A simple tool for small assembly/C/C++ projects")
        .setting(clap::AppSettings::SubcommandRequired)
        .subcommand(
            App::new("init")
                .about("Initializes a new project")
                .arg(clap::arg!(--json "Create a barge.json file only in the target directory"))
                .arg(clap::arg!(<NAME> "Name of the project"))
                .arg(clap::arg!([TYPE] "Project type: executable, shared-lib, or static-lib")),
        )
        .subcommand(
            App::new("build")
                .alias("b")
                .about("Builds the current project")
                .arg(clap::arg!([TARGET] "Build target (debug or release)")),
        )
        .subcommand(
            App::new("rebuild")
                .about("Removes build artifacts and builds the current project")
                .arg(clap::arg!([TARGET] "Build target (debug or release)")),
        )
        .subcommand(
            App::new("run")
                .alias("r")
                .about("Builds and runs the current project (binary projects only)")
                .arg(clap::arg!([TARGET] "Build target (debug or release)")),
        )
        .subcommand(App::new("clean").about("Removes build artifacts"))
        .subcommand(App::new("lines").about("Counts the source code lines in the project"))
        .subcommand(App::new("analyze").about("Runs static analysis on the project"))
        .subcommand(App::new("format").about("Formats the source code of the project"))
        .subcommand(App::new("doc").about("Generates HTML documentation for the project"))
        .try_get_matches()?;

    if let Some(init_args) = matches.subcommand_matches("init") {
        let project_name = init_args
            .value_of("NAME")
            .ok_or(BargeError::NoneOption("Couldn't parse project name"))?;

        let project_type = if let Some(project_type) = init_args.value_of("TYPE") {
            match project_type {
                "executable" => Ok(ProjectType::Executable),
                "shared-lib" => Ok(ProjectType::SharedLibrary),
                "shared-library" => Ok(ProjectType::SharedLibrary),
                "static-lib" => Ok(ProjectType::StaticLibrary),
                "static-library" => Ok(ProjectType::StaticLibrary),
                &_ => Err(BargeError::InvalidValue("Invalid project type, valid choices are: executable, shared-lib(rary), static-lib(rary)"))
            }
        } else {
            Ok(ProjectType::Executable)
        };

        let json = init_args.contains_id("json");
        return if let Ok(project_type) = project_type {
            init(project_name, project_type, json)
        } else {
            project_type.map(|_| ())
        };
    }

    if !in_project_folder() {
        color_eprintln!(
            "This subcommand must be run in a project folder, which contains barge.json"
        );
        std::process::exit(1);
    }

    let project = Project::load("barge.json")?;
    if let Some(build_args) = matches.subcommand_matches("build") {
        let target = parse_build_target(build_args.value_of("TARGET"))?;
        project.build(target)?;
    } else if let Some(rebuild_args) = matches.subcommand_matches("rebuild") {
        let target = parse_build_target(rebuild_args.value_of("TARGET"))?;
        project.rebuild(target)?;
    } else if let Some(run_args) = matches.subcommand_matches("run") {
        let target = parse_build_target(run_args.value_of("TARGET"))?;
        project.run(target)?;
    } else if matches.subcommand_matches("clean").is_some() {
        clean()?;
    } else if matches.subcommand_matches("lines").is_some() {
        lines()?;
    } else if matches.subcommand_matches("analyze").is_some() {
        project.analyze()?;
    } else if matches.subcommand_matches("format").is_some() {
        project.format()?;
    } else if matches.subcommand_matches("doc").is_some() {
        project.document()?;
    }

    Ok(())
}

fn main() -> Result<()> {
    let project_dir = look_for_project_directory();
    if let Err(BargeError::ProjectNotFound(s)) = project_dir {
        color_eprintln!("{}", s);
        std::process::exit(1);
    }

    let previous_dir = std::env::current_dir()?;
    std::env::set_current_dir(project_dir?)?;
    let result = parse_and_run_subcommands();
    std::env::set_current_dir(previous_dir)?;

    if let Err(error) = &result {
        match error {
            BargeError::StdIoError(e) => color_eprintln!("{}", e.to_string()),
            BargeError::StdStrUtf8Error(e) => color_eprintln!("{}", e.to_string()),
            BargeError::SerdeJsonError(e) => color_eprintln!("{}", e.to_string()),
            BargeError::ClapError(e) => println!("{}", e),
            BargeError::NoneOption(s) => color_eprintln!("{}", s),
            BargeError::InvalidValue(s) => color_eprintln!("{}", s),
            BargeError::FailedOperation(s) => color_eprintln!("{}", s),
            BargeError::ProjectNotFound(s) => color_eprintln!("{}", s),
        };
        std::process::exit(1);
    }
    std::process::exit(0);
}
