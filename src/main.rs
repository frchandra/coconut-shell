use crate::tokenizer::Token;

mod builtins;
mod executor;
mod parser;
mod readline;
mod redirect;
mod terminal;
mod tokenizer;
pub(crate) mod trie;
mod utils;

fn main() {
    let registry = builtins::BuiltinRegistry::new();
    let ctx = builtins::BuiltinContext::from_registry(&registry);

    loop {
        let result = readline::read_line(&ctx);
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
