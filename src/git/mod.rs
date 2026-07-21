use std::path::PathBuf;

mod checkout;
mod clone;
mod discovery;
mod init;
mod negotiate;
mod objects;
mod pack;
mod pktline;
mod transport;

pub use checkout::checkout;
pub use clone::clone;
pub use discovery::parse_advertisement;
pub use init::git_init;
pub use negotiate::{build_request, strip_nak};
pub use objects::kind::{CommitPerson, ObjectKind, TreeEntry, TreeEntryMode};
pub use objects::{get_object, put_object, put_object_raw};
pub use pack::{get_pack, parse_pack};
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
