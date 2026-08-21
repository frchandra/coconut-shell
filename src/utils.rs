use std::collections::HashSet;
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

pub fn get_path_executables() -> Vec<String> {
    let mut executables = Vec::new();

    let path_var = match env::var_os("PATH") {
        Some(val) => val,
        None => return executables, // PATH not set
    };

    // PATH is a list of directories separated by ':' (Unix) or ';' (Windows)
    for dir in env::split_paths(&path_var) {
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue, // skip unreadable/non-existent dirs
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_file() && is_executable(&path) {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    executables.push(name.to_string());
                }
            }
        }
    }

    executables
}

pub fn get_path_executables_deduped() -> Vec<String> {
    let mut seen = HashSet::new();
    get_path_executables()
        .into_iter()
        .filter(|name| seen.insert(name.clone()))
        .collect()
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

// fn get_file_names(dir: &str) -> std::io::Result<Vec<String>> {
//     let mut names = Vec::new();
//     for entry in fs::read_dir(dir)? {
//         let entry = entry?;
//         if entry.file_type()?.is_file() {
//             if let Some(name) = entry.file_name().to_str() {
//                 names.push(name.to_string());
//             }
//         }
//     }
//     Ok(names)
// }
