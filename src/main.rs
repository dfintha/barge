use crate::makefile::{generate_analyze_makefile, generate_build_makefile, BuildTarget};
use crate::project::{Project, ProjectType};
use crate::result::{BargeError, Result};
use ansi_term::{Color, Style};
use clap::App;
use lazy_static::lazy_static;
use std::convert::TryFrom;
use std::fs::File;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Instant;
use sysinfo::SystemExt;

mod makefile;
mod project;
mod result;

lazy_static! {
    static ref NO_COLOR: bool = std::env::var("NO_COLOR").is_ok();
    static ref BLUE: Style = Style::new().bold().fg(Color::Blue);
    static ref GREEN: Style = Style::new().bold().fg(Color::Green);
    static ref RED: Style = Style::new().bold().fg(Color::Red);
    static ref WHITE: Style = Style::new().bold().fg(Color::White);
}

macro_rules! color_println {
    ($style:tt, $($arg:tt)*) => {
        if *NO_COLOR {
            println!("{}", format!($($arg)*))
        } else {
            println!("{}", $style.paint(format!($($arg)*)))
        }
    }
}

macro_rules! color_eprintln {
    ($($arg:tt)*) => {
        if *NO_COLOR {
            eprintln!("{}", format!($($arg)*))
        } else {
            eprintln!("{}", RED.paint(format!($($arg)*)))
        }
    }
}

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

fn build(project: &Project, target: BuildTarget) -> Result<()> {
    color_println!(
        BLUE,
        "Building project with {} configuration",
        target.to_string()
    );
    let start_time = Instant::now();

    let makeopts = if let Some(makeopts) = &project.custom_makeopts {
        makeopts.split(' ').map(|str| str.to_string()).collect()
    } else {
        generate_default_makeopts()?
    };

    let mut make = Command::new("make")
        .arg("-s")
        .arg("-f")
        .arg("-")
        .arg("all")
        .args(makeopts)
        .stdin(Stdio::piped())
        .spawn()?;

    let makefile = generate_build_makefile(project, target)?;
    make.stdin
        .as_mut()
        .ok_or(BargeError::NoneOption("Could not interact with make"))?
        .write_all(makefile.as_bytes())?;
    let status = make.wait()?.success();

    if status {
        let finish_time = Instant::now();
        let build_duration = finish_time - start_time;
        color_println!(
            BLUE,
            "Build finished in {:.2} seconds",
            build_duration.as_secs_f64()
        );
        Ok(())
    } else {
        color_eprintln!("Build failed");
        Err(BargeError::FailedOperation(
            "One or more dependencies failed to build",
        ))
    }
}

fn rebuild(project: &Project, target: BuildTarget) -> Result<()> {
    color_println!(BLUE, "{}", "Removing relevant build artifacts");
    std::fs::remove_dir_all(format!("./bin/{}", target.to_string()))?;
    std::fs::remove_dir_all(format!("./obj/{}", target.to_string()))?;
    build(project, target)
}

fn analyze(project: &Project) -> Result<()> {
    color_println!(BLUE, "Running static analysis on project");

    let mut make = Command::new("make")
        .arg("-s")
        .arg("-f")
        .arg("-")
        .arg("analyze")
        .stdin(Stdio::piped())
        .spawn()?;

    let makefile = generate_analyze_makefile(project)?;

    make.stdin
        .as_mut()
        .ok_or(BargeError::NoneOption("Could not interact with make"))?
        .write_all(makefile.as_bytes())?;
    make.wait()?;

    Ok(())
}

fn run(project: &Project, target: BuildTarget) -> Result<()> {
    if project.project_type != ProjectType::Executable {
        color_eprintln!("Only binary projects can be run");
        return Ok(());
    }

    build(project, target)?;

    let path = String::from("bin/") + &target.to_string() + "/" + &project.name;
    color_println!(BLUE, "Running executable {}", &path);
    Command::new(&path).spawn()?.wait()?;
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

fn format(project: &Project) -> Result<()> {
    let sources = collect_source_files()?;
    let style_arg = if let Some(format_style) = &project.format_style {
        "--style=".to_string() + format_style
    } else {
        "--style=Google".to_string()
    };

    Command::new("clang-format")
        .arg("-i")
        .arg(style_arg)
        .args(sources)
        .spawn()?
        .wait()?;

    color_println!(BLUE, "The project source files were formatted");
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

fn generate_default_makeopts() -> Result<Vec<String>> {
    let mut system = sysinfo::System::new_all();
    system.refresh_all();

    let processor_cores = system.processors().len() as u64;
    let free_memory_in_kb = system.total_memory() - system.used_memory();
    let free_2g_memory = free_memory_in_kb / (2 * 1024 * 1024);
    let parallel_jobs = std::cmp::max(1, std::cmp::min(processor_cores, free_2g_memory));

    Ok(vec![format!("-j{}", parallel_jobs)])
}

fn parse_build_target(target: Option<&str>) -> Result<BuildTarget> {
    if let Some(target) = target {
        BuildTarget::try_from(target)
    } else {
        Ok(BuildTarget::Debug)
    }
}

fn collect_source_files() -> Result<Vec<String>> {
    let find_src = Command::new("find")
        .arg("src")
        .args(vec!["-type", "f"])
        .args(vec!["-name", "*.c"])
        .args(vec!["-o", "-name", "*.cpp"])
        .args(vec!["-o", "-name", "*.s"])
        .args(vec!["-o", "-name", "*.h"])
        .args(vec!["-o", "-name", "*.hpp"])
        .output()?
        .stdout;

    let mut find_src: Vec<_> = std::str::from_utf8(&find_src)?.split('\n').collect();

    let find_include = Command::new("find")
        .arg("include")
        .args(vec!["-type", "f"])
        .args(vec!["-name", "*.h"])
        .args(vec!["-o", "-name", "*.hpp"])
        .output()?
        .stdout;

    let mut found: Vec<_> = std::str::from_utf8(&find_include)?.split('\n').collect();
    found.append(&mut find_src);
    found.retain(|str| !str.is_empty());

    Ok(found.iter().map(|s| s.to_string()).collect())
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
        build(&project, target)?;
    } else if let Some(rebuild_args) = matches.subcommand_matches("rebuild") {
        let target = parse_build_target(rebuild_args.value_of("TARGET"))?;
        rebuild(&project, target)?;
    } else if let Some(run_args) = matches.subcommand_matches("run") {
        let target = parse_build_target(run_args.value_of("TARGET"))?;
        run(&project, target)?;
    } else if matches.subcommand_matches("clean").is_some() {
        clean()?;
    } else if matches.subcommand_matches("lines").is_some() {
        lines()?;
    } else if matches.subcommand_matches("analyze").is_some() {
        analyze(&project)?;
    } else if matches.subcommand_matches("format").is_some() {
        format(&project)?;
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
