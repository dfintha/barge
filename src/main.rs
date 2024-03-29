use crate::makefile::BuildTarget;
use crate::output::*;
use crate::project::{collect_source_files, CollectSourceFilesMode, Project, ProjectType};
use crate::result::{print_error, BargeError, Result};
use crate::utilities::{attempt_remove_directory, look_for_project_directory};
use std::fs::File;
use std::io::Write;
use std::process::{Command, Stdio};

mod makefile;
mod output;
mod project;
mod result;
mod scripts;
mod utilities;

fn init(name: String, project_type: ProjectType, json: bool) -> Result<()> {
    std::fs::create_dir(name.clone())?;
    let project = Project::new(&name, project_type)?;
    let mut file = File::create(name.clone() + "/barge.json")?;
    let content = serde_json::to_string_pretty(&project)?;
    file.write_all(content.as_bytes())?;
    file.write_all(b"\n")?;

    if !json {
        std::fs::create_dir(name.clone() + "/res")?;
        std::fs::create_dir(name.clone() + "/src")?;
        std::fs::create_dir(name.clone() + "/include")?;
        let mut file = File::create(name.clone() + "/.gitignore")?;
        file.write_all("build/*\n".as_bytes())?;
        let mut file = File::create(name.clone() + "/README.md")?;
        file.write_all(format!("# `{}`\n", &name).as_bytes())?;
        let mut file = File::create(name.clone() + "/Doxyfile")?;
        file.write_all(include_str!("template-doxyfile.in").as_bytes())?;
        let mut file = File::create(name.clone() + "/res/doxygen-style.css")?;
        file.write_all(include_str!("template-doxygen-style.css").as_bytes())?;
        let mut file = File::create(name.clone() + "/src/main.cpp")?;
        file.write_all(include_str!("template-main.in").as_bytes())?;
        Command::new("git").arg("init").arg(&name).output()?;
        color_println!(GREEN, "Project {} successfully created", &name);
    } else {
        color_println!(
            GREEN,
            "JSON file for project {} successfully created",
            &name
        );
    }

    Ok(())
}

fn clean() -> Result<()> {
    color_println!(BLUE, "{}", "Removing build artifacts");
    attempt_remove_directory("build")?;
    Ok(())
}

fn lines() -> Result<()> {
    let sources = collect_source_files(CollectSourceFilesMode::All)?;

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

fn in_project_directory() -> bool {
    let metadata = std::fs::metadata("barge.json");
    if let Ok(metadata) = metadata {
        metadata.is_file()
    } else {
        false
    }
}

fn parse_build_target(target: Option<&String>) -> Result<BuildTarget> {
    if let Some(target) = target {
        BuildTarget::try_from(target.as_str())
    } else {
        Ok(BuildTarget::Debug)
    }
}

fn parse_and_run_subcommands() -> Result<()> {
    let matches = clap::Command::new(env!("CARGO_PKG_NAME"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .version(env!("CARGO_PKG_VERSION"))
        .about("A simple tool for small assembly/C/C++ projects")
        .subcommand_required(true)
        .subcommand(
            clap::Command::new("init")
                .about("Initializes a new project")
                .arg(clap::arg!(--json "Create a barge.json file only in the target directory"))
                .arg(clap::arg!(<NAME> "Name of the project"))
                .arg(clap::arg!([TYPE] "Project type: executable, shared-lib, or static-lib")),
        )
        .subcommand(
            clap::Command::new("build")
                .alias("b")
                .about("Builds the current project")
                .arg(clap::arg!([TARGET] "Build target (debug or release)")),
        )
        .subcommand(
            clap::Command::new("rebuild")
                .about("Removes build artifacts and builds the current project")
                .arg(clap::arg!([TARGET] "Build target (debug or release)")),
        )
        .subcommand(
            clap::Command::new("run")
                .alias("r")
                .about("Builds and runs the current project (binary projects only)")
                .arg(clap::arg!([TARGET] "Build target (debug or release)"))
                .arg(
                    clap::Arg::new("args")
                        .allow_hyphen_values(true)
                        .last(true)
                        .raw(true),
                ),
        )
        .subcommand(
            clap::Command::new("debug")
                .alias("d")
                .about("Builds and runs the current project in the debugger (binary projects only)")
                .arg(clap::arg!([TARGET] "Build target (debug or release)"))
                .arg(
                    clap::Arg::new("args")
                        .allow_hyphen_values(true)
                        .last(true)
                        .raw(true),
                ),
        )
        .subcommand(clap::Command::new("clean").about("Removes build artifacts"))
        .subcommand(
            clap::Command::new("lines").about("Counts the source code lines in the project"),
        )
        .subcommand(clap::Command::new("analyze").about("Runs static analysis on the project"))
        .subcommand(clap::Command::new("format").about("Formats the source code of the project"))
        .subcommand(clap::Command::new("doc").about("Generates HTML documentation for the project"))
        .try_get_matches()?;

    if let Some(init_args) = matches.subcommand_matches("init") {
        let project_name: &String = init_args
            .get_one("NAME")
            .ok_or(BargeError::NoneOption("Couldn't parse project name"))?;

        let project_type = if let Some(&project_type) = (*init_args).get_one("TYPE") {
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

        let json = init_args.contains_id("json") && *init_args.get_one("json").unwrap_or(&false);
        return if let Ok(project_type) = project_type {
            init(project_name.to_string(), project_type, json)?;
            std::process::exit(0);
        } else {
            project_type.map(|_| ())
        };
    }

    let project_dir = look_for_project_directory()?;
    let previous_dir = std::env::current_dir()?;
    std::env::set_current_dir(project_dir)?;

    if !in_project_directory() {
        std::env::set_current_dir(previous_dir)?;
        return Err(BargeError::ProjectNotFound("Project file not found."));
    }

    let project = Project::load("barge.json")?;
    if let Some(build_args) = matches.subcommand_matches("build") {
        let target = parse_build_target(build_args.get_one::<String>("TARGET"))?;
        project.build(target)?;
    } else if let Some(rebuild_args) = matches.subcommand_matches("rebuild") {
        let target = parse_build_target(rebuild_args.get_one::<String>("TARGET"))?;
        project.rebuild(target)?;
    } else if let Some(run_args) = matches.subcommand_matches("run") {
        let target = parse_build_target(run_args.get_one::<String>("TARGET"))?;
        let arguments = if let Some(args) = run_args.get_many::<String>("args") {
            args.cloned().collect()
        } else {
            vec![]
        };
        project.run(target, arguments)?;
    } else if let Some(debug_args) = matches.subcommand_matches("debug") {
        let target = parse_build_target(debug_args.get_one::<String>("TARGET"))?;
        let arguments = if let Some(args) = debug_args.get_many::<String>("args") {
            args.cloned().collect()
        } else {
            vec![]
        };
        project.debug(target, arguments)?;
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

    Ok(std::env::set_current_dir(previous_dir)?)
}

fn main() -> Result<()> {
    if let Err(error) = parse_and_run_subcommands() {
        print_error(&error);
        std::process::exit(1);
    }
    std::process::exit(0);
}
