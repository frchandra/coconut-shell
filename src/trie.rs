use std::collections::HashMap;

/// A single node within the [`Trie`].
#[derive(Default)]
struct TrieNode {
    children: HashMap<char, TrieNode>,
    is_end: bool,
}

/// A prefix-tree that supports fast autocompletion and
/// longest-common-prefix queries.
pub struct Trie {
    root: TrieNode,
}

impl Trie {
    /// Create an empty trie.
    pub fn new() -> Self {
        Self {
            root: TrieNode::default(),
        }
    }

    /// Insert a word into the trie — O(L).
    pub fn insert(&mut self, word: &str) {
        let mut node = &mut self.root;
        for ch in word.chars() {
            node = node.children.entry(ch).or_default();
        }
        node.is_end = true;
    }

    // the next 2 function is actually can be combined for simplifying the computational flow.
    // but considering single responsibility principle and predictability it left as it is

    /// Return all words that share the given `prefix` — O(P + N·L).
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

    /// Return the longest unambiguous suffix that extends `prefix`.
    ///
    /// For example, if the trie contains `["cargo", "cat"]` and `prefix`
    /// is `"ca"`, this returns `None` because the next character is
    /// ambiguous.  If the trie only contains `"cargo"`, this returns
    /// `Some("rgo")`.
    pub fn longest_common_extension(&self, prefix: &str) -> Option<String> {
        let mut node = self.find_node(prefix)?;
        let mut extension = String::new();

        while node.children.len() == 1 && !node.is_end {
            if let Some((key, child)) = node.children.iter().next() {
                extension.push(*key);
                node = child;
            }
        }

        if extension.is_empty() {
            None
        } else {
            Some(extension)
        }
    }

    // ------------------------------------------------------------------
    // Private helpers
    // ------------------------------------------------------------------

    /// Walk the trie following `prefix`; return the node at the end.
    fn find_node(&self, prefix: &str) -> Option<&TrieNode> {
        let mut node = &self.root;
        for ch in prefix.chars() {
            node = node.children.get(&ch)?;
        }
        Some(node)
    }

    /// DFS from `node`, accumulating complete words into `results`.
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
