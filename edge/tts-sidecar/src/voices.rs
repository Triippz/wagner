//! NPZ voice-pack loader and style-vector selection.
//!
//! The voices file (`voices-v1.0.bin`) is a ZIP archive of `.npy` arrays.
//! Each entry is named `<voice_name>.npy` and has shape (510, 1, 256) f32.
//!
//! To produce the style vector for a synthesis request with N phoneme tokens:
//! - clamp N to max index 509  (i.e. `min(N, 509)`)
//! - return `voices[clamped_N]` → shape [1, 256]

use std::collections::HashMap;
use std::io::Read as _;

use ndarray::{Array2, Array3, ArrayView1};

/// Maximum allowed size for a single zip entry (4 MiB); guards against zip-bombs.
const MAX_ENTRY_BYTES: u64 = 4 * 1024 * 1024;

/// All loaded voice arrays, keyed by voice name.
pub struct Voices {
    pub data: HashMap<String, Array3<f32>>,
}

impl Voices {
    /// Parse a `voices-v1.0.bin` NPZ file from the given path.
    pub fn load(path: &str) -> Result<Self, String> {
        let file_bytes = std::fs::read(path).map_err(|e| format!("read voices '{path}': {e}"))?;
        let cursor = std::io::Cursor::new(file_bytes);
        let mut zip =
            zip::ZipArchive::new(cursor).map_err(|e| format!("zip open '{path}': {e}"))?;
        let mut data = HashMap::new();

        for i in 0..zip.len() {
            let mut entry = zip
                .by_index(i)
                .map_err(|e| format!("zip entry {i}: {e}"))?;
            let name = entry.name().to_owned();
            if !name.ends_with(".npy") {
                continue;
            }
            let voice_name = name.trim_end_matches(".npy").to_owned();

            // Guard against zip-bomb: reject entries that are too large.
            if entry.size() > MAX_ENTRY_BYTES {
                return Err(format!(
                    "zip entry '{voice_name}' too large ({} bytes > {} limit)",
                    entry.size(),
                    MAX_ENTRY_BYTES
                ));
            }

            let mut npy_bytes = Vec::new();
            entry
                .read_to_end(&mut npy_bytes)
                .map_err(|e| format!("read npy '{voice_name}': {e}"))?;

            let array = parse_npy_f32(&npy_bytes, &voice_name)?;
            data.insert(voice_name, array);
        }

        Ok(Self { data })
    }

    /// Return the style vector for `voice_name` at token-count `n_tokens`.
    ///
    /// `n_tokens` is clamped to 509 so the index never exceeds the 510-row
    /// dimension (rows 0–509).
    pub fn style_vector(&self, voice_name: &str, n_tokens: usize) -> Result<Array2<f32>, String> {
        let voice = self
            .data
            .get(voice_name)
            .ok_or_else(|| format!("voice '{voice_name}' not found in pack"))?;

        // voice shape: (510, 1, 256) — clamp index to last valid row (509).
        let max_idx = voice.dim().0.saturating_sub(1); // 509 for normal packs
        let idx = n_tokens.min(max_idx);

        let binding = voice.index_axis(ndarray::Axis(0), idx);
        let row: ArrayView1<f32> = binding.index_axis(ndarray::Axis(0), 0);
        let mut style = Array2::zeros((1, 256));
        for (j, &v) in row.iter().enumerate() {
            style[[0, j]] = v;
        }
        Ok(style)
    }
}

/// Parse a raw `.npy` byte buffer into a `(510, 1, 256)` f32 ndarray.
///
/// NPY wire format:
///   - bytes 0–5:  `\x93NUMPY` magic
///   - byte  6:    major version
///   - byte  7:    minor version
///   - v1.x: header_len as u16 LE at bytes 8–9, data at 10+header_len
///   - v2.x: header_len as u32 LE at bytes 8–11, data at 12+header_len
pub fn parse_npy_f32(bytes: &[u8], name: &str) -> Result<Array3<f32>, String> {
    if bytes.len() < 10 || &bytes[0..6] != b"\x93NUMPY" {
        return Err(format!("invalid npy magic for '{name}'"));
    }
    let major = bytes[6];
    let (header_len, data_offset) = match major {
        1 => {
            // v1.x: u16 header_len at bytes 8-9, data at 10+header_len
            let hl = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
            (hl, 10 + hl)
        }
        2 => {
            // v2.x: u32 header_len at bytes 8-11, data at 12+header_len
            if bytes.len() < 12 {
                return Err(format!("npy v2 header truncated for '{name}'"));
            }
            let hl =
                u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
            (hl, 12 + hl)
        }
        other => {
            return Err(format!(
                "unsupported npy major version {other} for '{name}'"
            ));
        }
    };
    let _ = header_len; // used only to compute data_offset
    if data_offset > bytes.len() {
        return Err(format!("npy header_len overflow for '{name}'"));
    }
    let data_bytes = &bytes[data_offset..];

    // Expected shape (510, 1, 256) → 510 × 256 f32 values.
    const EXPECTED: usize = 510 * 256;
    if data_bytes.len() < EXPECTED * 4 {
        return Err(format!(
            "npy data too short for '{name}': {} bytes < {} expected",
            data_bytes.len(),
            EXPECTED * 4
        ));
    }

    let mut floats = vec![0f32; EXPECTED];
    for (i, chunk) in data_bytes[..EXPECTED * 4].chunks_exact(4).enumerate() {
        floats[i] = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }

    Array3::from_shape_vec((510, 1, 256), floats)
        .map_err(|e| format!("ndarray shape error for '{name}': {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal fake .npy blob with shape (510, 1, 256) filled with
    /// a constant value, then wrap it in a Voices struct directly.
    fn fake_voices(voice_name: &str, fill: f32) -> Voices {
        let n: usize = 510 * 256;
        let floats: Vec<f32> = vec![fill; n];
        let array = Array3::from_shape_vec((510, 1, 256), floats).unwrap();
        let mut data = HashMap::new();
        data.insert(voice_name.to_owned(), array);
        Voices { data }
    }

    /// Build a minimal fake .npy blob with distinct per-row values:
    /// row `i` is filled with `i as f32 + 1.0`.
    fn fake_voices_distinct_rows(voice_name: &str) -> Voices {
        let mut floats = vec![0f32; 510 * 256];
        for row in 0..510usize {
            let val = row as f32 + 1.0;
            for col in 0..256usize {
                floats[row * 256 + col] = val;
            }
        }
        let array = Array3::from_shape_vec((510, 1, 256), floats).unwrap();
        let mut data = HashMap::new();
        data.insert(voice_name.to_owned(), array);
        Voices { data }
    }

    #[test]
    fn style_vector_at_normal_index() {
        let voices = fake_voices("af_heart", 1.0);
        let sv = voices.style_vector("af_heart", 5).unwrap();
        assert_eq!(sv.shape(), &[1, 256]);
        assert!((sv[[0, 0]] - 1.0f32).abs() < f32::EPSILON);
    }

    #[test]
    fn style_vector_clamps_above_509() {
        // n_tokens = 600 should clamp to 509; row 509 has value 510.0
        let voices = fake_voices_distinct_rows("af_heart");
        let sv = voices.style_vector("af_heart", 600).unwrap();
        assert_eq!(sv.shape(), &[1, 256]);
        // Row index 509 → value 510.0 (row 509 + 1.0)
        assert!(
            (sv[[0, 0]] - 510.0f32).abs() < f32::EPSILON,
            "expected row 509 (value 510.0), got {}",
            sv[[0, 0]]
        );
    }

    #[test]
    fn style_vector_at_509_is_ok() {
        let voices = fake_voices("af_heart", 3.0);
        let sv = voices.style_vector("af_heart", 509).unwrap();
        assert_eq!(sv.shape(), &[1, 256]);
    }

    #[test]
    fn unknown_voice_returns_err() {
        let voices = fake_voices("af_heart", 0.0);
        assert!(voices.style_vector("no_such_voice", 10).is_err());
    }

    // -----------------------------------------------------------------------
    // parse_npy_f32 unit tests
    // -----------------------------------------------------------------------

    /// Build a minimal valid v1.0 NPY byte buffer for shape (510, 1, 256).
    fn build_valid_npy_v1(fill: f32) -> Vec<u8> {
        // Header string (Python dict literal padded to 64-byte alignment).
        let header_str =
            "{'descr': '<f4', 'fortran_order': False, 'shape': (510, 1, 256), }";
        // header_len must make (10 + header_len) a multiple of 64.
        let base = 10usize;
        let raw_header_len = header_str.len() + 1; // +1 for '\n'
        let padded_len = (base + raw_header_len).div_ceil(64) * 64 - base;
        let mut header_bytes = header_str.as_bytes().to_vec();
        while header_bytes.len() < padded_len - 1 {
            header_bytes.push(b' ');
        }
        header_bytes.push(b'\n');
        assert_eq!(header_bytes.len(), padded_len);

        let mut buf: Vec<u8> = Vec::new();
        buf.extend_from_slice(b"\x93NUMPY"); // magic
        buf.push(1); // major
        buf.push(0); // minor
        let hl = padded_len as u16;
        buf.extend_from_slice(&hl.to_le_bytes()); // header_len (u16 LE)
        buf.extend_from_slice(&header_bytes);

        // Data: 510 * 1 * 256 f32 values
        let n = 510 * 256;
        for _ in 0..n {
            buf.extend_from_slice(&fill.to_le_bytes());
        }
        buf
    }

    #[test]
    fn parse_npy_invalid_magic_returns_err() {
        let bad = b"NOTNPY\x01\x00\x10\x00".to_vec();
        assert!(parse_npy_f32(&bad, "test").is_err());
    }

    #[test]
    fn parse_npy_header_len_overflow_returns_err() {
        // Valid magic + v1, but header_len puts data_offset beyond the buffer.
        let mut buf = b"\x93NUMPY\x01\x00".to_vec();
        buf.extend_from_slice(&0xFFFFu16.to_le_bytes()); // huge header_len
        buf.extend_from_slice(&[0u8; 8]); // tiny remaining data
        assert!(parse_npy_f32(&buf, "test").is_err());
    }

    #[test]
    fn parse_npy_data_too_short_returns_err() {
        // Valid magic + v1, tiny header, but data too small for 510*256 f32s.
        let mut buf = b"\x93NUMPY\x01\x00".to_vec();
        // header_len = 0 so data starts at offset 10
        buf.extend_from_slice(&0u16.to_le_bytes());
        buf.extend_from_slice(&[0u8; 8]); // 8 bytes of data — way too short
        assert!(parse_npy_f32(&buf, "test").is_err());
    }

    #[test]
    fn parse_npy_valid_v1_correct_value() {
        let fill = std::f32::consts::PI;
        let buf = build_valid_npy_v1(fill);
        let arr = parse_npy_f32(&buf, "test").expect("should parse");
        assert_eq!(arr.dim(), (510, 1, 256));
        // Spot-check a known index.
        assert!(
            (arr[[42, 0, 7]] - fill).abs() < 1e-6,
            "expected {fill} at [42,0,7], got {}",
            arr[[42, 0, 7]]
        );
    }

    #[test]
    fn parse_npy_unsupported_major_returns_err() {
        let mut buf = b"\x93NUMPY\x03\x00".to_vec();
        buf.extend_from_slice(&0u16.to_le_bytes());
        buf.extend_from_slice(&[0u8; 8]);
        let err = parse_npy_f32(&buf, "test").unwrap_err();
        assert!(
            err.contains("unsupported npy major version 3"),
            "unexpected error: {err}"
        );
    }
}
