use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::str::FromStr;

const BUILTINS: &[&str] = &["exit", "echo", "type", "pwd", "cd"];

fn find_executable_in_path(command: &str) -> Option<PathBuf> {
    let path_var = env::var("PATH").unwrap_or_default();

    env::split_paths(&path_var)
        .map(|dir| dir.join(command))
        .find(|target| target.exists() && is_executable(target))
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

    match find_executable_in_path(argument) {
        Some(path) => println!("{} is {}", argument, path.display()),
        None => println!("{}: not found", argument),
    }
}

fn execute_external_command(command: &str, args: &[&str]) {
    match find_executable_in_path(command) {
        Some(path) => {
            let mut child = std::process::Command::new(path.file_name().unwrap())
                .args(args)
                .spawn()
                .expect("Failed to execute command");

            child.wait().expect("Failed to wait on child");
        }
        None => println!("{command}: command not found"),
    }
}

fn execute_cd_command(path: &str) {
    if PathBuf::from_str(path).unwrap().exists() {
        env::set_current_dir(path).unwrap();
    } else if path == "~" {
        env::set_current_dir(env::var("HOME").unwrap()).unwrap();
    } else {
        println!("cd: {path}: No such file or directory");
    }
}


/*

         space
        ┌─────────────────────────────┐
        ▼                             │
  ┌──────────┐   ' or "   ┌─────────────────┐
  │  NORMAL  │──────────► │   IN_QUOTE      │
  │          │◄────────── │                 │
  └──────────┘  ' or "    └─────────────────┘
   (closing)               (closing quote)

*/
fn tokenize(input: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut chars = input.trim_end().chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                if let Some(&next_ch) = chars.peek() {
                    current.push(next_ch);
                    chars.next();
                }
            }

            '\'' => {
                for ch in chars.by_ref() {
                    if ch == '\'' { break; }
                    current.push(ch);
                }
            }
            '"' => {
                for ch in chars.by_ref() {
                    if ch == '"' { break; }
                    current.push(ch);
                }
            }
            ' ' => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
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
        let tokens = tokenize(&input);
        let tokens_ref = tokens.iter().map(|s| s.as_str()).collect::<Vec<&str>>();

        match tokens_ref.as_slice() {
            [] => (),
            ["exit", ..] => break,
            ["echo", args @ ..] => println!("{}", args.join(" ")),
            ["type", args @ ..] => execute_type_command(&args.join(" ")),
            ["pwd", ..] => println!("{}", env::current_dir().unwrap().display()),
            ["cd", args] => execute_cd_command(&args),
            [cmd, args @ ..] => execute_external_command(cmd, args),
        }
    }
}
