use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

const BUILTINS: &[&str] = &["exit", "echo", "type"];

fn find_in_path(command: &str) -> Option<PathBuf> {
    let path_var = env::var("PATH").unwrap_or_default();

    env::split_paths(&path_var)
        .map(|dir| dir.join(command))
        .find(|path| path.exists() && is_executable(path))
}

fn is_executable(path: &PathBuf) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(not(unix))]
    {
        path.extension()
            .map(|e| e == "exe" || e == "bat" || e == "cmd")
            .unwrap_or(false)
    }
}

fn execute_type_command(argument: &str) {
    if BUILTINS.contains(&argument) {
        println!("{argument} is a shell builtin");
        return;
    }

    match find_in_path(argument) {
        Some(path) => println!("{} is {}", argument, path.display()),
        None => println!("{}: not found", argument),
    }
}

fn read_line() -> String {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input
}

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let input = read_line();
        let tokens: Vec<&str> = input.split_whitespace().collect();

        match tokens.as_slice() {
            [] => (),
            ["exit", ..] => break,
            ["echo", args @ ..] => println!("{}", args.join(" ")),
            ["type", args @ ..] => execute_type_command(&args.join(" ")),
            [cmd, ..] => println!("{cmd}: command not found"),
        }
    }
}