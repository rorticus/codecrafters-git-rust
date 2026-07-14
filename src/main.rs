use anyhow::Result;
use anyhow::{anyhow, bail};
use git::find_gitroot;
#[allow(unused_imports)]
use std::env;
#[allow(unused_imports)]
use std::fs;

use clap::{Parser, Subcommand};

use crate::git::get_object;
use crate::git::put_object;
use crate::git::{GitObject, TreeEntryMode};

mod git;

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
                GitObject::Blob(data) => {
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

            let obj = GitObject::Blob(data);

            if write {
                let sha = put_object(&git_root, &obj)?;

                println!("{}", sha);
            }
        }
        Command::LsTree { name_only, sha } => {
            let git_root = find_gitroot().ok_or_else(|| anyhow!("not a .git repository"))?;

            let obj = get_object(&git_root, &sha)?;

            match obj {
                GitObject::Tree(entries) => {
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
    }

    Ok(())
}
