use crate::builtins::{BuiltinContext, BuiltinRegistry, BuiltinResult};
use crate::parser::Pipeline;
use crate::redirect::{apply_redirects, CmdOutput};
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

    if cmd.is_background{

        return true;
    }



    // Try builtin first, then fall back to external.
    let (output, should_continue) = if let Some(func) = registry.get(&cmd.program) {
        match func(&cmd.args, ctx) {
            BuiltinResult::Exit => return false,
            BuiltinResult::Output(out) => (out, true),
        }
    } else {
            (run_external(&cmd.program, &cmd.args), true)
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
