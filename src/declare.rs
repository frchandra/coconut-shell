use crate::builtins::BuiltinResult;
use crate::context::RuntimeContext;
use crate::redirect::CmdOutput;

pub fn builtin_declare(_args: &[String], _ctx: &RuntimeContext) -> BuiltinResult {
    return BuiltinResult::Output(CmdOutput::empty())
}