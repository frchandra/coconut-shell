use crate::parser::Redirect;
use crate::tokenizer::RedirectMode;

/// The output of any command (builtin or external).
///
/// Every command produces a `CmdOutput`. The executor then applies
/// redirections uniformly — this eliminates the old duplication where
/// each builtin had its own redirect handling.
#[derive(Debug, Default)]
pub struct CmdOutput {
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}

impl CmdOutput {
    /// A successful output with text on stdout.
    pub fn out(text: String) -> Self {
        Self {
            stdout: Some(text),
            stderr: None,
        }
    }

    /// An error output with text on stderr.
    pub fn err(text: String) -> Self {
        Self {
            stdout: None,
            stderr: Some(text),
        }
    }

    /// Empty output (no stdout, no stderr).
    pub fn empty() -> Self {
        Self::default()
    }
}

/// Apply the given redirections to a [`CmdOutput`], writing redirected
/// streams to files and printing non-redirected streams normally.
///
/// Supports both `Truncate` (`>`) and `Append` (`>>`) modes.
pub fn apply_redirects(output: &CmdOutput, redirects: &[Redirect]) {
    let mut stdout_redirected = false;
    let mut stderr_redirected = false;

    for redir in redirects {
        let content = match redir.fd {
            1 => {
                stdout_redirected = true;
                output.stdout.as_deref().unwrap_or("")
            }
            2 => {
                stderr_redirected = true;
                output.stderr.as_deref().unwrap_or("")
            }
            _ => continue,
        };

        match redir.mode {
            RedirectMode::Truncate => {
                std::fs::write(&redir.target, content).unwrap();
            }
            RedirectMode::Append => {
                use std::fs::OpenOptions;
                use std::io::Write;
                let mut file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&redir.target)
                    .unwrap();
                write!(file, "{}", content).unwrap();
            }
        }
    }

    // Print anything that wasn't redirected.
    if !stdout_redirected
        && let Some(ref s) = output.stdout
    {
        println!("{}", s);
    }

    if !stderr_redirected
        && let Some(ref e) = output.stderr
    {
        eprintln!("{}", e);
    }
}
