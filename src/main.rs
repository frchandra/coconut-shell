mod builtins;
mod executor;
mod parser;
mod redirect;
mod tokenizer;
mod utils;
use std::io::{self, Read, Write};

pub struct ReadResult {
    pub line: String,
    pub from_tab: bool,
}

fn read_line() -> ReadResult {
    // Put terminal in raw mode so we get bytes one at a time
    set_raw_mode(true);

    let mut line = String::new();
    let stdin = io::stdin();
    let mut byte = [0u8; 1];

    loop {
        stdin.lock().read_exact(&mut byte).unwrap();

        match byte[0] {
            b'\t' => {
                // 0x09 — Tab
                set_raw_mode(false);
                return ReadResult {
                    line,
                    from_tab: true,
                };
            }
            b'\r' | b'\n' => {
                // 0x0D / 0x0A — Enter
                set_raw_mode(false);
                print!("\n");
                io::stdout().flush().unwrap();
                return ReadResult {
                    line,
                    from_tab: false,
                };
            }
            127 | 8 => {
                // Backspace (DEL / BS)
                if line.pop().is_some() {
                    print!("\x08 \x08"); // erase character on terminal
                    io::stdout().flush().unwrap();
                }
            }
            c if c >= 32 => {
                // Printable ASCII
                let ch = c as char;
                line.push(ch);
                print!("{ch}");
                io::stdout().flush().unwrap();
            }
            _ => {} // ignore control characters
        }
    }
}

#[cfg(unix)]
fn set_raw_mode(enable: bool) {
    use std::os::fd::AsRawFd;
    unsafe {
        let fd = io::stdin().as_raw_fd();
        let mut termios: libc::termios = std::mem::zeroed();
        libc::tcgetattr(fd, &mut termios);
        if enable {
            termios.c_lflag &= !(libc::ICANON | libc::ECHO);
            termios.c_cc[libc::VMIN] = 1;
            termios.c_cc[libc::VTIME] = 0;
        } else {
            termios.c_lflag |= libc::ICANON | libc::ECHO;
        }
        libc::tcsetattr(fd, libc::TCSANOW, &termios);
    }
}

fn main() {
    let registry = builtins::BuiltinRegistry::new();

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let result = read_line();

        if result.from_tab {
            println!("{}", result.line); // just print current line
            continue;
        }

        let tokens = tokenizer::tokenize(&result.line);
        let pipeline = parser::parse(tokens);

        if pipeline.is_empty() {
            continue;
        }

        if !executor::execute(&pipeline, &registry) {
            break;
        }
    }
}
