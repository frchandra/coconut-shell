use crate::builtins::BuiltinRegistry;
use crate::jobs::JobContext;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Shared runtime context passed throughout the shell.
///
/// Contains all mutable and read-only state that various subsystems
/// (builtins, executor, readline) might need.  Constructed once at
/// startup and shared for the entire lifetime of the shell.
#[derive(Clone)]
pub struct RuntimeContext {
    pub builtin_names: Vec<String>,
    /// Mutable at runtime via interior mutability (`RefCell`).
    /// Use `.borrow()` to read and `.borrow_mut()` to write.
    pub completion: RefCell<HashMap<String, String>>,
    /// Job table shared between the executor and the `jobs` builtin.
    /// Uses `Arc<Mutex<>>` so it can be safely sent to background threads.
    pub jobs: Arc<Mutex<JobContext>>,
}

impl RuntimeContext {
    /// Build the runtime context from a [`BuiltinRegistry`].
    ///
    /// This is the single point of construction — keeps `main.rs` clean.
    pub fn new(registry: &BuiltinRegistry) -> Self {
        Self {
            builtin_names: registry.names().into_iter().map(|s| s.to_owned()).collect(),
            completion: RefCell::new(HashMap::new()),
            jobs: Arc::new(Mutex::new(JobContext::new())),
        }
    }
}
