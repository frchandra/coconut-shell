mod builtins;
mod executor;
mod parser;
mod redirect;
mod tokenizer;
mod utils;
mod readline;
mod terminal;
pub(crate) mod trie;

fn main() {
    let registry = builtins::BuiltinRegistry::new();
    let ctx = builtins::BuiltinContext::from_registry(&registry);

    loop {
        let result = readline::read_line();
        let tokens = tokenizer::tokenize(&result);
        let pipeline = parser::parse(tokens);

        if pipeline.is_empty() {
            continue;
        }

        if !executor::execute(&pipeline, &registry, &ctx) {
            break;
        }
    }
}
