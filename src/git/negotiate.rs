use crate::git::pktline::PktLine;

pub fn build_request(shas: &[&str]) -> Vec<u8> {
    let mut body = Vec::<u8>::new();

    // Request each wanted object. We deliberately send no capabilities so the
    // server replies with a plain "NAK\n" followed by the raw packfile, which
    // keeps the response easy to parse.
    for sha in shas {
        body.extend(PktLine::data(format!("want {}\n", sha).as_bytes()).encode());
    }

    body.extend(PktLine::flush().encode());
    body.extend(PktLine::data("done\n".as_bytes()).encode());

    body
}
