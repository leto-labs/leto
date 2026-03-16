pub mod file;
pub mod memory;

pub use file::FileStore;
pub use memory::InMemoryStore;

use std::path::PathBuf;

/// Resolve the global brain home directory.
///
/// Uses `$BRAIN_HOME` if set, otherwise `~/.brain/`.
pub fn brain_home() -> PathBuf {
    if let Ok(home) = std::env::var("BRAIN_HOME") {
        return PathBuf::from(home);
    }
    dirs::home_dir()
        .expect("could not determine home directory")
        .join(".brain")
}
