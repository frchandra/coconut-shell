use std::fs;
use std::io::{self, Read, Write};

use crate::builtins;
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
pub fn read_line() -> String {
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

                if prev_words.is_empty() {
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
fn handle_tab_files(
    line: &mut String,
    prev_words: &[String],
    prefix: &str,
    tab_count: u32,
) {
    // TODO: When prefix contains '/', split into (directory, partial_name)
    // and list entries from that directory instead of ".".
    // e.g. "src/tok" → dir = "src", partial = "tok"
    // This will also need to reconstruct the completed path with the
    // directory prefix when writing back to `line`.
    let files = get_file_names(".").unwrap_or_default();
    let trie = build_custom_trie(files);

    let extension = trie.longest_common_extension(prefix);
    let mut predictions = trie.autocomplete(prefix);

    if predictions.len() == 1 {
        // Single match — rebuild line with the completed filename.
        let mut new_line = prev_words.join(" ");
        new_line.push(' ');
        new_line.push_str(&predictions[0]);
        new_line.push(' ');
        *line = new_line;
        print!("\r$ {}", line);
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
