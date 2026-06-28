use crate::builtins;
use crate::utils::get_path_executables_deduped;
use std::collections::HashMap;
use std::io::{self, Read, Write};

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

    set_raw_mode(true);

    let mut line = String::new();
    let stdin = io::stdin();
    let mut byte = [0u8; 1];

    loop {
        stdin.lock().read_exact(&mut byte).unwrap();

        match byte[0] {
            b'\t' => {
                if let Some(prediction) = trie.autocomplete(&line).first() {
                    line = prediction.to_string() + " ";
                    print!("\r$ {}", line);
                } else {
                    print!("\x07");
                }
                io::stdout().flush().unwrap();
            }
            b'\r' | b'\n' => {
                set_raw_mode(false);
                print!("\n");
                io::stdout().flush().unwrap();
                return line;
            }
            127 | 8 => {
                if line.pop().is_some() {
                    print!("\x08 \x08");
                    io::stdout().flush().unwrap();
                }
            }
            c if c >= 32 => {
                let ch = c as char;
                line.push(ch);
                print!("{ch}");
                io::stdout().flush().unwrap();
            }
            _ => {}
        }
    }
}

#[cfg(unix)]
fn set_raw_mode(enable: bool) {
    use std::os::fd::AsRawFd;
    unsafe {
        let fd = io::stdin().as_raw_fd();
        let mut termios: libc::termios = std::mem::zeroed();
        libc::tcgetattr(fd, &mut termios);
        if enable {
            termios.c_lflag &= !(libc::ICANON | libc::ECHO);
            termios.c_cc[libc::VMIN] = 1;
            termios.c_cc[libc::VTIME] = 0;
        } else {
            termios.c_lflag |= libc::ICANON | libc::ECHO;
        }
        libc::tcsetattr(fd, libc::TCSANOW, &termios);
    }
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
    pub fn contains(&self, word: &str) -> bool {
        self.find_node(word).map(|n| n.is_end).unwrap_or(false)
    }

    /// Check if any word starts with prefix — O(P)
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.find_node(prefix).is_some()
    }

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
