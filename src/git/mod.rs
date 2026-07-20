use std::path::PathBuf;

mod discovery;
mod negotiate;
mod objects;
mod pack;
mod pktline;
mod transport;

pub use discovery::parse_advertisement;
pub use negotiate::{build_request, strip_nak};
pub use objects::kind::{CommitPerson, ObjectKind, TreeEntry, TreeEntryMode};
pub use objects::{get_object, put_object};
pub use pack::get_pack;
pub use transport::{get_info_refs, post_upload_pack};

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
