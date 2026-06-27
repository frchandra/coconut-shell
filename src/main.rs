mod builtins;
mod executor;
mod parser;
mod redirect;
mod tokenizer;
mod utils;
mod io;
use std::io as stdio;
use stdio::{Write};

fn main() {
    let registry = builtins::BuiltinRegistry::new();

    loop {
        print!("$ ");
        stdio::stdout().flush().unwrap();

        let result = io::read_line();
        let tokens = tokenizer::tokenize(&result);
        let pipeline = parser::parse(tokens);

        if pipeline.is_empty() {
            continue;
        }

        if !executor::execute(&pipeline, &registry) {
            break;
        }
    }
}
