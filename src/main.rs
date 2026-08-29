// ─── WHERE TO WIRE THE REAPER ─────────────────────────────────────────────────
//
// After you implement reaper.rs, make these three changes here:
//
// 1. Add `mod reaper;` at the top with the other mods.
//
// 2. In `main()`, before the readline loop, spawn the reaper:
//
//      let (job_rx, _reaper_handle) = reaper::spawn(ctx.clone());
//
//    The `_reaper_handle` is intentionally unused for now — the thread runs
//    for the lifetime of the process.  If you later want a graceful shutdown,
//    store the handle and join it on exit.
//
// 3. Pass `job_rx` into the readline loop so it can drain pending events
//    and print "[N]+ Done <command>" before each prompt.
//    The simplest approach: add `job_rx` as a parameter to `read_line()`.
//
// LEARNING POINT — why before the loop, not inside it?
//   The reaper runs concurrently with readline. Starting it once means it's
//   always watching, even when the user is mid-typing a long command.
// ──────────────────────────────────────────────────────────────────────────────

use std::io::{self, Write};

use crate::readline::print_prompt;

mod builtins;
mod context;
mod executor;
mod jobs;
mod parser;
mod readline;
mod reaper;
mod redirect;
mod terminal;
mod tokenizer;
mod trie;
mod utils;
// mod reaper;  ← uncomment when reaper.rs is ready

fn main() {
    let registry = builtins::BuiltinRegistry::new();
    let ctx = context::RuntimeContext::new(&registry);

    // TODO (step 2): spawn the reaper thread here
    // let (job_rx, _reaper_handle) = reaper::spawn(ctx.clone());
    let (job_rx, _reaper_handle) = reaper::spawn(ctx.clone());

    print_prompt();
    loop {
        // TODO (step 3): drain job_rx before each prompt and print notifications
        // while let Ok(event) = job_rx.try_recv() {
        //     println!("\r\n[{}]+ Done\t{}", event.job_id, event.command);
        // }

        let result = readline::read_line(&ctx);

        let tokens = tokenizer::tokenize(&result);
        let pipeline = parser::parse(tokens);

        if pipeline.is_empty() {
            continue;
        }

        if !executor::execute(&pipeline, &registry, &ctx) {
            break;
        }
        while let Ok(event) = job_rx.try_recv() {
            println!("[{}]+ Done\t{}", event.job_id, event.command);
        }
        print_prompt();
    }
}
