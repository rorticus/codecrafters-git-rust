use anyhow::{Result, bail};
use sha1::{Digest, Sha1};

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
