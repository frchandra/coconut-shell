use crate::builtins::BuiltinResult;
use crate::context::RuntimeContext;
use crate::redirect::CmdOutput;
use libc::{WEXITSTATUS, WIFEXITED, WNOHANG, waitpid};
use std::collections::HashMap;

pub struct JobContext {
    pub job_table: HashMap<u32, Job>,
    pub recent_job_id: u32,
}
pub struct Job {
    pub status: [u8; 24],
    pub command: String,
    pub pid: Option<u32>, // Store the PID of the background process
}

pub fn builtin_jobs(_args: &[String], ctx: &RuntimeContext) -> BuiltinResult {
    let mut job_ctx = ctx.jobs.lock().unwrap();

    if job_ctx.job_table.is_empty() {
        return BuiltinResult::Output(CmdOutput::empty());
    }

    update_jobs_status_locked(&mut job_ctx.job_table);

    let mut lines: Vec<String> = Vec::new();
    let mut ids: Vec<&u32> = job_ctx.job_table.keys().collect();
    ids.sort();

    for id in ids {
        let job = &job_ctx.job_table[id];
        let status = std::str::from_utf8(&job.status).unwrap_or("Unknown").trim(); // last status
        let marker = if *id == job_ctx.recent_job_id {
            "+"
        } else if *id == job_ctx.recent_job_id - 1 {
            "-"
        } else {
            " "
        };
        lines.push(format!("[{}]{} {} {}", id, marker, status, job.command));
    }

    clear_finished_jobs(&mut job_ctx.job_table);

    BuiltinResult::Output(CmdOutput::out(lines.join("\n")))
}

fn update_jobs_status_locked(job_table: &mut HashMap<u32, Job>) {
    for job in job_table.values_mut() {
        if let Some(pid) = job.pid {
            match check_if_job_exited(pid) {
                Some(_exit_code) => {
                    // Process has exited, update the status
                    job.status = format!("{:<24}", "Done").as_bytes().try_into().unwrap();
                    job.pid = None; // Clear the PID since the process has exited
                }
                None => {
                    // Process is still running, keep the status as is
                }
            }
        }
    }
}

pub fn clear_finished_jobs(job_table: &mut HashMap<u32, Job>) {
    job_table.retain(|_, job| job.pid.is_some());
}

pub fn check_if_job_exited(pid: u32) -> Option<i32> {
    let mut status: i32 = 0;
    let result = unsafe { waitpid(pid as i32, &mut status, WNOHANG) };

    if result == pid as i32 {
        // Process has exited, and we've reaped it
        if WIFEXITED(status) {
            Some(WEXITSTATUS(status))
        } else {
            Some(-1) // exited abnormally (signal, etc.)
        }
    } else {
        None // still running (result == 0), or error (result == -1)
    }
}

impl JobContext {
    pub fn new() -> Self {
        Self {
            job_table: HashMap::new(),
            recent_job_id: 0,
        }
    }
}
