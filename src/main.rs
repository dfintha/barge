use crate::project::Project;
use ansi_term::*;
use std::io::{Result, Write};
use std::process::{Command, Stdio};

mod project;

fn usage() {
    println!("Usage: barge [init|build|run|clean|lines]");
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
    println!(
        "{}{}{}",
        Style::new().bold().fg(Color::Green).paint("Project '"),
        Style::new().bold().fg(Color::Green).paint(name),
        Style::new()
            .bold()
            .fg(Color::Green)
            .paint("' successfully created.")
    );
    Ok(())
}

fn build(project: &Project) -> Result<()> {
    let mut make = Command::new("make")
        .arg("-f")
        .arg("-")
        .arg("all")
        .stdin(Stdio::piped())
        .spawn()?;

    let makefile = project.generate_makefile();
    make.stdin
        .as_mut()
        .unwrap()
        .write_all(makefile.as_bytes())?;
    make.wait()?;
    Ok(())
}

fn run(project: &Project) -> Result<()> {
    build(project)?;

    println!(
        "{}{}",
        Style::new()
            .bold()
            .fg(Color::Green)
            .paint("Running executable bin/"),
        Style::new()
            .bold()
            .fg(Color::Green)
            .paint(project.name.clone())
    );
    Command::new(format!("bin/{}", project.name.clone()).as_str())
        .spawn()?
        .wait()?;

    Ok(())
}

fn clean() -> Result<()> {
    println!(
        "{}",
        Style::new()
            .bold()
            .fg(Color::Red)
            .paint("Removing build artifacts.")
    );
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

    println!(
        "{}{}{}",
        Style::new()
            .bold()
            .fg(Color::Blue)
            .paint("The project contains "),
        Style::new().bold().fg(Color::Blue).paint(wc),
        Style::new().bold().fg(Color::Blue).paint(" lines of code.")
    );
    Ok(())
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
    if mode == "build" {
        build(&project)?;
    } else if mode == "run" {
        run(&project)?;
    } else if mode == "clean" {
        clean()?;
    } else if mode == "lines" {
        lines()?;
    } else {
        usage();
    }

    Ok(())
}
