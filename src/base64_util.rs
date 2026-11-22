//! Minimal, zero-dependency RFC 4648 Base64 codec and PNG header dimension reader.

use crate::error::{HydrogenError, Result};

const B64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encode raw bytes as standard padded RFC 4648 Base64.
pub fn encode_base64(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let (chunks, rem) = input.as_chunks::<3>();
    for chunk in chunks {
        let n = (u32::from(chunk[0]) << 16) | (u32::from(chunk[1]) << 8) | u32::from(chunk[2]);
        out.push(B64_ALPHABET[((n >> 18) & 0x3f) as usize] as char);
        out.push(B64_ALPHABET[((n >> 12) & 0x3f) as usize] as char);
        out.push(B64_ALPHABET[((n >> 6) & 0x3f) as usize] as char);
        out.push(B64_ALPHABET[(n & 0x3f) as usize] as char);
    }
    match rem.len() {
        1 => {
            let n = u32::from(rem[0]) << 16;
            out.push(B64_ALPHABET[((n >> 18) & 0x3f) as usize] as char);
            out.push(B64_ALPHABET[((n >> 12) & 0x3f) as usize] as char);
            out.push('=');
            out.push('=');
        }
        2 => {
            let n = (u32::from(rem[0]) << 16) | (u32::from(rem[1]) << 8);
            out.push(B64_ALPHABET[((n >> 18) & 0x3f) as usize] as char);
            out.push(B64_ALPHABET[((n >> 12) & 0x3f) as usize] as char);
            out.push(B64_ALPHABET[((n >> 6) & 0x3f) as usize] as char);
            out.push('=');
        }
        _ => {}
    }
    out
}

/// Decode standard RFC 4648 Base64 (ignoring ASCII whitespace; accepts optional `data:...;base64,` prefix).
pub fn decode_base64(raw: &str) -> Result<Vec<u8>> {
    let payload = match raw.split_once(";base64,") {
        Some((_, b64)) => b64,
        None => raw,
    };

    let mut clean = Vec::with_capacity(payload.len());
    for b in payload.bytes() {
        if !b.is_ascii_whitespace() {
            clean.push(b);
        }
    }
    if clean.is_empty() {
        return Ok(Vec::new());
    }
    if clean.len() % 4 != 0 {
        return Err(HydrogenError::Validation(
            "invalid base64 length (not a multiple of 4)".into(),
        ));
    }

    let mut out = Vec::with_capacity((clean.len() / 4) * 3);
    let (quads, _) = clean.as_chunks::<4>();
    for quad in quads {
        let a = decode_sextet(quad[0])?;
        let b = decode_sextet(quad[1])?;
        let pad_c = quad[2] == b'=';
        let pad_d = quad[3] == b'=';
        if pad_c && !pad_d {
            return Err(HydrogenError::Validation(
                "invalid base64 padding sequence".into(),
            ));
        }
        let c = if pad_c { 0 } else { decode_sextet(quad[2])? };
        let d = if pad_d { 0 } else { decode_sextet(quad[3])? };

        let n = (u32::from(a) << 18) | (u32::from(b) << 12) | (u32::from(c) << 6) | u32::from(d);
        out.push(((n >> 16) & 0xff) as u8);
        if !pad_c {
            out.push(((n >> 8) & 0xff) as u8);
        }
        if !pad_d {
            out.push((n & 0xff) as u8);
        }
    }
    Ok(out)
}

fn decode_sextet(byte: u8) -> Result<u8> {
    match byte {
        b'A'..=b'Z' => Ok(byte - b'A'),
        b'a'..=b'z' => Ok(byte - b'a' + 26),
        b'0'..=b'9' => Ok(byte - b'0' + 52),
        b'+' | b'-' => Ok(62),
        b'/' | b'_' => Ok(63),
        _ => Err(HydrogenError::Validation(format!(
            "invalid base64 character byte 0x{byte:02x}"
        ))),
    }
}

/// Format raw asset bytes into a browser/Carbon `data:<mime>;base64,...` URI.
pub fn to_data_uri(mime_type: &str, bytes: &[u8]) -> String {
    format!("data:{mime_type};base64,{}", encode_base64(bytes))
}

/// Extract `(width_px, height_px)` directly from a valid PNG's 24-byte `IHDR` header.
pub fn read_png_dimensions(bytes: &[u8]) -> Option<(i64, i64)> {
    const PNG_SIG: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if bytes.len() < 24 || &bytes[0..8] != PNG_SIG || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    if width == 0 || height == 0 {
        return None;
    }
    Some((i64::from(width), i64::from(height)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_base64_and_data_uri() -> Result<()> {
        for sample in [
            b"".as_slice(),
            b"G".as_slice(),
            b"Gy".as_slice(),
            b"Gya".as_slice(),
            b"Gyara+ 11+ Maths!".as_slice(),
        ] {
            let encoded = encode_base64(sample);
            let decoded = decode_base64(&encoded)?;
            assert_eq!(decoded, sample);

            let uri = to_data_uri("image/png", sample);
            let from_uri = decode_base64(&uri)?;
            assert_eq!(from_uri, sample);
        }
        Ok(())
    }
}
