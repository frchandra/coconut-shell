use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fmt::format;
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
///
/// This struct is constructed once from a [`BuiltinRegistry`] and
/// shared for the entire lifetime of the shell.
#[derive(Clone)]
pub struct BuiltinContext {
    pub builtin_names: Vec<String>,
    /// Mutable at runtime via interior mutability (`RefCell`).
    /// Use `.borrow()` to read and `.borrow_mut()` to write.
    pub completion: RefCell<HashMap<String, String>>,
}

impl BuiltinContext {
    /// Build a context from the current state of a [`BuiltinRegistry`].
    pub fn from_registry(registry: &BuiltinRegistry) -> Self {
        Self {
            builtin_names: registry.names().into_iter().map(|s| s.to_owned()).collect(),
            completion: RefCell::new(HashMap::new()),
        }
    }
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
        reg.register("complete", builtin_complete);
        reg.register("jobs", builtin_jobs);
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

    if ctx.builtin_names.iter().any(|n| n == &name) {
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

fn builtin_complete(args: &[String], ctx: &BuiltinContext) -> BuiltinResult {
    if args[0] == "-p" {
        // let value = ctx.completion.borrow().get().unwrap_or(&"".to_string()).clone();
        if let Some(value) = ctx.completion.borrow().get(args[1].as_str()) {
            return BuiltinResult::Output(CmdOutput::out(format!(
                "complete -C '{}' {}",
                value, args[1]
            )));
        } else {
            return BuiltinResult::Output(CmdOutput::err(format!(
                "complete: {}: no completion specification",
                args[1]
            )));
        }
    } else if args[0] == "-C" {
        ctx.completion
            .borrow_mut()
            .insert(args[2].clone(), args[1].clone());
    } else if args[0] == "-r" {
        ctx.completion.borrow_mut().remove(args[1].as_str());
    }
    BuiltinResult::Output(CmdOutput::empty())
}

fn builtin_jobs(args: &[String], ctx: &BuiltinContext) -> BuiltinResult {
    BuiltinResult::Output(CmdOutput::empty())
}

pub fn run_background_builtin(
    func: BuiltinFn,
    args: Vec<String>,
    ctx: BuiltinContext,
    redirects: Vec<crate::parser::Redirect>,
) -> CmdOutput {
    let handle = std::thread::spawn(move || {
        if let BuiltinResult::Output(out) = func(&args, &ctx) {
            crate::redirect::apply_redirects(&out, &redirects);
        }
    });
    let tid_str = format!("{:?}", handle.thread().id());
    let pid = tid_str.replace("ThreadId(", "").replace(")", "");
    println!("[1] {}", pid);
    CmdOutput::empty()
}
