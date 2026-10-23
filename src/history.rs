use crate::builtins::BuiltinResult;
use crate::context::RuntimeContext;
use crate::redirect::CmdOutput;
use std::fs::File;
use std::io::{BufRead, BufReader};
use crate::utils::expand_tilde;

pub fn builtin_history(args: &[String], ctx: &RuntimeContext) -> BuiltinResult {
    //check if args[0] exist
    if args.get(0) == Some(&"-r".into()) {
        if let Some(file_path) = args.get(1) {
            load_history_from_file(file_path, ctx);
            return BuiltinResult::Output(CmdOutput::empty())
        } else {
            return BuiltinResult::Output(CmdOutput::err(
                "history: missing file path after -r".to_string(),
            ));
        }
    } else if let Some(limit) = args.get(0) {
        if let Ok(limit) = limit.parse::<usize>() {
            let history = ctx.history.borrow();
            let start = history.len() - limit;
            let mut output = String::new();
            for (index, entry) in history[start..].iter().enumerate() {
                output += &format!("{} {}\n", index + start, entry);
            }
            output = output.trim_end_matches('\n').to_string();

            return BuiltinResult::Output(CmdOutput::out(output));
        } else {
            return BuiltinResult::Output(CmdOutput::err(format!(
                "history: invalid number: {}",
                limit
            )));
        }
    } else {
        let history = ctx.history.borrow();
        let mut output = String::new();
        for (index, entry) in history.iter().enumerate() {
            output += &format!("{} {}\n", index + 1, entry);
        }
        output = output.trim_end_matches('\n').to_string();
        BuiltinResult::Output(CmdOutput::out(output))
    }
}

pub fn append_history(command: &String, ctx: &RuntimeContext) {
    ctx.history.borrow_mut().push(command.clone());
}

pub fn load_history_from_file(path: &String, ctx: &RuntimeContext) {
    // Expand a leading '~' and try to open the file. Silently return on errors.
    let expanded = expand_tilde(path);
    let file = match File::open(expanded) {
        Ok(f) => f,
        Err(_) => return,
    };

    let reader = BufReader::new(file);
    let mut history = ctx.history.borrow_mut();

    for line in reader.lines().filter_map(Result::ok) {
        // Skip empty/whitespace-only lines
        if !line.trim().is_empty() {
            history.push(line);
        }
    }
}
