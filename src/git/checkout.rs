use anyhow::{Result, bail};
use std::path::Path;

use crate::git::{ObjectKind, TreeEntryMode, get_object};

pub fn checkout(git_root: &Path, sha: &str, ref_name: &str) -> Result<()> {
    let target = git_root.parent().unwrap();

    println!("checking out {}", sha);

    match get_object(git_root, sha)? {
        ObjectKind::Commit { tree, .. } => {
            walk_tree(git_root, &target, &tree)?;
        }
        _ => bail!("invalid sha, must be a commit"),
    }

    std::fs::write(
        git_root.join("HEAD"),
        format!("ref: {}", ref_name).as_bytes(),
    )?;

    Ok(())
}

fn walk_tree(git_root: &Path, target: &Path, sha: &str) -> Result<()> {
    match get_object(git_root, sha)? {
        ObjectKind::Tree(entries) => {
            for entry in entries {
                let full_path = target.join(entry.name);
                let sha = hex::encode(entry.sha1);

                match entry.mode {
                    TreeEntryMode::Directory => {
                        std::fs::create_dir(&full_path)?;
                        walk_tree(git_root, &full_path, &sha)?;
                    }
                    _ => {
                        let blob = get_object(git_root, &sha)?;
                        match blob {
                            ObjectKind::Blob(data) => {
                                std::fs::write(&full_path, data)?;
                            }
                            _ => bail!("expected blob"),
                        }
                    }
                }
            }
        }
        _ => bail!("expected tree"),
    }

    Ok(())
}
