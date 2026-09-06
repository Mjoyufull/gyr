//! Reversible filesystem-path keys for persistent storage.

use std::path::{Path, PathBuf};

pub(crate) fn encode(path: &Path) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;

        hex_encode(path.as_os_str().as_bytes())
    }

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;

        let mut bytes = Vec::new();
        for unit in path.as_os_str().encode_wide() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        hex_encode(&bytes)
    }

    #[cfg(not(any(unix, windows)))]
    {
        hex_encode(path.to_string_lossy().as_bytes())
    }
}

pub(crate) fn decode(encoded: &str) -> Option<PathBuf> {
    let bytes = hex_decode(encoded)?;

    #[cfg(unix)]
    {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        Some(PathBuf::from(OsString::from_vec(bytes)))
    }

    #[cfg(windows)]
    {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;

        let mut wide = Vec::with_capacity(bytes.len() / 2);
        let mut chunks = bytes.chunks_exact(2);
        for chunk in &mut chunks {
            wide.push(u16::from_le_bytes([chunk[0], chunk[1]]));
        }
        if !chunks.remainder().is_empty() {
            return None;
        }

        Some(PathBuf::from(OsString::from_wide(&wide)))
    }

    #[cfg(not(any(unix, windows)))]
    {
        String::from_utf8(bytes).ok().map(PathBuf::from)
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn hex_decode(encoded: &str) -> Option<Vec<u8>> {
    if !encoded.len().is_multiple_of(2) {
        return None;
    }

    let mut bytes = Vec::with_capacity(encoded.len() / 2);
    let (pairs, remainder) = encoded.as_bytes().as_chunks::<2>();
    debug_assert!(remainder.is_empty());
    for pair in pairs {
        let high = decode_hex_nibble(pair[0])?;
        let low = decode_hex_nibble(pair[1])?;
        bytes.push((high << 4) | low);
    }
    Some(bytes)
}

fn decode_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{decode, encode};
    use std::path::Path;

    #[test]
    fn path_keys_round_trip() {
        let path = Path::new("/usr/share/applications/example.desktop");

        assert_eq!(decode(&encode(path)).as_deref(), Some(path));
    }
}
