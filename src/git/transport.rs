use anyhow::{Result, anyhow, bail};
use reqwest;
use url::Url;

use crate::git::pktline::PktLineReader;

pub fn get_info_refs(url: Url) -> Result<()> {
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

    for pkt_line in pkt_reader {
        println!("{:?}", pkt_line);
    }

    Ok(())
}
