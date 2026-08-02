/// Redirect mode for file redirections.
#[derive(Debug, Clone, PartialEq)]
pub enum RedirectMode {
    Truncate, // >
    Append,   // >>
}

/// A lexical token produced by the tokenizer.
#[derive(Debug)]
pub enum Token {
    Word(String),
    Pipe,
    Redirect { fd: u32, mode: RedirectMode },
    Ampersand,
}

/// Tokenize a raw input line into a sequence of [`Token`]s.
///
/// Handles single quotes, double quotes (with POSIX-compliant backslash
/// escaping), bare backslash escapes, and classifies redirect operators
/// and pipes.
pub fn tokenize(input: &str) -> Vec<Token> {
    let raw_words = split_words(input);
    raw_words.into_iter().map(classify).collect()
}

/// Classify a raw word string into a typed [`Token`].
fn classify(word: String) -> Token {
    match word.as_str() {
        "|" => Token::Pipe,
        ">" | "1>" => Token::Redirect {
            fd: 1,
            mode: RedirectMode::Truncate,
        },
        "2>" => Token::Redirect {
            fd: 2,
            mode: RedirectMode::Truncate,
        },
        ">>" | "1>>" => Token::Redirect {
            fd: 1,
            mode: RedirectMode::Append,
        },
        "2>>" => Token::Redirect {
            fd: 2,
            mode: RedirectMode::Append,
        },
        "&" => Token::Ampersand,
        _ => Token::Word(word),
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

/// Split raw input into word strings, handling quoting and escaping.
///
/// This does *not* classify operators — it only splits on whitespace
/// while respecting quote boundaries and backslash escapes.
pub fn split_words(input: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut chars = input.trim_end().chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            // Bare backslash: escape the next character literally.
            '\\' => {
                if let Some(&next_ch) = chars.peek() {
                    current.push(next_ch);
                    chars.next();
                }
            }

            // Single-quoted string: everything is literal until closing quote.
            '\'' => {
                for ch in chars.by_ref() {
                    if ch == '\'' {
                        break;
                    }
                    current.push(ch);
                }
            }

            // Double-quoted string: backslash only escapes \, $, ", `, and
            // newline (POSIX). For all other characters the backslash is
            // preserved literally. (Fixes former bug where *every* character
            // after a backslash was consumed.)
            '"' => {
                while let Some(ch) = chars.next() {
                    if ch == '\\'
                        && let Some(&next_ch) = chars.peek()
                    {
                        match next_ch {
                            '\\' | '$' | '"' | '`' | '\n' => {
                                chars.next();
                                current.push(next_ch);
                            }
                            _ => {
                                // Not a POSIX-escapable char — keep the backslash.
                                current.push('\\');
                            }
                        }
                    } else if ch == '"' {
                        break;
                    } else {
                        current.push(ch);
                    }
                }
            }

            // Whitespace: flush the current word.
            ' ' => {
                if !current.is_empty() {
                    words.push(current.clone());
                    current.clear();
                }
            }

            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        words.push(current);
    }

    words
}
