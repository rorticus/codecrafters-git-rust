use anyhow::{Result, anyhow, bail};
use flate2::read::ZlibDecoder;
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::io::Read;

// Pack object type tags (the 3-bit field in the entry header).
const OBJ_COMMIT: u8 = 1;
const OBJ_TREE: u8 = 2;
const OBJ_BLOB: u8 = 3;
const OBJ_TAG: u8 = 4;
const OBJ_OFS_DELTA: u8 = 6;
const OBJ_REF_DELTA: u8 = 7;

pub struct Pack<'a> {
    pub count: u32,
    pub objects: &'a [u8],
}

pub fn get_pack(bytes: &[u8]) -> Result<Pack<'_>> {
    // 12-byte header + 20-byte trailer is the minimum possible pack.
    if bytes.len() < 32 {
        bail!("pack too short: {} bytes", bytes.len());
    }

    if &bytes[0..4] != b"PACK" {
        bail!("bad pack magic: {:?}", &bytes[0..4]);
    }

    let version = u32::from_be_bytes(bytes[4..8].try_into().unwrap());
    if version != 2 {
        bail!("unsupported pack version: {}", version);
    }

    let count = u32::from_be_bytes(bytes[8..12].try_into().unwrap());

    // The last 20 bytes are a SHA-1 over everything preceding them.
    let (body, trailer) = bytes.split_at(bytes.len() - 20);
    let actual = Sha1::digest(body);
    if actual.as_slice() != trailer {
        bail!("pack checksum mismatch");
    }

    // Object entries live between the header and the trailer.
    let objects = &body[12..];

    Ok(Pack { count, objects })
}

pub fn parse_pack(pack: &Pack) -> Result<HashMap<String, (String, Vec<u8>)>> {
    let mut cursor = 0;
    let stream = pack.objects;

    let mut by_sha: HashMap<String, (String, Vec<u8>)> = HashMap::new();

    for _ in 0..pack.count {
        let (obj_type, _size, new_pos) = parse_type_size(stream, cursor);
        cursor = new_pos;

        // For a ref-delta the zlib stream is preceded by a 20-byte base SHA.
        let base_sha = if obj_type == OBJ_REF_DELTA {
            let sha = hex::encode(&stream[cursor..cursor + 20]);
            cursor += 20;
            Some(sha)
        } else {
            None
        };

        let (raw, consumed) = inflate(&stream[cursor..])?;
        cursor += consumed;

        let (type_str, content) = match obj_type {
            OBJ_COMMIT => ("commit".to_string(), raw),
            OBJ_TREE => ("tree".to_string(), raw),
            OBJ_BLOB => ("blob".to_string(), raw),
            OBJ_TAG => ("tag".to_string(), raw),
            OBJ_REF_DELTA => {
                let base_sha = base_sha.unwrap();
                let (base_type, base_content) = by_sha
                    .get(&base_sha)
                    .ok_or_else(|| anyhow!("delta base {} not seen yet", base_sha))?;
                let content = apply_delta(base_content, &raw)?;
                (base_type.clone(), content)
            }
            OBJ_OFS_DELTA => bail!("ofs-delta not supported (ofs-delta was not requested)"),
            _ => bail!("unsupported object type {}", obj_type),
        };

        let sha = hash_object(&type_str, &content);
        by_sha.insert(sha, (type_str.clone(), content.clone()));
    }

    Ok(by_sha)
}

/// Inflate a zlib stream, returning the decompressed bytes and how many
/// compressed bytes were consumed (so the caller can advance its cursor).
fn inflate(data: &[u8]) -> Result<(Vec<u8>, usize)> {
    let mut dec = ZlibDecoder::new(data);
    let mut out = Vec::new();
    dec.read_to_end(&mut out)?;
    Ok((out, dec.total_in() as usize))
}

/// SHA-1 of a git object: `"<type> <len>\0<content>"`.
fn hash_object(obj_type: &str, content: &[u8]) -> String {
    let mut buf = Vec::new();
    buf.extend_from_slice(format!("{} {}\0", obj_type, content.len()).as_bytes());
    buf.extend_from_slice(content);
    format!("{:x}", Sha1::digest(&buf))
}

/// Reconstruct an object from a delta applied against its base content.
/// Delta = source-size varint, target-size varint, then copy/insert opcodes.
fn apply_delta(base: &[u8], delta: &[u8]) -> Result<Vec<u8>> {
    let mut pos = 0;

    let (_source_size, n) = read_varint(delta, pos);
    pos += n;
    let (target_size, n) = read_varint(delta, pos);
    pos += n;

    let mut out = Vec::with_capacity(target_size);

    while pos < delta.len() {
        let instr = delta[pos];
        pos += 1;

        if instr & 0x80 != 0 {
            // Copy: the low 7 bits select which offset/size bytes follow.
            let mut offset = 0usize;
            for i in 0..4 {
                if instr & (1 << i) != 0 {
                    offset |= (delta[pos] as usize) << (8 * i);
                    pos += 1;
                }
            }
            let mut size = 0usize;
            for i in 0..3 {
                if instr & (1 << (4 + i)) != 0 {
                    size |= (delta[pos] as usize) << (8 * i);
                    pos += 1;
                }
            }
            if size == 0 {
                size = 0x10000; // a size of 0 means 64KiB
            }
            out.extend_from_slice(&base[offset..offset + size]);
        } else if instr != 0 {
            // Insert: `instr` literal bytes follow.
            let n = instr as usize;
            out.extend_from_slice(&delta[pos..pos + n]);
            pos += n;
        } else {
            bail!("invalid delta opcode 0x00");
        }
    }

    if out.len() != target_size {
        bail!(
            "delta target size mismatch: expected {}, got {}",
            target_size,
            out.len()
        );
    }

    Ok(out)
}

/// Little-endian base-128 varint (as used in delta headers). Returns the value
/// and the number of bytes consumed.
fn read_varint(data: &[u8], start: usize) -> (usize, usize) {
    let mut pos = start;
    let mut result = 0usize;
    let mut shift = 0;

    loop {
        let b = data[pos];
        pos += 1;
        result |= ((b & 0x7f) as usize) << shift;
        shift += 7;
        if b & 0x80 == 0 {
            break;
        }
    }

    (result, pos - start)
}

fn parse_type_size(bytes: &[u8], pos: usize) -> (u8, usize, usize) {
    let b = bytes[pos];
    let mut p = pos;
    p += 1;

    let obj_type = (b >> 4) & 0x07; // bits 6 - 4 are size
    let mut size = (b & 0x0F) as usize; // lower bits are size
    let mut shift = 4;
    let mut more = (b & 0x80) != 0;

    while more {
        let b = bytes[p];
        p += 1;

        size |= ((b & 0x7F) as usize) << shift;
        shift += 7;
        more = b & 0x80 != 0;
    }

    (obj_type, size, p)
}
