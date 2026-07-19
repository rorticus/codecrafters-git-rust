use std::path::PathBuf;

mod objects;

pub use objects::kind::{CommitPerson, ObjectKind, TreeEntry, TreeEntryMode};
pub use objects::{get_object, put_object};

pub fn find_gitroot() -> Option<PathBuf> {
    if let Ok(dir) = std::env::current_dir() {
        dir.ancestors().find_map(|dir| {
            let candidate = dir.join(".git");
            candidate.exists().then_some(candidate)
        })
    } else {
        None
    }
}
