use crate::builtins::{BuiltinContext, BuiltinRegistry, BuiltinResult};
use crate::parser::Pipeline;
use crate::redirect::{CmdOutput, apply_redirects};
use crate::utils;

/// Run a [`Pipeline`].
///
/// Currently only single-command pipelines are supported. When pipeline
/// support is added, this function will wire up `pipe()` between
/// successive [`SimpleCommand`]s.
///
/// Returns `true` if the shell should continue, `false` to exit.
pub fn execute(pipeline: &Pipeline, registry: &BuiltinRegistry, ctx: &BuiltinContext) -> bool {
    // For now, handle only the first command. because pipeline || has not been implemented yet.
    let cmd = match pipeline.commands.first() {
        Some(c) => c,
        None => return true,
    };

    // Try builtin first, then fall back to external.
    let (output, should_continue) = match (registry.get(&cmd.program), cmd.is_background) {
        (Some(func), true) => {
            let args = cmd.args.clone();
            let ctx = ctx.clone();
            let redirects = cmd.redirects.clone();
            let out = crate::builtins::run_background_builtin(func, args, ctx, redirects);
            (out, true)
        }
        (Some(func), false) => match func(&cmd.args, ctx) {
            BuiltinResult::Exit => return false,
            BuiltinResult::Output(out) => (out, true),
        },
        (None, true) => match run_external_background(&cmd.program, &cmd.args) {
            Ok(pid) => {
                println!("[1] {}", pid);
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
