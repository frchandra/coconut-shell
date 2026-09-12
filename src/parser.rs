use crate::tokenizer::{RedirectMode, Token};

/// A single I/O redirection (e.g. `2> errors.log`).
#[derive(Debug, Clone)]
pub struct Redirect {
    pub fd: u32,
    pub mode: RedirectMode,
    pub target: String,
}

impl std::fmt::Display for Redirect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let op = match self.mode {
            RedirectMode::Truncate => ">",
            RedirectMode::Append => ">>",
        };
        if self.fd == 1 {
            write!(f, "{} {}", op, self.target)
        } else {
            write!(f, "{}{} {}", self.fd, op, self.target)
        }
    }
}

/// A simple command: a program name, arguments, and zero or more redirections.
#[derive(Debug, Clone)]
pub struct UnitCommand {
    pub program: String,
    pub args: Vec<String>,
    pub redirects: Vec<Redirect>,
    pub is_background: bool,
}

impl UnitCommand {
    pub fn to_string(&self) -> String {
        let mut parts = Vec::new();
        parts.push(self.program.clone());
        parts.extend(self.args.clone());
        for redirect in &self.redirects {
            parts.push(redirect.to_string());
        }
        if self.is_background {
            parts.push("&".to_string());
        }
        parts.join(" ")
    }
}

impl std::fmt::Display for UnitCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// A pipeline of one or more [`UnitCommand`]s connected by pipes.
///
/// Currently only single-command pipelines are executed, but the
/// structure is ready for multi-command pipes in the future.
#[derive(Debug)]
pub struct Pipeline {
    pub commands: Vec<UnitCommand>,
}

impl Pipeline {
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn to_string(&self) -> String {
        self.commands
            .iter()
            .map(|cmd| cmd.to_string())
            .collect::<Vec<_>>()
            .join(" | ")
    }
}

impl std::fmt::Display for Pipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Parse a sequence of tokens into a [`Pipeline`].
///
/// Tokens are consumed left-to-right. `Word` tokens fill the current
/// command's program/args. `Redirect` tokens consume the next `Word`
/// as a filename. `Pipe` tokens finalize the current command and start
/// a new one.
pub fn parse(tokens: Vec<Token>) -> Pipeline {
    let mut commands: Vec<UnitCommand> = Vec::new();
    let mut program: Option<String> = None;
    let mut args: Vec<String> = Vec::new();
    let mut redirects: Vec<Redirect> = Vec::new();

    let mut iter = tokens.into_iter().peekable();

    while let Some(token) = iter.next() {
        match token {
            Token::Word(w) => {
                if program.is_none() {
                    program = Some(w);
                } else {
                    args.push(w);
                }
            }

            Token::Redirect { fd, mode } => {
                // The next Word token is the redirect target filename.
                if let Some(Token::Word(target)) = iter.next() {
                    redirects.push(Redirect { fd, mode, target });
                }
                // If there is no following Word, the redirect is silently
                // ignored (matches current behaviour).
            }

            Token::Pipe => {
                // Finalize current command and start a fresh one.
                if let Some(prog) = program.take() {
                    commands.push(UnitCommand {
                        program: prog,
                        args: std::mem::take(&mut args),
                        redirects: std::mem::take(&mut redirects),
                        is_background: true,
                    });
                }
            }

            // this can be further chained latter
            Token::Ampersand => {
                // Run current command in the background and start a fresh one.
                if let Some(prog) = program.take() {
                    commands.push(UnitCommand {
                        program: prog, // implicit std::mem::take had already handled by Option<>
                        args: std::mem::take(&mut args), // we use std::mem::take because we push an outside loop variable multiple times, so we need to 'clear' it after each push
                        redirects: std::mem::take(&mut redirects),
                        is_background: true,
                    });
                }
            }
        }
    }

    // Push the last (or only) command.
    if let Some(prog) = program {
        commands.push(UnitCommand {
            program: prog,
            args,
            redirects,
            is_background: false,
        });
    }

    Pipeline { commands }
}
