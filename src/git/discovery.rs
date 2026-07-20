use crate::git::pktline::{PktLine, PktLineReader};
use anyhow::Result;

#[derive(Debug)]
#[allow(dead_code)]
pub struct Ref {
    pub sha: String,
    pub name: String,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Advertisement {
    pub capabilities: Vec<String>,
    pub refs: Vec<Ref>,
}

pub fn parse_advertisement(bytes: &[u8]) -> Result<Advertisement> {
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

    Ok(Advertisement { capabilities, refs })
}
