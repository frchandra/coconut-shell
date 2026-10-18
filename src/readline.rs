use std::io::{self, Read, Write};
use std::{env, fs};

use crate::builtins;
use crate::context::RuntimeContext;
use crate::utils::get_path_executables_deduped;

use crate::terminal;
use crate::tokenizer;
use crate::trie::Trie;

/// Display the shell prompt.
pub fn print_prompt() {
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
///
/// When stdin is not a terminal (e.g. piped input from a tester),
/// raw mode is skipped and input is read line-by-line to avoid
/// double-echo issues caused by `tcsetattr` failing on pipes.
pub fn read_line(ctx: &RuntimeContext) -> String {
    // jobs::print_already_finished_jobs(&mut ctx.jobs.lock().unwrap().job_table);

    let trie = build_completion_trie();

    terminal::set_raw_mode(true).expect("failed to enable raw mode");

    print_prompt();

    let mut line = String::new();
    let stdin = io::stdin();
    let mut byte = [0u8; 1];
    let mut tab_count: u32 = 0;

    let mut cursor: usize = line.len(); // add this above the loop, tracks position in `line`

    loop {
        stdin.lock().read_exact(&mut byte).unwrap();

        match byte[0] {
            0x1B => {
                // possible escape sequence — read the next two bytes
                let mut seq = [0u8; 2];
                if stdin.lock().read_exact(&mut seq).is_err() {
                    continue; // lone ESC, ignore
                }

                if seq[0] == b'[' {
                    match seq[1] {
                        b'C' => {
                            // right arrow
                            if cursor < line.len() {
                                cursor += 1;
                                print!("\x1B[1C");
                                io::stdout().flush().unwrap();
                            }
                        }
                        b'D' => {
                            // left arrow
                            if cursor > 0 {
                                cursor -= 1;
                                print!("\x1B[1D");
                                io::stdout().flush().unwrap();
                            }
                        }
                        b'A' | b'B' => {
                            // up/down — hook into history navigation here if you have it
                        }
                        _ => {}
                    }
                }
                tab_count = 0;
            }
            b'\t' => {
                tab_count += 1;

                let (prev_words, prefix) = get_completion_context(&line);

                if prev_words.is_empty() {
                    handle_tab_executable(&trie, &mut line, &prefix, tab_count);
                } else if is_completion_exist(&prev_words[0], ctx) {
                    handle_tab_completion(&mut line, prefix, &prev_words, tab_count, ctx);
                } else {
                    handle_tab_files(&mut line, &prev_words, &prefix, tab_count);
                }
                cursor = line.len(); // re-sync cursor after completion mutates line
            }
            b'\r' | b'\n' => {
                terminal::set_raw_mode(false).expect("failed to restore terminal");
                println!();
                return line;
            }
            127 | 8 => {
                handle_backspace(&mut line, &mut cursor);
                tab_count = 0;
            }
            c if c >= 32 => {
                let ch = c as char;
                line.insert(cursor, ch);
                cursor += 1;

                // redraw from insertion point to end so mid-line inserts shift correctly
                print!("{}", &line[cursor - 1..]);
                for _ in cursor..line.len() {
                    print!("\x08");
                }
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
        print!("\n{}", predictions.join(" "));
        print!("\n$ {line}");
    } else {
        print!("\x07"); // bell
    }

    io::stdout().flush().unwrap();
}

/// Process a backspace / delete-backward press.
fn handle_backspace(line: &mut String, cursor: &mut usize) {
    if *cursor == 0 {
        print!("\x07"); // bell — nothing to delete
        io::stdout().flush().unwrap();
        return;
    }

    line.remove(*cursor - 1);
    *cursor -= 1;

    // redraw from cursor to end of line, then clear the leftover trailing char,
    // then move the terminal cursor back to the correct spot
    let tail = &line[*cursor..];
    print!("\x08{tail} \x08"); // backspace, reprint tail, blank the old last char, backspace once
    for _ in 0..tail.len() {
        print!("\x08"); // move cursor back to correct position after the redraw
    }

    io::stdout().flush().unwrap();
}

fn handle_tab_completion(
    line: &mut String,
    last_word: String,
    prev_words: &Vec<String>,
    tab_count: u32,
    ctx: &RuntimeContext,
) {
    let executable_loc = ctx.completion.borrow().get(&prev_words[0]).unwrap().clone();

    unsafe {
        env::set_var("COMP_LINE", line.clone());
        env::set_var("COMP_POINT", line.len().to_string());
    }

    let arguments: Vec<String>;
    if prev_words.len() < 2 {
        arguments = vec![
            prev_words[0].clone(),
            last_word.clone(),
            prev_words[0].clone(),
        ];
    } else {
        arguments = vec![
            prev_words[0].clone(),
            last_word.clone(),
            prev_words[1].clone(),
        ];
    }
    let executable_output = std::process::Command::new(&executable_loc)
        .args(arguments) //prev_words[0] prefix prev_words[1]
        .output()
        .expect("Failed to execute command");
    let output_data: Vec<String> = String::from_utf8_lossy(&executable_output.stdout)
        .lines()
        .map(|x| x.to_string())
        .collect();

    if output_data.is_empty() {
        print!("\x07"); // bell
        io::stdout().flush().unwrap();
    } else if output_data.len() > 1 && tab_count < 2 {
        let trie = build_custom_trie(output_data);
        let completion = trie.longest_common_extension(last_word.as_str());
        if completion.is_some() {
            line.push_str(&completion.clone().unwrap());
            print!("{}", &completion.unwrap());
            io::stdout().flush().unwrap();
        } else {
            print!("\x07"); // bell
            io::stdout().flush().unwrap();
        }
    } else if output_data.len() > 1 && tab_count >= 2 {
        println!("\n{}\n$ {}", output_data.join(" ").to_string(), line);
    } else {
        let completion = output_data[0]
            .strip_prefix(last_word.as_str())
            .unwrap_or(output_data[0].as_str());
        print!("{completion} ");
        io::stdout().flush().unwrap();
    }
}

fn is_completion_exist(first_cmd: &str, ctx: &RuntimeContext) -> bool {
    if ctx.completion.borrow().get(first_cmd).is_none() {
        return false;
    }
    true
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

/// Extract the completion context from a partial input line.
///
/// Returns `(words_before_cursor, prefix_being_typed)`.
/// If the cursor is at a fresh word boundary (trailing space), `prefix` is `""`.
///
/// Uses the quote/escape-aware [`tokenizer::split_words`] so that
/// quoted and escaped strings are handled consistently with execution
/// tokenization.
fn get_completion_context(input: &str) -> (Vec<String>, String) {
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
