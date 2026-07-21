use anyhow::Result;
use std::path::Path;
use url::Url;

use crate::{
    build_request, get_info_refs, get_pack, parse_advertisement, parse_pack, post_upload_pack,
    strip_nak,
};

pub fn clone(git_root: &Path, url: Url) -> Result<()> {
    let advertisement_bytes = get_info_refs(&url)?;
    let result = parse_advertisement(&advertisement_bytes)?;

    let req_body = build_request(&[result.refs[0].sha.as_str()]);

    let upload_pack = post_upload_pack(&url, &req_body)?;
    let no_nak = strip_nak(&upload_pack)?;

    let pack = get_pack(&no_nak)?;
    let objects = parse_pack(&pack)?;

    Ok(())
}
