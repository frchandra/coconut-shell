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

    loop {
        stdin.lock().read_exact(&mut byte).unwrap();

        match byte[0] {
            b'\t' => handle_tab(&trie, &mut line, &mut first_tab),
            b'\r' | b'\n' => {
                terminal::set_raw_mode(false).expect("failed to restore terminal");
                println!();
                return line;
            }
            127 | 8 => handle_backspace(&mut line),
            c if c >= 32 => {
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
fn handle_tab(trie: &Trie, line: &mut String, first_tab: &mut bool) {
    let extension = trie.longest_common_extension(line);
    let mut predictions = trie.autocomplete(line);

    if predictions.len() == 1 {
        *line = predictions.remove(0) + " ";
        print!("\r$ {line}");
    } else if let Some(ext) = extension {
        line.push_str(&ext);
        print!("\r$ {line}");
    } else if predictions.len() > 1 && *first_tab {
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

/// Process a backspace / delete-backward press.
fn handle_backspace(line: &mut String) {
    if line.pop().is_some() {
        print!("\x08 \x08");
    } else {
        print!("\x07"); // bell
    }
    io::stdout().flush().unwrap();
}
