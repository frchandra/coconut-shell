use std::fs::File;
use std::io;
use std::io::Write;
use std::os::unix::io::FromRawFd;
use std::process::{Command, Stdio};

use crate::builtins::{BuiltinFn, BuiltinRegistry, BuiltinResult};
use crate::context::RuntimeContext;
use crate::parser::{Pipeline, UnitCommand};
use crate::redirect::apply_redirects;
use crate::utils;

enum ResolvedCommand {
    Builtin(BuiltinFn),
    External(std::ffi::OsString),
}

fn resolve(program: &str, registry: &BuiltinRegistry) -> Option<ResolvedCommand> {
    if let Some(func) = registry.get(program) {
        Some(ResolvedCommand::Builtin(func))
    } else if let Some(path) = utils::find_executable_in_path(program) {
        Some(ResolvedCommand::External(
            path.file_name().unwrap().to_os_string(),
        ))
    } else {
        None
    }
}

enum SpawnedProcess {
    Child(std::process::Child),
    Thread(Option<std::thread::JoinHandle<()>>),
}

impl SpawnedProcess {
    fn wait(&mut self) {
        match self {
            SpawnedProcess::Child(c) => {
                let _ = c.wait();
            }
            SpawnedProcess::Thread(opt) => {
                if let Some(t) = opt.take() {
                    let _ = t.join();
                }
            }
        }
    }
}

fn get_cmd_stdio(
    cmd: &UnitCommand,
    default_pipe_write: Option<File>,
) -> (Option<File>, Option<File>) {
    let mut out = default_pipe_write;
    let mut err = None;
    for redir in &cmd.redirects {
        let file = match redir.mode {
            crate::tokenizer::RedirectMode::Truncate => std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&redir.target),
            crate::tokenizer::RedirectMode::Append => std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .append(true)
                .open(&redir.target),
        };
        if let Ok(f) = file {
            if redir.fd == 1 {
                out = Some(f);
            } else if redir.fd == 2 {
                err = Some(f);
            }
        }
    }
    (out, err)
}

/// Run a [`Pipeline`].
///
/// Returns `true` if the shell should continue, `false` to exit.
pub fn execute(pipeline: &Pipeline, registry: &BuiltinRegistry, ctx: &RuntimeContext) -> bool {
    let commands = &pipeline.commands;
    if commands.is_empty() {
        return true;
    }

    let is_background = commands.last().unwrap().is_background;
    let n = commands.len();

    // ── Step 1: Resolve all commands ──
    let resolved: Vec<Option<ResolvedCommand>> = commands
        .iter()
        .map(|cmd| resolve(&cmd.program, registry))
        .collect();

    // ── Step 2: Special case — single foreground builtin ──
    if n == 1 && !is_background {
        if let Some(ResolvedCommand::Builtin(func)) = &resolved[0] {
            match func(&commands[0].args, ctx) {
                BuiltinResult::Exit => return false,
                BuiltinResult::Output(out) => {
                    apply_redirects(&out, &commands[0].redirects);
                    return true;
                }
            }
        }
    }

    // ── Step 3 & 4: Plumb pipes and spawn ──
    let mut prev_read_fd: Option<File> = None;
    let mut spawned: Vec<SpawnedProcess> = Vec::new();

    for i in 0..n {
        let cmd = &commands[i];
        let is_last = i == n - 1;

        let mut read_fd = None;
        let mut write_fd = None;

        if !is_last {
            let mut fds = [0i32; 2];
            unsafe {
                libc::pipe(fds.as_mut_ptr());
            }
            read_fd = Some(unsafe { File::from_raw_fd(fds[0]) });
            write_fd = Some(unsafe { File::from_raw_fd(fds[1]) });
        }

        let (stdout_file, stderr_file) = get_cmd_stdio(cmd, write_fd);
        let stdin_file = prev_read_fd.take();

        match &resolved[i] {
            Some(ResolvedCommand::External(path)) => {
                let stdin = stdin_file.map(Stdio::from).unwrap_or_else(Stdio::inherit);
                let stdout = stdout_file.map(Stdio::from).unwrap_or_else(Stdio::inherit);
                let stderr = stderr_file.map(Stdio::from).unwrap_or_else(Stdio::inherit);

                let child = Command::new(path)
                    .args(&cmd.args)
                    .stdin(stdin)
                    .stdout(stdout)
                    .stderr(stderr)
                    .spawn();

                match child {
                    Ok(c) => spawned.push(SpawnedProcess::Child(c)),
                    Err(e) => eprintln!("{}: {}", cmd.program, e),
                }
            }
            Some(ResolvedCommand::Builtin(func)) => {
                let func = *func;
                let args = cmd.args.clone();
                let ctx = ctx.clone();
                let mut out_file = stdout_file;
                let mut err_file = stderr_file;

                let handle = std::thread::spawn(move || {
                    if let BuiltinResult::Output(out) = func(&args, &ctx) {
                        if let Some(text) = out.stdout {
                            if let Some(f) = out_file.as_mut() {
                                use std::io::Write;
                                writeln!(f, "{}", text).unwrap();
                                // let _ = f.write_all(text.as_bytes());
                                // if !text.ends_with('\n') {
                                // let _ = f.write_all(b"\n");
                                // }
                            } else {
                                println!("{}", text);
                            }
                        }
                        if let Some(text) = out.stderr {
                            if let Some(f) = err_file.as_mut() {
                                use std::io::Write;
                                writeln!(f, "{}", text).unwrap();
                                // let _ = f.write_all(text.as_bytes());
                                // if !text.ends_with('\n') {
                                // let _ = f.write_all(b"\n");
                                // }
                            } else {
                                eprintln!("{}", text);
                            }
                        }
                    }
                });
                spawned.push(SpawnedProcess::Thread(Some(handle)));
            }
            None => {
                eprintln!("{}: command not found", cmd.program);
            }
        }

        prev_read_fd = read_fd;
    }

    // ── Step 5: Wait or Background ──
    if is_background {
        let mut last_pid = None;
        for p in &spawned {
            if let SpawnedProcess::Child(c) = p {
                last_pid = Some(c.id());
            }
        }

        let pid = last_pid.unwrap_or(0);
        let mut command = pipeline.to_string();
        if let Some(stripped) = command.strip_suffix('&') {
            command = stripped.trim_end().to_string();
        }

        let job_id = {
            let mut jobs = ctx.jobs.lock().unwrap();
            jobs.add_job(command, pid)
        };
        println!("[{}] {}", job_id, pid);
    } else {
        for mut proc in spawned {
            proc.wait();
        }
    }

    true
}
