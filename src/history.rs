use crate::builtins::BuiltinResult;
use crate::context::RuntimeContext;
use crate::redirect::CmdOutput;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
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
    } else if args.get(0) == Some(&"-w".into()) {
        if let Some(file_path) = args.get(1) {
            save_history_to_file(file_path, ctx);
            return BuiltinResult::Output(CmdOutput::empty())
        } else {
            return BuiltinResult::Output(CmdOutput::err(
                "history: missing file path after -w".to_string(),
            ));
        }
    } else if args.get(0) == Some(&"-a".into()) {
        if let Some(file_path) = args.get(1) {
            append_history_to_file(file_path, ctx);
            return BuiltinResult::Output(CmdOutput::empty())
        } else {
            return BuiltinResult::Output(CmdOutput::err(
                "history: missing file path after -a".to_string(),
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

    // Collect lines first to avoid holding mutable borrow while reading
    let mut lines: Vec<String> = reader
        .lines()
        .filter_map(Result::ok)
        .filter(|l| !l.trim().is_empty())
        .collect();

    if lines.is_empty() {
        // nothing new to add, but still mark as written
        *ctx.history_write_index.borrow_mut() = ctx.history.borrow().len();
        return;
    }

    let mut history = ctx.history.borrow_mut();
    history.append(&mut lines);

    // After loading from file consider those entries as already written/appended
    *ctx.history_write_index.borrow_mut() = history.len();
}

pub fn save_history_to_file(path: &String, ctx: &RuntimeContext) {
    // Expand a leading '~' and try to open the file. Silently return on errors.
    let expanded = expand_tilde(path);
    let file = match File::create(expanded) {
        Ok(f) => f,
        Err(_) => return,
    };

    let mut writer = std::io::BufWriter::new(file);
    let history = ctx.history.borrow();

    for entry in history.iter() {
        if let Err(_) = writeln!(writer, "{}", entry) {
            // Silently ignore write errors
            return;
        }
    }

    // All entries are now persisted
    *ctx.history_write_index.borrow_mut() = history.len();
}

pub fn append_history_to_file(path: &String, ctx: &RuntimeContext) {
    // Expand a leading '~' and try to open the file. Silently return on errors.
    let expanded = expand_tilde(path);
    let file = match OpenOptions::new().append(true).create(true).open(expanded) {
        Ok(f) => f,
        Err(_) => return,
    };

    let mut writer = std::io::BufWriter::new(file);
    let history = ctx.history.borrow();
    let mut write_index = *ctx.history_write_index.borrow();

    if write_index > history.len() {
        // Defensive: clamp to current history length
        write_index = history.len();
    }

    if write_index == history.len() {
        // nothing new to append
        return;
    }

    for entry in history[write_index..].iter() {
        if let Err(_) = writeln!(writer, "{}", entry) {
            // Silently ignore write errors
            return;
        }
    }

    // Update the written index to the end of history
    *ctx.history_write_index.borrow_mut() = history.len();
}
