use std::process::{Command, Stdio};

use crate::builtins::{BuiltinRegistry, BuiltinResult};
use crate::context::RuntimeContext;
use crate::parser::{Pipeline, SimpleCommand};
use crate::redirect::{CmdOutput, apply_redirects};
use crate::utils;

/// Run a [`Pipeline`].
///
/// Currently only single-command pipelines are supported. When pipeline
/// support is added, this function will wire up `pipe()` between
/// successive [`SimpleCommand`]s.
///
/// Returns `true` if the shell should continue, `false` to exit.
pub fn execute(pipeline: &Pipeline, registry: &BuiltinRegistry, ctx: &RuntimeContext) -> bool {
    // For now, handle only the first command. because pipeline || has not been implemented yet.
    let cmd = match pipeline.commands.first() {
        Some(c) => c,
        None => return true,
    };

    // temporary refactorable flow
    let second_cmd = pipeline.commands.get(1);
    if second_cmd.is_some() {
        return execute_with_pipe(cmd, second_cmd.unwrap(), registry, ctx);
    }

    // Try builtin first, then fall back to external.
    let (output, should_continue) = match (registry.get(&cmd.program), cmd.is_background) {
        (Some(func), true) => {
            let args = cmd.args.clone();
            let cloned_ctx = ctx.clone();
            let redirects = cmd.redirects.clone();
            let out = crate::builtins::run_background_builtin(func, args, cloned_ctx, redirects);
            (out, true)
        }
        (Some(func), false) => match func(&cmd.args, ctx) {
            BuiltinResult::Exit => return false,
            BuiltinResult::Output(out) => (out, true),
        },
        (None, true) => match run_external_background(&cmd.program, &cmd.args) {
            Ok(pid) => {
                let command = format!("{} {}", cmd.program, cmd.args.join(" "));

                let job_id = {
                    let mut jobs = ctx.jobs.lock().unwrap(); // lock ONCE
                    jobs.add_job(command, pid)
                }; // guard dropped here, lock released after everything's done

                println!("[{}] {}", job_id, pid);
                (CmdOutput::empty(), true)
            }
            Err(err) => {
                eprintln!("{}", err);
                (CmdOutput::err(err), true)
            }
        },
        (None, false) => (run_external(&cmd.program, &cmd.args), true),
    };
    apply_redirects(&output, &cmd.redirects);
    should_continue
}

// just a temporary refacotable function
pub fn execute_with_pipe(
    cmd: &SimpleCommand,
    second_cmd: &SimpleCommand,
    registry: &BuiltinRegistry,
    ctx: &RuntimeContext,
) -> bool {
    // lets assume that there is no built in commands for now.
    // lets assume that there is no background commands for now.

    let mut first_proc = Command::new(&cmd.program)
        .args(&cmd.args)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");

    // let mut first_proc = first_proc; // need `mut` to call .wait()
    let first_proc_stdout = first_proc.stdout.take().expect("Failed to get stdout");

    let second_proc = Command::new(&second_cmd.program)
        .args(&second_cmd.args)
        .stdin(Stdio::from(first_proc_stdout))
        .spawn()
        .expect("Failed to execute command");

    let output = second_proc.wait_with_output().unwrap();
    print!("{}", String::from_utf8_lossy(&output.stdout).trim());

    // don't forget this:
    first_proc.wait().expect("failed to wait on first_proc");

    true
}

/// Spawn an external process and capture its output.
fn run_external(command: &str, args: &[String]) -> CmdOutput {
    let path = match utils::find_executable_in_path(command) {
        Some(p) => p,
        None => {
            return CmdOutput::err(format!("{command}: command not found"));
        }
    };

    let output = std::process::Command::new(path.file_name().unwrap())
        .args(args)
        .output()
        .expect("Failed to execute command");

    CmdOutput {
        stdout: if output.stdout.is_empty() {
            None
        } else {
            Some(
                String::from_utf8_lossy(&output.stdout)
                    .trim_end()
                    .to_string(),
            )
        },
        stderr: if output.stderr.is_empty() {
            None
        } else {
            Some(
                String::from_utf8_lossy(&output.stderr)
                    .trim_end()
                    .to_string(),
            )
        },
    }
}

fn run_external_background(command: &str, args: &[String]) -> Result<u32, String> {
    let path = match utils::find_executable_in_path(command) {
        Some(p) => p,
        None => {
            return Err(format!("{command}: command not found"));
        }
    };

    let child = std::process::Command::new(path.file_name().unwrap())
        .args(args)
        .spawn()
        .map_err(|e| format!("{command}: failed to start ({e})"))?;

    Ok(child.id())
}
