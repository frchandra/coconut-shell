mod builtins;
mod executor;
mod parser;
mod redirect;
mod tokenizer;
mod utils;

use std::io::{self, Write};

fn main() {
    let registry = builtins::BuiltinRegistry::new();

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let input = read_line();
        let tokens = tokenizer::tokenize(&input);
        let pipeline = parser::parse(tokens);

        if pipeline.is_empty() {
            continue;
        }

        if !executor::execute(&pipeline, &registry) {
            break;
        }
    }
}

fn read_line() -> String {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input
}
