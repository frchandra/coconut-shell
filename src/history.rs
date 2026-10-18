use crate::builtins::BuiltinResult;
use crate::context::RuntimeContext;
use crate::redirect::CmdOutput;

pub fn builtin_history(args: &[String], ctx: &RuntimeContext) -> BuiltinResult {
    //check if args[0] exist
    if let Some(limit) = args.get(0) {
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
