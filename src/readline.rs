use std::fs;
use std::io::{self, Read, Write};

use crate::builtins;
use crate::builtins::BuiltinContext;
use crate::utils::get_path_executables_deduped;

use crate::terminal;
use crate::tokenizer;
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

/// Extract the completion context from a partial input line.
///
/// Returns `(words_before_cursor, prefix_being_typed)`.
/// If the cursor is at a fresh word boundary (trailing space), `prefix` is `""`.
///
/// Uses the quote/escape-aware [`tokenizer::split_words`] so that
/// quoted and escaped strings are handled consistently with execution
/// tokenization.
fn completion_context(input: &str) -> (Vec<String>, String) {
    // Trailing unescaped space means the cursor is at a new-word position.
    if input.ends_with(' ') && !input.ends_with("\\ ") {
        let words = tokenizer::split_words(input);
        return (words, String::new());
    }

    let words = tokenizer::split_words(input);
    match words.split_last() {
        Some((last, rest)) => (rest.to_vec(), last.clone()),
        None => (vec![], String::new()),
    }
}

/// Read a single line of input from the terminal.
///
/// This enters raw mode so individual key-presses can be processed,
/// providing tab-completion via a [`Trie`] built from builtin commands
/// and `$PATH` executables.  Raw mode is restored before returning.
pub fn read_line(ctx: &BuiltinContext) -> String {
    let trie = build_completion_trie();

    terminal::set_raw_mode(true).expect("failed to enable raw mode");
    print_prompt();

    let mut line = String::new();
    let stdin = io::stdin();
    let mut byte = [0u8; 1];
    let mut tab_count: u32 = 0;

    loop {
        stdin.lock().read_exact(&mut byte).unwrap();

        match byte[0] {
            b'\t' => {
                tab_count += 1;

                let (prev_words, prefix) = completion_context(&line);

                if is_completion_exist(&prev_words[0], ctx) {
                    handle_tab_completion(&mut line, &prev_words[0], ctx);
                } else if prev_words.is_empty() {
                    handle_tab_executable(&trie, &mut line, &prefix, tab_count);
                } else {
                    handle_tab_files(&mut line, &prev_words, &prefix, tab_count);
                }
            }
            b'\r' | b'\n' => {
                terminal::set_raw_mode(false).expect("failed to restore terminal");
                println!();
                return line;
            }
            127 | 8 => {
                handle_backspace(&mut line);
                tab_count = 0;
            }
            c if c >= 32 => {
                let ch = c as char;
                line.push(ch);
                print!("{ch}");
                io::stdout().flush().unwrap();
                tab_count = 0;
            }
            _ => {}
        }
    }
}

// ------------------------------------------------------------------
// Key handlers
// ------------------------------------------------------------------

/// Process a `<Tab>` press: attempt autocompletion for executable/command names.
fn handle_tab_executable(trie: &Trie, line: &mut String, prefix: &str, tab_count: u32) {
    let extension = trie.longest_common_extension(prefix);
    let mut predictions = trie.autocomplete(prefix);

    if predictions.len() == 1 {
        // Only one match — complete it and add a trailing space.
        *line = predictions.remove(0) + " ";
        print!("\r$ {line}");
    } else if let Some(ext) = extension {
        // Multiple matches sharing a common extension — fill it in.
        line.push_str(&ext);
        print!("\r$ {line}");
    } else if predictions.len() > 1 && tab_count >= 2 {
        // Second tab with ambiguous completions — list them all.
        predictions.sort();
        io::stdout().flush().unwrap();
        print!("\n{}", predictions.join(" "));
        print!("\n$ {line}");
    } else {
        print!("\x07"); // bell
    }

    io::stdout().flush().unwrap();
}

/// Process a `<Tab>` press: attempt autocompletion for file (and directory) names.
fn handle_tab_files(line: &mut String, prev_words: &[String], prefix: &str, tab_count: u32) {
    // TODO: When prefix contains '/', split into (directory, partial_name)
    // and list entries from that directory instead of ".".
    // e.g. "src/tok" → dir = "src", partial = "tok"
    // This will also need to reconstruct the completed path with the
    // directory prefix when writing back to `line`.
    let mut base_directory = ".";
    let mut leaf = prefix;

    if let Some(idx) = prefix.rfind("/") {
        (base_directory, leaf) = prefix.split_at(idx + 1);
    }

    let mut files = get_files_list(base_directory).unwrap_or_default();
    let directories = get_directories_list(base_directory).unwrap_or_default();
    files.extend(directories);
    let trie = build_custom_trie(files);

    let completion = trie.longest_common_extension(leaf);
    let mut predictions = trie.autocomplete(leaf);

    if let Some(ext) = completion {
        // Multiple matches sharing a common extension — fill it in.
        line.push_str(&ext);
        if !ext.rfind('/').is_some() && predictions.len() == 1 {
            line.push(' ');
        }
        print!("\r$ {line}");
    } else if predictions.len() == 1 {
        // Single match — rebuild line with the completed filename.
        let mut new_line = prev_words.join(" ");
        new_line.push_str(base_directory);
        new_line.push(' ');
        new_line.push_str(&predictions[0]);
        new_line.push(' ');
        *line = new_line;
        print!("\r$ {}", line);
    } else if predictions.len() > 1 && tab_count >= 2 {
        // Second tab with ambiguous completions — list them all.
        predictions.sort();
        io::stdout().flush().unwrap();
        print!("\n{}", predictions.join(" "));
        print!("\n$ {line}");
    } else {
        print!("\x07"); // bell
    }

    io::stdout().flush().unwrap();
}

fn is_completion_exist(first_cmd: &str, ctx: &BuiltinContext) -> bool {
    if ctx.completion.borrow().get(first_cmd).is_none() {
        return false;
    }
    true
}

fn handle_tab_completion(line: &mut String, first_cmd: &str, ctx: &BuiltinContext) {
    let executable_loc = ctx.completion.borrow().get(first_cmd).unwrap().clone();
    // let executable_loc = String::from("/test/salah");
    let executable_output = std::process::Command::new(&executable_loc)
        .output()
        .expect("Failed to execute command");
    let output_str = String::from_utf8_lossy(&executable_output.stdout)
        .trim_end()
        .to_string();

    line.push_str(&output_str);
    line.push(' ');
    print!("\n$ git commit ");

    io::stdout().flush().unwrap();
}

fn get_files_list(dir: &str) -> std::io::Result<Vec<String>> {
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

fn get_directories_list(dir: &str) -> std::io::Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                names.push(format!("{}/", name).to_string());
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
