use std::io;
use std::os::fd::AsRawFd;
use std::sync::OnceLock;

/// Stores the original termios settings so they can be restored later.
static ORIGINAL_TERMIOS: OnceLock<libc::termios> = OnceLock::new();

/// Switch the terminal between raw and cooked (canonical) mode.
///
/// In raw mode:
/// - Input is delivered character-by-character (no line buffering).
/// - Local echo is disabled.
///
/// When `enable` is `false` the terminal is restored to its original
/// settings captured the first time raw mode was entered.
#[cfg(unix)]
pub fn set_raw_mode(enable: bool) -> io::Result<()> {
    let fd = io::stdin().as_raw_fd();

    if enable {
        unsafe {
            let mut termios: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(fd, &mut termios) != 0 {
                return Err(io::Error::last_os_error());
            }
            // Save original settings once
            ORIGINAL_TERMIOS.get_or_init(|| termios);

            let mut raw = termios;
            raw.c_lflag &= !(libc::ICANON | libc::ECHO);
            raw.c_cc[libc::VMIN] = 1;
            raw.c_cc[libc::VTIME] = 0;

            if libc::tcsetattr(fd, libc::TCSAFLUSH, &raw) != 0 {
                return Err(io::Error::last_os_error());
            }
        }
    } else if let Some(original) = ORIGINAL_TERMIOS.get() {
        unsafe {
            if libc::tcsetattr(fd, libc::TCSAFLUSH, original) != 0 {
                return Err(io::Error::last_os_error());
            }
        }
    }

    Ok(())
}
