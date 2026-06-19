use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Search `$PATH` for an executable matching `command`.
pub fn find_executable_in_path(command: &str) -> Option<PathBuf> {
    let path_var = env::var("PATH").unwrap_or_default();

    env::split_paths(&path_var)
        .map(|dir| dir.join(command))
        .find(|target| target.exists() && is_executable(target))
}

/// Check whether a path points to an executable file.
fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(not(unix))]
    {
        path.extension()
            .map(|e| e == "exe" || e == "bat" || e == "cmd")
            .unwrap_or(false)
    }
}

/// Expand a leading `~` to `$HOME`.
///
/// - `"~"`      → `$HOME`
/// - `"~/foo"`  → `$HOME/foo`
/// - anything else is returned unchanged.
pub fn expand_tilde(path: &str) -> String {
    if path == "~" {
        env::var("HOME").unwrap_or_else(|_| path.to_string())
    } else if path.starts_with("~/") {
        let home = env::var("HOME").unwrap_or_default();
        format!("{}{}", home, &path[1..])
    } else {
        path.to_string()
    }
}
