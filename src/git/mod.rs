use anyhow::{Result, anyhow, bail};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use sha1::{Digest, Sha1};
use std::io::Read;
use std::io::Write;
use std::path::{Path, PathBuf};

pub enum GitObject {
    Blob(Vec<u8>),
}

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

fn split_once_byte(s: &[u8], sep: u8) -> Option<(&[u8], &[u8])> {
    let i = s.iter().position(|&b| b == sep)?;
    Some((&s[..i], &s[i + 1..]))
}

pub fn get_object(git_root: &Path, sha: &str) -> Result<GitObject> {
    let lowercased = sha.to_lowercase();
    let (prefix, rest) = lowercased.split_at(2);

    let object_path = git_root.join("objects").join(prefix).join(rest);

    if object_path.exists() {
        // read the value
        let contents = std::fs::read(object_path)?;
        let out = zlib_decompress(&contents)?;

        let nul = out.iter().position(|&b| b == 0).unwrap_or(0);
        let (header, content) = out.split_at(nul);
        let content = &content[1..];

        let (obj_type, _size) = split_once_byte(header, b' ').unwrap();
        let obj_type = std::str::from_utf8(obj_type)?;
        // let size: usize = std::str::from_utf8(size)?.parse()?;

        match obj_type {
            "blob" => Ok(GitObject::Blob(content.to_vec())),
            t => bail!("unsupported object type {}", t),
        }
    } else {
        Err(anyhow!("object not found at {}", object_path.display()))
    }
}

pub fn put_object(git_root: &Path, obj: &GitObject) -> Result<PathBuf> {
    let obj_content: Vec<u8> = match obj {
        GitObject::Blob(data) => {
            let mut bytes = Vec::new();
            bytes.extend(format!("blob {}\0", data.len()).as_bytes());
            bytes.extend(data);
            bytes
        }
    };

    // get the sha
    let sha = sha1_of(&obj_content);

    let compressed_data = zlib_compress(&obj_content)?;

    let lowercased = sha.to_lowercase();
    let (prefix, rest) = lowercased.split_at(2);

    let parent_path = git_root.join("objects").join(prefix);
    let object_path = parent_path.join(rest);

    std::fs::create_dir_all(parent_path)?;

    std::fs::write(&object_path, &compressed_data)?;

    return Ok(object_path);
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
