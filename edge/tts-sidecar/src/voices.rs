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
/// NPY wire format: `\x93NUMPY` magic (6 B) + major (1 B) + minor (1 B) +
/// header_len (2 B LE) + header (header_len B) + raw f32 LE data.
pub fn parse_npy_f32(bytes: &[u8], name: &str) -> Result<Array3<f32>, String> {
    if bytes.len() < 10 || &bytes[0..6] != b"\x93NUMPY" {
        return Err(format!("invalid npy magic for '{name}'"));
    }
    let header_len = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
    let data_offset = 10 + header_len;
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

    #[test]
    fn style_vector_at_normal_index() {
        let voices = fake_voices("af_heart", 1.0);
        let sv = voices.style_vector("af_heart", 5).unwrap();
        assert_eq!(sv.shape(), &[1, 256]);
        assert!((sv[[0, 0]] - 1.0f32).abs() < f32::EPSILON);
    }

    #[test]
    fn style_vector_clamps_above_509() {
        let voices = fake_voices("af_heart", 2.0);
        // n_tokens = 600 should clamp to 509; result is still valid
        let sv = voices.style_vector("af_heart", 600).unwrap();
        assert_eq!(sv.shape(), &[1, 256]);
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
}
