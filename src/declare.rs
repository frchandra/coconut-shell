use crate::builtins::BuiltinResult;
use crate::context::RuntimeContext;
use crate::redirect::CmdOutput;

pub fn builtin_declare(args: &[String], _ctx: &RuntimeContext) -> BuiltinResult {
    if args.get(0) == Some(&"-p".into()) {
        // print missing variable
        //check if variable is present in declared_variables
        let declared_variables = _ctx.declared_variables.borrow();
        if let Some(value) = declared_variables.get(args.get(1).unwrap()) {
            let output = format!("declare -- {}=\"{}\"", args.get(1).unwrap(), value);
            return BuiltinResult::Output(CmdOutput::out(output));
        } else {
            let output = format!("declare: {}: not found", args.get(1).unwrap());        
            return BuiltinResult::Output(CmdOutput::out(output));    
        }        
    } else {
        // assign variable
        if let Some(name) = args.get(0) {
            // seperate the value from the name if it is in the form of name=value
            let parts: Vec<&str> = name.splitn(2, '=').collect();
            let name = parts[0];
            let value = if parts.len() > 1 { parts[1] } else {
                // if no value is provided, set it to an empty string
                ""
            };
            assign_overwrite_variable(_ctx, name, value);
            
        } else {
            return BuiltinResult::Output(CmdOutput::out("declare: missing variable name".to_string()));

        }
    }

    return BuiltinResult::Output(CmdOutput::empty())
}


pub fn assign_overwrite_variable(ctx: &RuntimeContext, name: &str, value: &str) {
    ctx.declared_variables
        .borrow_mut()
        .insert(name.to_string(), value.to_string());
}

