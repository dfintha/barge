use crate::project::Project;
use std::io::{Result, Write};
use std::process::{Command, Stdio};

mod project;

fn usage() {
    println!("Usage: barge [init|build|clean|lines]");
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
    Command::new("git").arg("init").arg(name).output()?;
    println!("Project '{}' successfully created.", name);
    Ok(())
}

fn build(project: &Project) -> Result<()> {
    let makefile = project.generate_makefile();
    let filename = ".barge.makefile";
    let mut file = std::fs::File::create(filename)?;
    file.write_all(makefile.as_bytes())?;
    Command::new("make")
        .arg("-f")
        .arg(".barge.makefile")
        .arg("all")
        .spawn()?
        .wait()?;
    std::fs::remove_file(filename)?;
    Ok(())
}

fn clean() -> Result<()> {
    println!("Removing build artifacts.");
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

    println!("The project contains {} lines of code.", wc);
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
    } else if mode == "clean" {
        clean()?;
    } else if mode == "lines" {
        lines()?;
    } else {
        usage();
    }

    Ok(())
}
