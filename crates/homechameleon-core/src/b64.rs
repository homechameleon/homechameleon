//! Small base64 (standard alphabet) for the wasm JSON boundary.
//! Raw pixels stay bytes inside the crate; JSON carries them as text.

#[allow(dead_code)]
const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

#[allow(dead_code)]
pub fn encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        let n = (u32::from(b0) << 16) | (u32::from(b1) << 8) | u32::from(b2);
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

pub fn decode(text: &str) -> Result<Vec<u8>, String> {
    let bytes = text.trim().as_bytes();
    if !bytes.len().is_multiple_of(4) {
        return Err("base64 length is not a multiple of 4".into());
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for chunk in bytes.chunks(4) {
        let mut n = 0u32;
        let mut pads = 0;
        for (i, c) in chunk.iter().enumerate() {
            if *c == b'=' {
                pads += 1;
                continue;
            }
            if pads > 0 {
                return Err("base64 padding is in the wrong place".into());
            }
            let v = val(*c).ok_or_else(|| format!("invalid base64 byte at {i}"))?;
            n = (n << 6) | u32::from(v);
        }
        if pads > 2 {
            return Err("too much base64 padding".into());
        }
        let shift = pads * 6;
        n <<= shift;
        out.push((n >> 16) as u8);
        if pads < 2 {
            out.push((n >> 8) as u8);
        }
        if pads < 1 {
            out.push(n as u8);
        }
    }
    Ok(out)
}

fn val(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        for sample in [
            b"".as_slice(),
            b"a",
            b"ab",
            b"abc",
            b"rgba-pixels",
            &[0, 1, 2, 255, 10],
        ] {
            let back = decode(&encode(sample)).unwrap();
            assert_eq!(back, sample);
        }
    }
}
