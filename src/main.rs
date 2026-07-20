use anyhow::Result;
use anyhow::{anyhow, bail};
use git::find_gitroot;
#[allow(unused_imports)]
use std::env;
#[allow(unused_imports)]
use std::fs;
use std::path::Path;
use url::Url;

mod git;

use clap::{Parser, Subcommand};

use crate::git::{
    CommitPerson, ObjectKind, TreeEntry, TreeEntryMode, build_request, get_info_refs, get_object,
    get_pack, parse_advertisement, post_upload_pack, put_object, strip_nak,
};

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
#[command()]
enum Command {
    Init,
    CatFile {
        #[arg(short = 'p')]
        print: bool,
        sha: String,
    },
    HashObject {
        #[arg(short = 'w')]
        write: bool,
        file: String,
    },
    LsTree {
        #[arg(long = "name-only")]
        name_only: bool,
        sha: String,
    },
    WriteTree,
    CommitTree {
        sha: String,
        #[arg(short = 'p')]
        parent: String,
        #[arg(short = 'm')]
        message: String,
    },
    Clone {
        url: String,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Init => {
            fs::create_dir(".git").unwrap();
            fs::create_dir(".git/objects").unwrap();
            fs::create_dir(".git/refs").unwrap();
            fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
            println!("Initialized git directory")
        }
        Command::CatFile { print, sha } => {
            let git_root = find_gitroot().ok_or_else(|| anyhow!("not a .git repository"))?;

            // find the sha
            let obj = get_object(&git_root, &sha)?;

            match obj {
                ObjectKind::Blob(data) => {
                    if print {
                        for &b in &data {
                            print!("{}", b as char);
                        }
                    }
                }
                _ => bail!("not a blob"),
            }
        }
        Command::HashObject { write, file } => {
            let git_root = find_gitroot().ok_or_else(|| anyhow!("not a .git repository"))?;
            let data = std::fs::read(file)?;

            let obj = ObjectKind::Blob(data);

            if write {
                let sha = put_object(&git_root, &obj)?;

                println!("{}", sha);
            }
        }
        Command::LsTree { name_only, sha } => {
            let git_root = find_gitroot().ok_or_else(|| anyhow!("not a .git repository"))?;

            let obj = get_object(&git_root, &sha)?;

            match obj {
                ObjectKind::Tree(entries) => {
                    for entry in entries {
                        if name_only {
                            println!("{}", entry.name);
                        } else {
                            println!(
                                "{} {} {}\t{}",
                                entry.mode.to_mode(),
                                if let TreeEntryMode::Directory = entry.mode {
                                    "tree"
                                } else {
                                    "blob"
                                },
                                hex::encode(entry.sha1),
                                entry.name
                            )
                        }
                    }
                }
                _ => bail!("not a tree object"),
            }
        }
        Command::WriteTree => {
            let git_root = find_gitroot().ok_or_else(|| anyhow!("not a .git repository"))?;

            fn obj_sha(git_root: &Path, path: &Path) -> Result<TreeEntry> {
                let metadata = std::fs::metadata(path)?;

                if metadata.is_file() {
                    let content = std::fs::read(path)?;
                    let sha1 = put_object(git_root, &ObjectKind::Blob(content))?;

                    Ok(TreeEntry {
                        mode: TreeEntryMode::RegularFile,
                        name: path
                            .file_name()
                            .ok_or_else(|| anyhow!("path has no file name: {}", path.display()))?
                            .to_string_lossy()
                            .to_string(),
                        sha1: hex::decode(sha1)?,
                    })
                } else if metadata.is_dir() {
                    let mut entries = Vec::new();

                    let results = std::fs::read_dir(path)?;
                    for file in results {
                        if let Ok(file) = file {
                            if !file.file_name().to_string_lossy().starts_with(".") {
                                let entry = obj_sha(git_root, &file.path())?;
                                entries.push(entry);
                            }
                        }
                    }

                    let sha1 = put_object(git_root, &ObjectKind::Tree(entries))?;
                    Ok(TreeEntry {
                        mode: TreeEntryMode::Directory,
                        name: path
                            .file_name()
                            .ok_or_else(|| anyhow!("path has no file name: {}", path.display()))?
                            .to_string_lossy()
                            .to_string(),
                        sha1: hex::decode(sha1)?,
                    })
                } else {
                    bail!("unsupported file")
                }
            }

            let cwd = std::env::current_dir()?;
            let entry = obj_sha(&git_root, &cwd)?;

            println!("{}", hex::encode(entry.sha1));
        }
        Command::CommitTree {
            sha,
            parent,
            message,
        } => {
            let git_root = find_gitroot().ok_or_else(|| anyhow!("not a .git repository"))?;
            let obj = ObjectKind::Commit {
                tree: sha,
                parent,
                author: CommitPerson::demo(),
                committer: CommitPerson::demo(),
                message,
            };

            let hash = put_object(&git_root, &obj)?;

            println!("{}", hash);
        }
        Command::Clone { url } => {
            let url = Url::parse(&url)?;

            let advertisement_bytes = get_info_refs(&url)?;
            let result = parse_advertisement(&advertisement_bytes)?;

            let req_body = build_request(&[result.refs[0].sha.as_str()]);

            let upload_pack = post_upload_pack(&url, &req_body)?;
            let no_nak = strip_nak(&upload_pack)?;

            let pack = get_pack(&no_nak)?;

            println!("count of objects in pack: {}", pack.count);
        }
    }

    Ok(())
}
