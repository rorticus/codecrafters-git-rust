use anyhow::{Result, bail};

use crate::git::pktline::{PktLine, PktLineReader};

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

pub fn strip_nak(bytes: &[u8]) -> Result<&[u8]> {
    let mut reader = PktLineReader::new(bytes);

    match reader.next() {
        Some(Ok(PktLine::Data(line))) => {
            let line = line.strip_suffix(b"\n").unwrap_or(line);
            if line != b"NAK" {
                bail!("expected NAK, got {:?}", String::from_utf8_lossy(line));
            }
        }
        Some(Ok(_)) => bail!("expected a NAK data line, got a control pkt-line"),
        Some(Err(e)) => return Err(e),
        None => bail!("empty upload-pack response"),
    }

    Ok(reader.remaining())
}
