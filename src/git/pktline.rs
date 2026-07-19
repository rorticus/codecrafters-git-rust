use anyhow::{Result, anyhow};

#[derive(Debug)]
pub enum PktLine<'a> {
    Data(&'a [u8]),
    Flush,
    Delim,
    ResponseEnd,
}

impl<'a> PktLine<'a> {
    pub fn trimmed(&self) -> Option<&'a [u8]> {
        match self {
            PktLine::Data(data) => Some(data.strip_suffix(b"\n").unwrap_or(data)),
            _ => None,
        }
    }
}

pub struct PktLineReader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> PktLineReader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        PktLineReader { buf, pos: 0 }
    }

    pub fn remaining(&self) -> &'a [u8] {
        &self.buf[self.pos..]
    }
}

impl<'a> Iterator for PktLineReader<'a> {
    type Item = Result<PktLine<'a>, anyhow::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.buf.len() {
            return None;
        }

        if self.buf.len() - self.pos < 4 {
            self.pos = self.buf.len();
            return Some(Err(anyhow!("header too short")));
        }

        let len = match parse_len(&self.buf[self.pos..self.pos + 4]) {
            Ok(l) => l,
            Err(e) => {
                self.pos = self.buf.len();
                return Some(Err(e));
            }
        };

        match len {
            0 => {
                self.pos += 4;
                Some(Ok(PktLine::Flush))
            }
            1 => {
                self.pos += 4;
                Some(Ok(PktLine::Delim))
            }
            2 => {
                self.pos += 4;
                Some(Ok(PktLine::ResponseEnd))
            }
            3 => {
                self.pos = self.buf.len();
                Some(Err(anyhow!("invalid length: 3")))
            }
            _ => {
                let end = self.pos + len;
                if end > self.buf.len() {
                    self.pos += self.buf.len();
                    return Some(Err(anyhow!("truncated")));
                }

                let payload = &self.buf[self.pos + 4..end];
                self.pos = end;
                Some(Ok(PktLine::Data(payload)))
            }
        }
    }
}

fn parse_len(hex: &[u8]) -> Result<usize> {
    let s = std::str::from_utf8(hex).map_err(|_| anyhow!("bad length"))?;
    usize::from_str_radix(s, 16).map_err(|_| anyhow!("bad length"))
}
