use crate::builtins::BuiltinResult;
use crate::context::RuntimeContext;
use crate::redirect::CmdOutput;
use std::collections::HashMap;

pub struct JobContext {
    pub job: HashMap<u32, Job>,
    pub recent_job_id: u32,
}
pub struct Job {
    pub status: [u8; 24],
    pub command: String,
}

pub fn builtin_jobs(_args: &[String], ctx: &RuntimeContext) -> BuiltinResult {
    let jobs = ctx.jobs.lock().unwrap();

    if jobs.job.is_empty() {
        return BuiltinResult::Output(CmdOutput::empty());
    }

    let mut lines: Vec<String> = Vec::new();
    let mut ids: Vec<&u32> = jobs.job.keys().collect();
    ids.sort();

    for id in ids {
        let job = &jobs.job[id];
        let status = std::str::from_utf8(&job.status).unwrap_or("Unknown").trim();
        let marker = if *id == jobs.recent_job_id {
            "+"
        } else if *id == jobs.recent_job_id - 1 {
            "-"
        } else {
            " "
        };
        lines.push(format!("[{}]{} {} {}", id, marker, status, job.command));
    }

    BuiltinResult::Output(CmdOutput::out(lines.join("\n")))
}

impl JobContext {
    pub fn new() -> Self {
        Self {
            job: HashMap::new(),
            recent_job_id: 0,
        }
    }
}
