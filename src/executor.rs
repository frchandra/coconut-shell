use crate::builtins::{BuiltinRegistry, BuiltinResult};
use crate::context::RuntimeContext;
use crate::parser::Pipeline;
use crate::redirect::{CmdOutput, apply_redirects};
use crate::utils;
use std::{
    io::{self, Write},
};

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
                // todo wrap this on a separate func
                let status = format!("{:<24}", "Running");
                let command = format!("{} {}", cmd.program, cmd.args.join(" "));

                let job_id = {
                    let mut jobs = ctx.jobs.lock().unwrap(); // lock ONCE
                    let job_id = jobs.recent_job_id + 1;
                    jobs.job_table.insert(
                        job_id,
                        crate::jobs::Job {
                            status: status.as_bytes().try_into().unwrap(),
                            command,
                            pid: Some(pid),
                        },
                    );
                    jobs.recent_job_id = job_id;
                    job_id
                }; // guard dropped here, lock released after everything's done

                println!("[{}] {}", job_id, pid);
                io::stdout().flush().unwrap();
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
