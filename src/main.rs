mod builtins;
mod context;
mod executor;
mod jobs;
mod parser;
mod readline;
mod redirect;
mod terminal;
mod tokenizer;
pub(crate) mod trie;
mod utils;

fn main() {
    let registry = builtins::BuiltinRegistry::new();
    let ctx = context::RuntimeContext::new(&registry);

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
