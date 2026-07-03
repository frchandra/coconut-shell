mod builtins;
mod executor;
mod io;
mod parser;
mod redirect;
mod tokenizer;
mod utils;

fn main() {
    let registry = builtins::BuiltinRegistry::new();

    loop {
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
