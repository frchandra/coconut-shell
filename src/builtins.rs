use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::str::FromStr;

use crate::redirect::CmdOutput;
use crate::utils;

// ---------------------------------------------------------------------------
// Builtin infrastructure
// ---------------------------------------------------------------------------

/// Context passed to every builtin invocation.
///
/// Contains read-only information the builtin might need (e.g. the
/// list of registered builtin names for the `type` command).
pub struct BuiltinContext<'a> {
    pub builtin_names: &'a [&'a str],
}

/// The result of running a builtin command.
pub enum BuiltinResult {
    /// Normal output (may contain stdout and/or stderr text).
    Output(CmdOutput),
    /// Signals the shell to exit.
    Exit,
}

/// Signature for all builtin handler functions.
pub type BuiltinFn = fn(args: &[String], ctx: &BuiltinContext) -> BuiltinResult;

/// A registry of shell builtin commands.
///
/// To add a new builtin:
/// 1. Write a function with the [`BuiltinFn`] signature.
/// 2. Call [`BuiltinRegistry::register`] in [`BuiltinRegistry::new`].
pub struct BuiltinRegistry {
    commands: HashMap<String, BuiltinFn>,
}

impl BuiltinRegistry {
    /// Create the registry with all default builtins.
    pub fn new() -> Self {
        let mut reg = Self {
            commands: HashMap::new(),
        };
        reg.register("exit", builtin_exit);
        reg.register("echo", builtin_echo);
        reg.register("type", builtin_type);
        reg.register("pwd", builtin_pwd);
        reg.register("cd", builtin_cd);
        reg
    }

    /// Register a new builtin command.
    pub fn register(&mut self, name: &str, func: BuiltinFn) {
        self.commands.insert(name.to_string(), func);
    }

    /// Look up a builtin by name, returning a copyable function pointer.
    pub fn get(&self, name: &str) -> Option<BuiltinFn> {
        self.commands.get(name).copied()
    }

    /// Check whether `name` is a registered builtin.
    pub fn is_builtin(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    /// Return a sorted list of builtin names.
    pub fn names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.commands.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }
}

// ---------------------------------------------------------------------------
// Builtin implementations
// ---------------------------------------------------------------------------

fn builtin_exit(_args: &[String], _ctx: &BuiltinContext) -> BuiltinResult {
    BuiltinResult::Exit
}

fn builtin_echo(args: &[String], _ctx: &BuiltinContext) -> BuiltinResult {
    BuiltinResult::Output(CmdOutput::out(args.join(" ")))
}

fn builtin_type(args: &[String], ctx: &BuiltinContext) -> BuiltinResult {
    let name = args.join(" ");

    if ctx.builtin_names.contains(&name.as_str()) {
        return BuiltinResult::Output(CmdOutput::out(format!("{name} is a shell builtin")));
    }

    let output = match utils::find_executable_in_path(&name) {
        Some(path) => CmdOutput::out(format!("{} is {}", name, path.display())),
        None => CmdOutput::out(format!("{}: not found", name)),
    };

    BuiltinResult::Output(output)
}

fn builtin_pwd(_args: &[String], _ctx: &BuiltinContext) -> BuiltinResult {
    let cwd = env::current_dir().unwrap().display().to_string();
    BuiltinResult::Output(CmdOutput::out(cwd))
}

fn builtin_cd(args: &[String], _ctx: &BuiltinContext) -> BuiltinResult {
    let raw_path = args.first().map(|s| s.as_str()).unwrap_or("~");

    // Expand tilde *before* checking existence (fixes the old bug where a
    // literal directory named "~" would shadow $HOME).
    let expanded = utils::expand_tilde(raw_path);

    if PathBuf::from_str(&expanded).unwrap().exists() {
        env::set_current_dir(&expanded).unwrap();
    } else {
        return BuiltinResult::Output(CmdOutput::err(format!(
            "cd: {raw_path}: No such file or directory"
        )));
    }

    BuiltinResult::Output(CmdOutput::empty())
}
