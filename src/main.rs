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

fn cmd_type_handler(argument: &str) {
    if BUILTINS.contains(&argument) {
        println!("{argument} is a shell builtin");
        return;
    }

    match find_executable_in_path(argument) {
        Some(path) => println!("{} is {}", argument, path.display()),
        None => println!("{}: not found", argument),
    }
}

struct CmdOutput {
    stdout: Option<String>,
    stderr: Option<String>,
}

fn cmd_external_handler(command: &str, args: &[&str]) -> CmdOutput {
    match find_executable_in_path(command) {
        Some(path) => {
            let output = std::process::Command::new(path.file_name().unwrap())
                .args(args)
                .output()
                .expect("Failed to execute command");

            CmdOutput {
                stdout: if output.stdout.is_empty() {
                    None
                } else {
                    Some(
                        String::from_utf8_lossy(&output.stdout)
                            .trim_end()
                            .to_string(),
                    )
                },
                stderr: if output.stderr.is_empty() {
                    None
                } else {
                    Some(
                        String::from_utf8_lossy(&output.stderr)
                            .trim_end()
                            .to_string(),
                    )
                },
            }
        }
        None => CmdOutput {
            stdout: None,
            stderr: Some(format!("{command}: command not found")),
        },
    }
}

fn cmd_cd_handler(path: &str) {
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
                    if ch == '\'' {
                        break;
                    }
                    current.push(ch);
                }
            }

            '"' => {
                while let Some(ch) = chars.next() {
                    if ch == '\\'
                        && let Some(&next_ch) = chars.peek()
                    {
                        // peek the next chars without consuming
                        current.push(next_ch); // now consume it, skipping the backslash
                        chars.next();
                    } else if ch == '"' {
                        break;
                    } else {
                        current.push(ch);
                    }
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

        let (cmd_tokens, redirect_target, operator) = if let Some(pos) = tokens_ref
            .iter()
            .position(|&t| t == ">" || t == "1>" || t == "2>")
        {
            let target = tokens_ref.get(pos + 1).copied();
            let operator = tokens_ref.get(pos).copied();
            (&tokens_ref[..pos], target, operator)
        } else {
            (tokens_ref.as_slice(), None, None)
        };

        match cmd_tokens {
            [] => {}
            ["exit", ..] => break,
            ["echo", args @ ..] => {
                let output = args.join(" ");
                match operator {
                    Some(">") | Some("1>") => {
                        std::fs::write(redirect_target.unwrap(), output).unwrap();
                    }
                    Some("2>") => {
                        std::fs::write(redirect_target.unwrap(), "").unwrap();
                        println!("{}", output);
                    }
                    _ => {
                        println!("{}", output);
                    }
                }
            }
            ["type", args @ ..] => cmd_type_handler(&args.join(" ")),
            ["pwd", ..] => println!("{}", env::current_dir().unwrap().display().to_string()),
            ["cd", args] => {
                cmd_cd_handler(args);
            }
            [cmd, args @ ..] => {
                let output = cmd_external_handler(cmd, args);
                match operator {
                    Some(">") | Some("1>") => {
                        std::fs::write(redirect_target.unwrap(), output.stdout.unwrap_or_default())
                            .unwrap();
                        output.stderr.map(|e| eprintln!("{}", e));
                    }
                    Some("2>") => {
                        std::fs::write(redirect_target.unwrap(), output.stderr.unwrap_or_default())
                            .unwrap();
                        output.stdout.map(|s| println!("{}", s));
                    }
                    _ => {
                        output.stdout.map(|s| println!("{}", s));
                        output.stderr.map(|e| eprintln!("{}", e));
                    }
                }
            }
        };
    }
}
