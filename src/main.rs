use crate::project::{BuildMode, Project};
use ansi_term::{Color, Style};
use lazy_static::lazy_static;
use std::io::{Result, Write};
use std::process::{Command, Stdio};
use std::time::Instant;

mod project;

lazy_static! {
    static ref BLUE: Style = Style::new().bold().fg(Color::Blue);
    static ref GREEN: Style = Style::new().bold().fg(Color::Green);
    static ref WHITE: Style = Style::new().bold().fg(Color::White);
}

macro_rules! color_println {
    ($style:tt, $($arg:tt)*) => {
        println!("{}", $style.paint(format!($($arg)*)))
    }
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

The MODE argument can be either 'debug' or 'release'. If non given, the default
is 'debug'."
    );
}

macro_rules! barge_template {
    () => {
        "{{
    \"name\": \"{}\",
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

fn init(name: &str) -> Result<()> {
    let path = String::from(name);

    std::fs::create_dir(path.clone())?;
    std::fs::create_dir(path.clone() + "/src")?;
    std::fs::create_dir(path.clone() + "/include")?;

    {
        let mut file = std::fs::File::create(path.clone() + "/barge.json")?;
        file.write_all(format!(barge_template!(), name).as_bytes())?;
    }
    {
        let mut file = std::fs::File::create(path.clone() + "/src/main.cpp")?;
        file.write_all(hello_template!().as_bytes())?;
    }
    {
        let mut file = std::fs::File::create(path.clone() + "/.gitignore")?;
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

    let mut make = Command::new("make")
        .arg("-f")
        .arg("-")
        .arg("all")
        .stdin(Stdio::piped())
        .spawn()?;

    let makefile = project.generate_makefile(build_mode);
    make.stdin
        .as_mut()
        .unwrap()
        .write_all(makefile.as_bytes())?;
    make.wait()?;

    let finish_time = Instant::now();
    let build_duration = finish_time - start_time;
    color_println!(
        BLUE,
        "Build finished in {:.2} seconds.",
        build_duration.as_secs_f64()
    );
    Ok(())
}

fn run(project: &Project, build_mode: BuildMode) -> Result<()> {
    build(project, build_mode)?;

    let mode_string = match build_mode {
        BuildMode::Debug => "debug",
        BuildMode::Release => "release",
    };

    let path = String::from("bin/") + &mode_string + "/" + &project.name;
    color_println!(BLUE, "Running executable {}", &path);
    Command::new(format!("{}", &path)).spawn()?.wait()?;
    Ok(())
}

fn clean() -> Result<()> {
    color_println!(BLUE, "{}", "Removing build artifacts");
    let _bin = std::fs::remove_dir_all("bin");
    let _obj = std::fs::remove_dir_all("obj");
    Ok(())
}

fn lines() -> Result<()> {
    let find = Command::new("find")
        .arg("src")
        .arg("-type")
        .arg("f")
        .arg("-name")
        .arg("*.c*")
        .output()?
        .stdout;

    let mut find: Vec<_> = std::str::from_utf8(&find).unwrap().split("\n").collect();
    find.retain(|str| str.len() != 0);

    let cat = Command::new("cat")
        .args(find)
        .stdout(Stdio::piped())
        .spawn()?;

    let wc = Command::new("wc")
        .arg("-l")
        .stdin(Stdio::from(cat.stdout.unwrap()))
        .output()?
        .stdout;
    let mut wc = String::from(std::str::from_utf8(&wc).unwrap());
    wc.pop();

    color_println!(BLUE, "The project contains {} lines of code.", wc);
    Ok(())
}

fn parse_build_mode(args: &Vec<String>, index: usize) -> BuildMode {
    if args.len() < (index + 1) || &args[index] == "debug" {
        BuildMode::Debug
    } else if &args[index] == "release" {
        BuildMode::Release
    } else {
        BuildMode::Debug
    }
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
    } else {
        usage();
    }

    Ok(())
}
