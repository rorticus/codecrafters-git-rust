use anyhow::{Result, anyhow, bail};
use reqwest;
use url::Url;

use crate::git::pktline::{PktLine, PktLineReader};

#[derive(Debug)]
pub struct Ref {
    pub sha: String,
    pub name: String,
}

#[derive(Debug)]
pub struct InfoRefs {
    pub capabilities: Vec<String>,
    pub refs: Vec<Ref>,
}

pub fn get_info_refs(url: Url) -> Result<InfoRefs> {
    let mut info_url = url.clone();
    info_url
        .path_segments_mut()
        .map_err(|_| anyhow!("bad repo URL"))?
        .pop_if_empty()
        .extend(&["info", "refs"]);

    info_url.set_query(Some("service=git-upload-pack"));

    let response = reqwest::blocking::get(info_url)?;

    if response.status() != 200 {
        bail!("bad response code: {}", response.status());
    }

    let content_type = response.headers().get("content-type");
    if content_type.is_some()
        && content_type.unwrap() != "application/x-git-upload-pack-advertisement"
    {
        bail!("invalid content type: {:?}", content_type.unwrap());
    }

    let bytes = response.bytes()?;

    let pkt_reader = PktLineReader::new(&bytes);

    let mut flushes = 0;

    let mut refs = Vec::new();
    let mut capabilities = Vec::new();

    for pkt_line in pkt_reader {
        match pkt_line {
            Ok(PktLine::Flush) => {
                flushes += 1;
            }
            Ok(PktLine::Data(data)) => {
                if flushes == 1 {
                    let ref_bit = {
                        if refs.len() == 0 {
                            let zero = data.iter().position(|b| *b == 0).unwrap_or(0);
                            let (ref_bit, rest) = data.split_at(zero);

                            for cap in std::str::from_utf8(rest)?.split(" ") {
                                capabilities.push(cap.to_string());
                            }

                            std::str::from_utf8(ref_bit)?
                        } else {
                            std::str::from_utf8(data)?
                        }
                    };

                    let (sha, name) = ref_bit.split_once(" ").unwrap();

                    refs.push(Ref {
                        sha: sha.trim().to_string(),
                        name: name.trim().to_string(),
                    });
                }
            }
            _ => {
                println!("unexpected pktline");
            }
        }
    }

    Ok(InfoRefs { capabilities, refs })
}
