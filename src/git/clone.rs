use anyhow::Result;
use std::path::Path;
use url::Url;

use crate::{
    build_request, get_info_refs, get_pack,
    git::{checkout, git_init, put_object_raw},
    parse_advertisement, parse_pack, post_upload_pack, strip_nak,
};

pub fn clone(target: &Path, url: Url) -> Result<()> {
    let advertisement_bytes = get_info_refs(&url)?;
    let result = parse_advertisement(&advertisement_bytes)?;

    let req_body = build_request(&[result.refs[0].sha.as_str()]);

    let upload_pack = post_upload_pack(&url, &req_body)?;
    let no_nak = strip_nak(&upload_pack)?;

    let pack = get_pack(&no_nak)?;
    let sha_map = parse_pack(&pack)?;

    git_init(target);

    let git_root = target.join(".git");

    for sha in sha_map.keys() {
        let (obj_type, content) = sha_map.get(sha).unwrap();
        put_object_raw(&git_root, &sha, &obj_type, &content)?;
    }

    // cloned, now check out head
    let default_ref = "symref=HEAD:refs/heads/main".to_string();

    let symref = result
        .capabilities
        .iter()
        .find(|c| c.starts_with("symref="))
        .unwrap_or(&default_ref);

    let (_, symref) = symref.split_once("symref=").unwrap();
    let (_, ref_name) = symref.split_once(":").unwrap();

    let main = &result.refs[0];

    std::fs::write(
        git_root.join("refs").join("heads").join(&main.name),
        main.sha.as_bytes(),
    )?;

    checkout(&git_root, &result.refs[0].sha, &ref_name)?;

    Ok(())
}
