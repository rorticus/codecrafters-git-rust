use std::fs;
use std::path::Path;

pub fn git_init(dir: &Path) {
    fs::create_dir(dir.join(".git")).unwrap();
    fs::create_dir(dir.join(".git/objects")).unwrap();
    fs::create_dir(dir.join(".git/refs")).unwrap();
    fs::create_dir(dir.join(".git/refs/heads")).unwrap();
    fs::write(dir.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
}
