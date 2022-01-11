use crate::makefile::{generate_analyze_makefile, generate_build_makefile, BuildMode};
use crate::project::Project;
use crate::result::{BargeError, Result};
use ansi_term::{Color, Style};
use lazy_static::lazy_static;
use std::fs::File;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Instant;
use sysinfo::SystemExt;

mod makefile;
mod project;
mod result;

lazy_static! {
    static ref BLUE: Style = Style::new().bold().fg(Color::Blue);
    static ref GREEN: Style = Style::new().bold().fg(Color::Green);
    static ref RED: Style = Style::new().bold().fg(Color::Red);
    static ref WHITE: Style = Style::new().bold().fg(Color::White);
}

macro_rules! color_println {
    ($style:tt, $($arg:tt)*) => {
        println!("{}", $style.paint(format!($($arg)*)))
    }
}

macro_rules! color_eprintln {
    ($($arg:tt)*) => {
        eprintln!("{}", RED.paint(format!($($arg)*)))
    }
}

macro_rules! barge_template {
    () => {
        "{{
    \"name\": \"{}\",
    \"version\": \"0.1.0\",
    \"c_standard\": \"c99\",
    \"cpp_standard\": \"c++14\"
}}
"
    };
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

fn usage() {
    println!(
        "A very basic build tool for very basic assembly/C/C++ projects.

USAGE:
    barge [SUBCOMMAND] [OPTIONS]

The available subcommands are:
    build, b [MODE]     Builds the current project in the given build mode
    clean               Remove the build artifacts
    init [NAME]         Create a new project named NAME in a new directory
    run, r [MODE]       Runs the binary of the current project
    rebuild [MODE]      Removed build artifacts, and builds the project
    analyze             Perform static analysis on the project

The MODE argument can be either 'debug' or 'release'. If non given, the default
is 'debug'."
    );
}

fn init(name: &str) -> Result<()> {
    let path = String::from(name);

    std::fs::create_dir(path.clone())?;
    std::fs::create_dir(path.clone() + "/src")?;
    std::fs::create_dir(path.clone() + "/include")?;

    {
        let mut file = File::create(path.clone() + "/barge.json")?;
        file.write_all(format!(barge_template!(), name).as_bytes())?;
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

fn build(project: &Project, build_mode: BuildMode) -> Result<()> {
    let mode_string = match build_mode {
        BuildMode::Debug => "debug",
        BuildMode::Release => "release",
    };

    color_println!(BLUE, "Building project in {} mode", mode_string);
    let start_time = Instant::now();

    let makeopts = if let Some(makeopts) = &project.custom_makeopts {
        makeopts.split(' ').map(|str| str.to_string()).collect()
    } else {
        generate_default_makeopts()?
    };

    let mut make = Command::new("make")
        .arg("-f")
        .arg("-")
        .arg("all")
        .args(makeopts)
        .stdin(Stdio::piped())
        .spawn()?;

    let makefile = generate_build_makefile(&project, build_mode)?;
    make.stdin
        .as_mut()
        .ok_or(BargeError::NoneOption)?
        .write_all(makefile.as_bytes())?;
    make.wait()?;

    let finish_time = Instant::now();
    let build_duration = finish_time - start_time;
    color_println!(
        BLUE,
        "Build finished in {:.2} seconds",
        build_duration.as_secs_f64()
    );
    Ok(())
}

fn analyze(project: &Project) -> Result<()> {
    color_println!(BLUE, "Running static analysis on project");

    let mut make = Command::new("make")
        .arg("-f")
        .arg("-")
        .arg("analyze")
        .stdin(Stdio::piped())
        .spawn()?;

    let makefile = generate_analyze_makefile(&project)?;

    make.stdin
        .as_mut()
        .ok_or(BargeError::NoneOption)?
        .write_all(makefile.as_bytes())?;
    make.wait()?;

    Ok(())
}

fn run(project: &Project, build_mode: BuildMode) -> Result<()> {
    build(project, build_mode)?;

    let mode_string = match build_mode {
        BuildMode::Debug => "debug",
        BuildMode::Release => "release",
    };

    let path = String::from("bin/") + mode_string + "/" + &project.name;
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
        .stdin(Stdio::from(cat.stdout.ok_or(BargeError::NoneOption)?))
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

fn generate_default_makeopts() -> Result<Vec<String>> {
    let mut system = sysinfo::System::new_all();
    system.refresh_all();

    let processor_cores = system.processors().len() as u64;
    let free_memory_in_kb = system.total_memory() - system.used_memory();
    let free_2g_memory = free_memory_in_kb / (2 * 1024 * 1024);
    let parallel_jobs = std::cmp::max(1, std::cmp::min(processor_cores, free_2g_memory));

    Ok(vec![format!("-j{}", parallel_jobs)])
}

fn parse_build_mode(args: &[String], index: usize) -> BuildMode {
    if args.len() < (index + 1) || &args[index] == "debug" {
        BuildMode::Debug
    } else if &args[index] == "release" {
        BuildMode::Release
    } else {
        BuildMode::Debug
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

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        usage();
        return Ok(());
    }

    let mode = &args[1];
    if mode == "init" {
        if args.len() < 3 {
            usage();
            return Ok(());
        }
        init(&args[2])?;
        return Ok(());
    }

    if !in_project_folder() {
        color_eprintln!("This command must be run in a project folder, which contains barge.json");
        return Ok(());
    }

    let project = Project::load("barge.json")?;
    if mode == "build" || mode == "b" {
        let build_mode = parse_build_mode(&args, 2);
        build(&project, build_mode)?;
    } else if mode == "rebuild" {
        let build_mode = parse_build_mode(&args, 2);
        clean()?;
        build(&project, build_mode)?;
    } else if mode == "run" || mode == "r" {
        let build_mode = parse_build_mode(&args, 2);
        run(&project, build_mode)?;
    } else if mode == "clean" {
        clean()?;
    } else if mode == "lines" {
        lines()?;
    } else if mode == "analyze" {
        analyze(&project)?;
    } else {
        usage();
    }

    Ok(())
}
