use crate::builtins::BuiltinResult;
use crate::context::RuntimeContext;
use crate::redirect::CmdOutput;

pub fn builtin_history(_args: &[String], ctx: &RuntimeContext) -> BuiltinResult {
    let output = ctx.history.borrow().clone().concat();
    BuiltinResult::Output(CmdOutput::out(output))
}
pub fn append_history(command: &String, ctx: &RuntimeContext) {
    let index = ctx.history.borrow().len() + 1;
    let mut entry = "\n".to_string() + &index.to_string() + " " + &command;
    if index == 1 {
        entry = entry.trim_start_matches('\n').to_string();
    }
    ctx.history.borrow_mut().push(entry);
}
