#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush();

        //wait for input
        let mut command = String::new();
        io::stdin().read_line(&mut command).unwrap();

        let command = command.trim();
        let tokens: Vec<&str> = command.split_whitespace().collect();

        match tokens.as_slice() {
            [] => (),
            ["exit"] => break,
            ["echo", args @ ..] => println!("{}", args.join(" ")),
            ["type", args @ ("type" | "exit" | "echo")] => println!("{args} is a shell builtin"),
            ["type", args @ ..] => println!("{}: not found", args[0]),
            _ => println!("{command}: command not found"),
        }
    }
}
