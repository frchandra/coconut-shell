use crate::builtins::BuiltinResult;
use crate::context::RuntimeContext;
use crate::redirect::CmdOutput;
use libc::{WEXITSTATUS, WIFEXITED, WNOHANG, waitpid};
use std::{
    collections::BTreeMap,
    io::{self, Write},
};

pub struct JobContext {
    pub job_table: BTreeMap<u32, Job>, // BTreeMap to keep the jobs sorted by ID.
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
    let ids: Vec<&u32> = job_ctx.job_table.keys().collect(); // already in order, no sort() needed thanks to BTreeMap

    let highest = ids.last().copied();
    let second_highest = if ids.len() >= 2 {
        Some(ids[ids.len() - 2])
    } else {
        None
    };

    for id in ids {
        let job = &job_ctx.job_table[id];
        let status = std::str::from_utf8(&job.status).unwrap_or("Unknown").trim(); // last status
        let marker = if Some(id) == highest {
            "+"
        } else if Some(id) == second_highest {
            "-"
        } else {
            " "
        };
        lines.push(format!("[{}]{} {} {}", id, marker, status, job.command));
    }

    clear_finished_jobs(&mut job_ctx.job_table);

    BuiltinResult::Output(CmdOutput::out(lines.join("\n")))
}

fn update_jobs_status_locked(job_table: &mut BTreeMap<u32, Job>) {
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

pub fn print_already_finished_jobs(job_table: &mut BTreeMap<u32, Job>) {
    update_jobs_status_locked(job_table);

    let ids: Vec<&u32> = job_table.keys().collect();
    let highest = ids.last().copied();

    let second_highest = if ids.len() >= 2 {
        Some(ids[ids.len() - 2])
    } else {
        None
    };

    for id in ids {
        let job = &job_table[id];
        if job.pid.is_some() {
            continue; // still running, skipid
        }
        let status = std::str::from_utf8(&job.status).unwrap_or("Unknown").trim();
        let marker = if Some(id) == highest {
            "+"
        } else if Some(id) == second_highest {
            "-"
        } else {
            " "
        };
        println!("[{}]{} {} {}", id, marker, status, job.command);
        // io::stdout().flush().unwrap();
    }

    clear_finished_jobs(job_table);
}

pub fn clear_finished_jobs(job_table: &mut BTreeMap<u32, Job>) {
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
            job_table: BTreeMap::new(),
            recent_job_id: 0,
        }
    }
}
