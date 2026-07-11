use std::fs;
use std::io::{self, Read, Write};

use crate::builtins;
use crate::utils::get_path_executables_deduped;

use crate::terminal;
use crate::trie::Trie;

/// Display the shell prompt.
fn print_prompt() {
    print!("$ ");
    io::stdout().flush().unwrap();
}

/// Build a [`Trie`] populated with builtin command names and executables
/// found on `$PATH`.
fn build_completion_trie() -> Trie {
    let mut trie = Trie::new();

    let registry = builtins::BuiltinRegistry::new();
    for name in &registry.names() {
        trie.insert(name);
    }
    for exe in get_path_executables_deduped() {
        trie.insert(&exe);
    }

    trie
}

fn build_custom_trie(names: Vec<String>) -> Trie {
    let mut trie = Trie::new();
    for name in names {
        trie.insert(&name);
    }
    trie
}

/// Read a single line of input from the terminal.
///
/// This enters raw mode so individual key-presses can be processed,
/// providing tab-completion via a [`Trie`] built from builtin commands
/// and `$PATH` executables.  Raw mode is restored before returning.
pub fn read_line() -> String {
    let trie = build_completion_trie();

    terminal::set_raw_mode(true).expect("failed to enable raw mode");
    print_prompt();

    let mut line = String::new();
    let stdin = io::stdin();
    let mut byte = [0u8; 1];
    let mut first_tab = false;
    let mut first_space = false;

    loop {
        stdin.lock().read_exact(&mut byte).unwrap();

        match byte[0] {
            b'\t' => {
                if first_space {
                    first_tab = false;

                    handle_tab_files(&mut line /*, &mut first_tab*/)
                } else {
                    handle_tab_executable(&trie, &mut line, &mut first_tab)
                }
            }
            b'\r' | b'\n' => {
                terminal::set_raw_mode(false).expect("failed to restore terminal");
                println!();
                return line;
            }
            127 | 8 => handle_backspace(&mut line),
            c if c >= 32 => {
                if c == 32 {
                    first_space = true;
                }
                let ch = c as char;
                line.push(ch);
                print!("{ch}");
                io::stdout().flush().unwrap();
                first_tab = false;
            }
            _ => {}
        }
    }
}

// ------------------------------------------------------------------
// Key handlers
// ------------------------------------------------------------------

/// Process a <Tab> press: attempt autocompletion on the current `line`.
fn handle_tab_executable(trie: &Trie, line: &mut String, first_tab: &mut bool) {
    let extension = trie.longest_common_extension(line);
    let mut predictions = trie.autocomplete(line);

    if predictions.len() == 1 {
        // if only 1 prediction exist
        *line = predictions.remove(0) + " ";
        print!("\r$ {line}");
    } else if let Some(ext) = extension {
        // if more than one prediction exist, get the longest common string
        line.push_str(&ext);
        print!("\r$ {line}");
    } else if predictions.len() > 1 && *first_tab {
        // if the two criteria above is not satisfied list the available prediction
        predictions.sort();
        io::stdout().flush().unwrap();
        print!("\n{}", predictions.join(" "));
        print!("\n$ {line}");
    } else {
        print!("\x07"); // bell
    }

    *first_tab = true;
    io::stdout().flush().unwrap();
}

fn handle_tab_files(line: &mut String /*, first_tab: &mut bool*/) {
    let mut files = get_file_names(".").unwrap_or_default();
    let mut trie = build_custom_trie(files.clone());
    let mut parts = line.split_whitespace();
    let first = parts.next().unwrap_or("");
    let second = parts.next().unwrap_or("");

    let mut predictions = trie.autocomplete(second);
    if predictions.len() >= 1 {
        *line = format!("{} {} ", first, predictions[0]);
        print!("\r$ {}", line);
    } else {
        print!("\x07"); // bell
    }

    io::stdout().flush().unwrap();
}

fn get_file_names(dir: &str) -> std::io::Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.to_string());
            }
        }
    }
    Ok(names)
}

/// Process a backspace / delete-backward press.
fn handle_backspace(line: &mut String) {
    if line.pop().is_some() {
        print!("\x08 \x08");
    } else {
        print!("\x07"); // bell
    }
    io::stdout().flush().unwrap();
}
