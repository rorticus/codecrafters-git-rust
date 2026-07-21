pub mod kind;

use anyhow::{Result, anyhow, bail};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use kind::{CommitPerson, ObjectKind, TreeEntry, TreeEntryMode};
use sha1::{Digest, Sha1};
use std::io::Read;
use std::io::Write;
use std::path::Path;

fn split_once_byte(s: &[u8], sep: u8) -> Option<(&[u8], &[u8])> {
    let i = s.iter().position(|&b| b == sep)?;
    Some((&s[..i], &s[i + 1..]))
}

/// Parse a loose object: `"<type> <size>\0<content>"`. Strips the header and
/// dispatches to `parse_body`.
pub fn parse_object(bytes: &[u8]) -> Result<ObjectKind> {
    let nul = bytes
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| anyhow!("object header has no NUL terminator"))?;
    let (header, content) = bytes.split_at(nul);
    let content = &content[1..];

    let (obj_type, _size) =
        split_once_byte(header, b' ').ok_or_else(|| anyhow!("malformed object header"))?;
    let obj_type = std::str::from_utf8(obj_type)?;
    // let size: usize = std::str::from_utf8(size)?.parse()?;

    parse_body(obj_type, content)
}

/// Parse an object body when the type is already known (e.g. from a packfile
/// entry header). `content` is the raw, un-prefixed object content.
pub fn parse_body(obj_type: &str, content: &[u8]) -> Result<ObjectKind> {
    match obj_type {
        "blob" => Ok(ObjectKind::Blob(content.to_vec())),
        "tree" => {
            let mut entries = Vec::new();

            let mut content_left = &content[..];
            loop {
                if content_left.len() == 0 {
                    break;
                }

                let (mode, rest) = split_once_byte(content_left, b' ').unwrap();
                let (name, rest) = split_once_byte(rest, b'\0').unwrap();
                let sha1 = &rest[0..20].to_vec();

                content_left = &rest[20..];

                entries.push(TreeEntry {
                    mode: TreeEntryMode::from_mode(std::str::from_utf8(mode)?),
                    name: std::str::from_utf8(name)?.to_string(),
                    sha1: sha1.clone(),
                });
            }

            Ok(ObjectKind::Tree(entries))
        }
        "commit" => {
            let mut is_message = false;
            let mut tree_sha = None;
            let mut parents = Vec::new();
            let mut author = None;
            let mut committer = None;
            let mut message = String::new();

            for line in std::str::from_utf8(content)?.lines() {
                if !is_message {
                    if let Some(tree) = line.strip_prefix("tree ") {
                        tree_sha = Some(tree.to_string());
                    }

                    if let Some(sha) = line.strip_prefix("parent ") {
                        parents.push(sha.to_string());
                    }
                    if let Some(author_str) = line.strip_prefix("author ") {
                        author = Some(CommitPerson::parse(author_str)?);
                    }
                    if let Some(committer_str) = line.strip_prefix("committer ") {
                        committer = Some(CommitPerson::parse(committer_str)?);
                    } else if line == "" {
                        is_message = true;
                    }
                } else {
                    message = message + line;
                }
            }

            Ok(ObjectKind::Commit {
                tree: tree_sha.ok_or_else(|| anyhow!("missing tree sha"))?,
                parents,
                author: author.ok_or_else(|| anyhow!("missing author"))?,
                committer: committer.ok_or_else(|| anyhow!("missing committer"))?,
                message,
            })
        }
        t => bail!("unsupported object type {}", t),
    }
}

pub fn get_object(git_root: &Path, sha: &str) -> Result<ObjectKind> {
    let lowercased = sha.to_lowercase();
    let (prefix, rest) = lowercased.split_at(2);

    let object_path = git_root.join("objects").join(prefix).join(rest);

    if object_path.exists() {
        // read the value
        let contents = std::fs::read(object_path)?;
        let out = zlib_decompress(&contents)?;

        parse_object(&out)
    } else {
        Err(anyhow!("object not found at {}", object_path.display()))
    }
}

fn sort_key(entry: &TreeEntry) -> Vec<u8> {
    let mut key = entry.name.as_bytes().to_vec();
    if let TreeEntryMode::Directory = entry.mode {
        key.push(b'/');
    }
    key
}

pub fn put_object(git_root: &Path, obj: &ObjectKind) -> Result<String> {
    let obj_content: Vec<u8> = match obj {
        ObjectKind::Blob(data) => {
            let mut bytes = Vec::new();
            bytes.extend(format!("blob {}\0", data.len()).as_bytes());
            bytes.extend(data);
            bytes
        }
        ObjectKind::Tree(entries) => {
            let mut bytes = Vec::new();
            let mut tree_data = Vec::new();

            // Git sorts tree entries by name, treating directories as if their
            // name had a trailing '/'.
            let mut entries: Vec<&TreeEntry> = entries.iter().collect();
            entries.sort_by(|a, b| sort_key(a).cmp(&sort_key(b)));

            for entry in entries {
                tree_data.extend(entry.mode.to_mode().as_bytes());
                tree_data.push(b' ');
                tree_data.extend(entry.name.as_bytes());
                tree_data.push(b'\0');
                tree_data.extend(&entry.sha1);
            }

            bytes.extend(format!("tree {}\0", tree_data.len()).as_bytes());
            bytes.extend(tree_data);
            bytes
        }
        ObjectKind::Commit {
            tree,
            parents,
            author,
            committer,
            message,
        } => {
            let mut bytes = Vec::new();
            let mut content = String::new();

            content += format!("tree {}\n", tree).as_str();
            for parent in parents {
                content += format!("parent {}\n", parent).as_str();
            }
            content += format!("author {}\n", author.to_str()).as_str();
            content += format!("committer {}\n", committer.to_str()).as_str();
            content += "\n";
            content += message;
            content += "\n";

            let content_bytes = content.as_bytes();

            bytes.extend(format!("commit {}\0", content_bytes.len()).as_bytes());
            bytes.extend(content_bytes);
            bytes
        }
    };

    // get the sha
    let sha = sha1_of(&obj_content);

    put_body(git_root, &sha, &obj_content)?;

    return Ok(sha);
}

fn put_body(git_root: &Path, sha: &str, body: &[u8]) -> Result<()> {
    let compressed_data = zlib_compress(&body)?;

    let lowercased = sha.to_lowercase();
    let (prefix, rest) = lowercased.split_at(2);

    let parent_path = git_root.join("objects").join(prefix);
    let object_path = parent_path.join(rest);

    std::fs::create_dir_all(parent_path)?;

    std::fs::write(&object_path, &compressed_data)?;

    Ok(())
}

pub fn put_object_raw(git_root: &Path, sha: &str, obj_type: &str, body: &[u8]) -> Result<()> {
    let mut full_body = Vec::new();
    full_body.extend(format!("{} {}\0", obj_type, body.len()).as_bytes());
    full_body.extend(body);

    put_body(git_root, sha, &full_body)?;

    Ok(())
}

fn zlib_compress(data: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    encoder.finish()
}

fn zlib_decompress(data: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(data);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out)?;
    Ok(out)
}

fn sha1_of(data: &[u8]) -> String {
    format!("{:x}", Sha1::digest(&data))
}
