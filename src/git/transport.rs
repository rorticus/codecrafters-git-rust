use anyhow::{Result, anyhow, bail};
use reqwest;
use url::Url;

pub fn get_info_refs(url: &Url) -> Result<Vec<u8>> {
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

    Ok(bytes.to_vec())
}

pub fn post_upload_pack(url: &Url, body: &[u8]) -> Result<Vec<u8>> {
    let mut negotiate_url = url.clone();
    negotiate_url
        .path_segments_mut()
        .map_err(|_| anyhow!("bad repo URL"))?
        .pop_if_empty()
        .extend(&["git-upload-pack"]);

    let client = reqwest::blocking::Client::builder().build()?;

    let response = client
        .post(negotiate_url)
        .header("content-type", "application/x-git-upload-pack-request")
        .header("accept", "application/x-git-upload-pack-result")
        .body(body.to_vec())
        .send()?;

    let bytes = response.bytes()?;

    Ok(bytes.to_vec())
}
