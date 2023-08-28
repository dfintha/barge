use crate::makefile::BuildTarget;
use crate::output::*;
use crate::project::{collect_source_files, Project};
use crate::result::{BargeError, Result};
use clap::App;
use std::fs::File;
use std::io::Write;
use std::process::{Command, Stdio};

mod makefile;
mod output;
mod project;
mod result;
mod scripts;

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

fn init(name: &str) -> Result<()> {
    let path = String::from(name);

    std::fs::create_dir(path.clone())?;
    std::fs::create_dir(path.clone() + "/src")?;
    std::fs::create_dir(path.clone() + "/include")?;

    {
        let project = Project::new(name)?;
        let mut file = File::create(path.clone() + "/barge.json")?;
        let json = serde_json::to_string_pretty(&project)?;
        file.write_all(json.as_bytes())?;
        file.write_all(b"\n")?;
    }
    {
        let mut file = File::create(path.clone() + "/src/main.cpp")?;
        file.write_all(hello_template!().as_bytes())?;
    }
    {
        let mut file = File::create(path + "/.gitignore")?;
        file.write_all("bin/*\nobj/*\n".as_bytes())?;
    }

    Command::new("git").arg("init").arg(name).output()?;

    color_println!(GREEN, "Project {} successfully created", name);
    Ok(())
}

fn clean() -> Result<()> {
    color_println!(BLUE, "{}", "Removing build artifacts");
    let _bin = std::fs::remove_dir_all("bin");
    let _obj = std::fs::remove_dir_all("obj");
    Ok(())
}

fn lines() -> Result<()> {
    let sources = collect_source_files()?;

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
                .arg(clap::arg!(<NAME> "The name of the project")),
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
        .try_get_matches()?;

    if let Some(init_args) = matches.subcommand_matches("init") {
        let name = init_args
            .value_of("NAME")
            .ok_or(BargeError::NoneOption("Couldn't parse project name"))?;
        return init(name);
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
    }

    Ok(())
}

fn main() -> Result<()> {
    let result = parse_and_run_subcommands();
    if let Err(error) = &result {
        match error {
            BargeError::StdIoError(e) => color_eprintln!("{}", e.to_string()),
            BargeError::StdStrUtf8Error(e) => color_eprintln!("{}", e.to_string()),
            BargeError::SerdeJsonError(e) => color_eprintln!("{}", e.to_string()),
            BargeError::ClapError(e) => println!("{}", e),
            BargeError::NoneOption(s) => color_eprintln!("{}", s),
            BargeError::InvalidValue(s) => color_eprintln!("{}", s),
            BargeError::FailedOperation(s) => color_eprintln!("{}", s),
        };
        std::process::exit(1);
    }
    std::process::exit(0);
}
