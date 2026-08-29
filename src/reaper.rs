// reaper.rs — Background child-process reaper
//
// This module owns the "reaper thread": a dedicated OS thread that continuously
// polls for exited child processes, updates the shared job table, and (optionally)
// sends a notification event so the main shell thread can print "[1]+ Done ..."
// at the next prompt — the same UX you see in bash/zsh.
//
// WHY A SEPARATE THREAD?
//   The main thread blocks on readline waiting for user input. It can't
//   simultaneously call waitpid. A dedicated thread solves this without
//   requiring async/await or signal handlers.
//
// WHY NOT NAMED PIPES (mkfifo)?
//   Named pipes are for IPC between separate OS processes. Here, the reaper
//   and main thread live inside the same process, so they can share memory
//   directly via Arc<Mutex<...>>. Named pipes would add needless complexity.
//
// THE TWO CHANNELS OF COMMUNICATION:
//   1. Arc<Mutex<JobContext>>  — shared state: the reaper WRITES exit status
//                                here; the main thread and `jobs` builtin READ it.
//   2. mpsc::Sender<JobEvent>  — notification: the reaper SENDS an event so
//                                the main thread can print "Done" messages
//                                without the user having to type `jobs`.

use crate::context::RuntimeContext;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// An event emitted by the reaper thread when a background job exits.
///
/// The main thread receives these via an `mpsc::Receiver<JobEvent>` and
/// can print bash-style "[1]+ Done <command>" notifications.
#[derive(Debug)]
pub struct JobEvent {
    pub job_id: u32,
    pub pid: u32,
    pub exit_code: i32,
    pub command: String,
}

/// How long the reaper thread sleeps between polling rounds.
///
/// LEARNING POINT: WNOHANG means waitpid returns immediately even if no child
/// has exited. So we poll in a loop with a short sleep instead of blocking.
/// 100–250 ms is a good balance: low CPU overhead, near-instant notification.
const REAPER_POLL_INTERVAL: Duration = Duration::from_millis(5);

/// Spawn the reaper thread and return the receiving end of the event channel.
///
/// Call this once at startup (in `main.rs`) and store the `Receiver` in
/// `RuntimeContext` or pass it directly to the readline/prompt loop.
///
/// # How it fits together
///
/// ```text
///  main.rs
///    let (rx, _handle) = reaper::spawn(ctx.clone());
///    // store rx somewhere the prompt loop can drain it
/// ```
pub fn spawn(ctx: RuntimeContext) -> (mpsc::Receiver<JobEvent>, std::thread::JoinHandle<()>) {
    // TODO: Create an mpsc channel.
    //   `mpsc::channel()` returns a (Sender, Receiver) pair.
    //   The Sender goes into the thread; the Receiver comes back to the caller.
    // todo!("create the mpsc channel here");
    let (tx, rx) = mpsc::channel::<JobEvent>();

    // TODO: Spawn an OS thread with `std::thread::spawn`.
    //   Move the Sender and the cloned `ctx` into the closure.
    //   The thread should loop forever (or until the shell exits).
    // todo!("spawn the thread, move sender + ctx into it");
    let reaper_handle = thread::spawn(move || reaper_loop(ctx, tx));

    // TODO: Return (receiver, join_handle).
    // todo!("return the receiver so the main thread can drain events")
    (rx, reaper_handle)
}

/// The main body of the reaper loop — runs inside the spawned thread.
///
/// This function never returns under normal operation.
///
/// ALGORITHM:
///   loop {
///     1. Sleep for REAPER_POLL_INTERVAL          ← give CPU back to OS
///     2. Lock the job table                      ← short critical section
///     3. For each job with a live PID:
///          call check_if_job_exited(pid)
///          if Some(exit_code) → update job.status, clear job.pid
///                             → build a JobEvent, collect it
///     4. Unlock the job table                    ← IMPORTANT: release before sending
///     5. Send collected JobEvents over the channel
///   }
///
/// WHY collect events before sending?
///   Sending over a channel could theoretically block (if the receiver is slow).
///   Holding the Mutex while blocking would deadlock the `jobs` builtin.
///   Always unlock first, then send.
fn reaper_loop(ctx: RuntimeContext, tx: mpsc::Sender<JobEvent>) {
    loop {
        // Step 1: sleep
        // TODO: std::thread::sleep(REAPER_POLL_INTERVAL);
        thread::sleep(REAPER_POLL_INTERVAL);

        // Step 2 + 3: poll all live jobs under the lock
        let events: Vec<JobEvent> = {
            // TODO: lock ctx.jobs
            // TODO: iterate job_table.values_mut()
            // TODO: for each job where job.pid.is_some():
            //         call crate::jobs::check_if_job_exited(pid)
            //         if it returned Some(exit_code):
            //           update job.status to "Done"
            //           set job.pid = None
            //           push a JobEvent into the local `events` vec
            let mut job_ctx = ctx.jobs.lock().unwrap();
            let mut events = vec![];
            for (id, job) in job_ctx.job_table.iter_mut() {
                if let Some(pid) = job.pid {
                    match crate::jobs::check_if_job_exited(pid) {
                        Some(exit_code) => {
                            job.status = format!("{:<24}", "Done").as_bytes().try_into().unwrap();
                            events.push(JobEvent {
                                job_id: *id,
                                pid,
                                exit_code,
                                command: job.command.clone(),
                            });
                            job.pid = None;
                        }
                        None => {} // still running
                    }
                }
            }

            // todo!("poll + collect events");
            events
        }; // Mutex guard drops here → lock released

        // Step 4: send events to the main thread
        for event in events {
            // TODO: tx.send(event)
            //   If send() returns Err, the receiver was dropped (shell is
            //   shutting down). Break out of the loop gracefully.
            // todo!("send event, handle Err(SendError) as shutdown signal");
            if tx.send(event).is_err() {
                break;
            }
        }
    }
}
