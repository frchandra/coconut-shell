use crate::builtins::BuiltinResult;
use crate::context::RuntimeContext;
use crate::redirect::CmdOutput;

pub fn builtin_history(_args: &[String], ctx: &RuntimeContext) -> BuiltinResult {
    let output = ctx.history.borrow().clone().concat();
    BuiltinResult::Output(CmdOutput::out(output))
}
pub fn append_history(command: &String, ctx: &RuntimeContext) {
    ctx.history.borrow_mut().push(command.clone());
    ctx.history.borrow_mut().push("\n".to_string());
}
