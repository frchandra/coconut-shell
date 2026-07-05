use crate::builtins;
use crate::utils::get_path_executables_deduped;
use libc::printf;
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::sync::OnceLock;

static ORIGINAL_TERMIOS: OnceLock<libc::termios> = OnceLock::new();

pub fn read_line() -> String {
    let mut trie = Trie::new();

    let registry = builtins::BuiltinRegistry::new();
    let builtins = registry.names();
    for b in &builtins {
        trie.insert(b);
    }

    for exe in get_path_executables_deduped() {
        trie.insert(&exe);
    }

    set_raw_mode(true).expect("TODO: panic message");

    print!("$ ");
    io::stdout().flush().unwrap();

    let mut line = String::new();
    let stdin = io::stdin();
    let mut byte = [0u8; 1];
    let mut first_tab = false;

    loop {
        stdin.lock().read_exact(&mut byte).unwrap();

        match byte[0] {
            b'\t' => {
                let longest = trie.get_longest_common_prefix(&line);
                let mut prediction = trie.autocomplete(&line);
                if prediction.len() == 1 {
                    line = prediction[0].to_string() + " ";
                    print!("\r$ {}", line);
                } else if longest.is_some() {
                    line = line + longest.unwrap().as_str();
                    print!("\r$ {}", line);
                } else if prediction.len() > 1 && first_tab == true {
                    prediction.sort();
                    io::stdout().flush().unwrap();
                    print!("\n{}", prediction.join(" "));
                    print!("\n$ {}", line);
                } else {
                    print!("\x07");
                }
                first_tab = true;

                io::stdout().flush().unwrap();
            }
            b'\r' | b'\n' => {
                set_raw_mode(false).expect("TODO: panic message");
                print!("\n");
                io::stdout().flush().unwrap();
                return line;
            }
            127 | 8 => {
                if line.pop().is_some() {
                    print!("\x08 \x08");
                    io::stdout().flush().unwrap();
                } else {
                    io::stdout().flush().unwrap();
                    print!("\x07");
                }
            }
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

#[cfg(unix)]
fn set_raw_mode(enable: bool) -> io::Result<()> {
    let fd = io::stdin().as_raw_fd();

    if enable {
        unsafe {
            let mut termios: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(fd, &mut termios) != 0 {
                return Err(io::Error::last_os_error());
            }
            // Save original settings once
            ORIGINAL_TERMIOS.get_or_init(|| termios);

            let mut raw = termios;
            raw.c_lflag &= !(libc::ICANON | libc::ECHO);
            raw.c_cc[libc::VMIN] = 1;
            raw.c_cc[libc::VTIME] = 0;

            if libc::tcsetattr(fd, libc::TCSAFLUSH, &raw) != 0 {
                return Err(io::Error::last_os_error());
            }
        }
    } else if let Some(original) = ORIGINAL_TERMIOS.get() {
        unsafe {
            if libc::tcsetattr(fd, libc::TCSAFLUSH, original) != 0 {
                return Err(io::Error::last_os_error());
            }
        }
    }

    Ok(())
}

#[derive(Default)]
struct TrieNode {
    children: HashMap<char, TrieNode>,
    is_end: bool,
}

struct Trie {
    root: TrieNode,
}

impl Trie {
    pub fn new() -> Self {
        Trie {
            root: TrieNode::default(),
        }
    }

    /// Insert a word into the trie — O(L)
    pub fn insert(&mut self, word: &str) {
        let mut node = &mut self.root;
        for ch in word.chars() {
            node = node.children.entry(ch).or_default();
        }
        node.is_end = true;
    }

    /// Check if an exact word exists — O(L)
    // pub fn contains(&self, word: &str) -> bool {
    //     self.find_node(word).map(|n| n.is_end).unwrap_or(false)
    // }

    /// Check if any word starts with prefix — O(P)
    // pub fn starts_with(&self, prefix: &str) -> bool {
    //     self.find_node(prefix).is_some()
    // }

    /// Return all words with the given prefix — O(P + N·L)
    pub fn autocomplete(&self, prefix: &str) -> Vec<String> {
        match self.find_node(prefix) {
            None => vec![],
            Some(node) => {
                let mut results = Vec::new();
                Self::collect(node, prefix.to_string(), &mut results);
                results
            }
        }
    }

    /// Walk the trie following `prefix`, return the node at the end
    fn find_node(&self, prefix: &str) -> Option<&TrieNode> {
        let mut node = &self.root;
        for ch in prefix.chars() {
            node = node.children.get(&ch)?; // ? short-circuits on missing char
        }
        Some(node)
    }

    pub fn get_longest_common_prefix(&self, prefix: &str) -> Option<String> {
        let mut node = self.find_node(prefix)?;
        let mut next_common = String::new();
        while (node.children.len() == 1 && !node.is_end) {
            if let Some((key, value)) = node.children.iter().next() {
                next_common.push(*key);
                node = value;
            }
        }
        if next_common.is_empty() {
            return None;
        }
        return Some(next_common);
    }

    /// DFS from `node`, accumulating complete words into `results`
    fn collect(node: &TrieNode, current: String, results: &mut Vec<String>) {
        if node.is_end {
            results.push(current.clone());
        }
        for (ch, child) in &node.children {
            let mut next = current.clone();
            next.push(*ch);
            Self::collect(child, next, results);
        }
    }
}
