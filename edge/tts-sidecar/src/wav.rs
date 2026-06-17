//! WAV encoding: f32 PCM samples → 16-bit WAV bytes.
//!
//! Output format: PCM, mono, 24 kHz, 16-bit signed integers (little-endian).

use std::io::Cursor;

use hound::{SampleFormat, WavSpec, WavWriter};

pub const SAMPLE_RATE: u32 = 24_000;

/// Encode a slice of f32 samples (range −1.0..=1.0) into a WAV byte vector.
///
/// Each sample is clamped to [−1.0, 1.0] then scaled to i16 range.
pub fn encode_wav(samples: &[f32]) -> Result<Vec<u8>, String> {
    let spec = WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut buf = Vec::new();
    {
        let cursor = Cursor::new(&mut buf);
        let mut writer =
            WavWriter::new(cursor, spec).map_err(|e| format!("wav writer: {e}"))?;

        for &sample in samples {
            let clamped = sample.clamp(-1.0, 1.0);
            let pcm = (clamped * i16::MAX as f32) as i16;
            writer
                .write_sample(pcm)
                .map_err(|e| format!("write sample: {e}"))?;
        }
        writer.finalize().map_err(|e| format!("wav finalize: {e}"))?;
    }

    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn riff_header_present_for_nonempty_samples() {
        let samples = vec![0.0f32, 0.5f32, -0.5f32, 1.0f32, -1.0f32];
        let wav = encode_wav(&samples).unwrap();

        // RIFF header: bytes 0..4 == b"RIFF"
        assert!(wav.len() >= 44, "WAV must have at least a 44-byte header");
        assert_eq!(&wav[0..4], b"RIFF", "expected RIFF magic");
        // WAVE marker at offset 8
        assert_eq!(&wav[8..12], b"WAVE", "expected WAVE marker");
    }

    #[test]
    fn empty_sample_slice_produces_valid_wav() {
        let wav = encode_wav(&[]).unwrap();
        // hound writes a valid (but empty-data) WAV even with zero samples
        assert!(wav.len() >= 44);
        assert_eq!(&wav[0..4], b"RIFF");
    }

    #[test]
    fn sample_count_reflected_in_data_chunk_size() {
        let n_samples: usize = 100;
        let samples = vec![0.25f32; n_samples];
        let wav = encode_wav(&samples).unwrap();

        // data chunk size at offset 40 (4 bytes LE) = n_samples * 2 (16-bit)
        let data_size = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]) as usize;
        assert_eq!(data_size, n_samples * 2);
    }

    #[test]
    fn clamping_does_not_panic() {
        // Values outside [−1, 1] should be clamped, not panic.
        let samples = vec![2.0f32, -3.0f32, 100.0f32];
        assert!(encode_wav(&samples).is_ok());
    }

    #[test]
    fn clamping_produces_correct_i16_values() {
        // 2.0 → clamped to 1.0 → i16::MAX; -3.0 → clamped to -1.0 → -i16::MAX
        let samples = vec![2.0f32, -3.0f32];
        let wav = encode_wav(&samples).unwrap();

        // Standard WAV layout: 44-byte header, then 16-bit samples LE.
        // data chunk starts at offset 44.
        assert!(wav.len() >= 44 + 4, "expected at least 48 bytes");

        let s0 = i16::from_le_bytes([wav[44], wav[45]]);
        let s1 = i16::from_le_bytes([wav[46], wav[47]]);

        assert_eq!(s0, i16::MAX, "2.0 should clamp to i16::MAX ({})", i16::MAX);
        assert_eq!(
            s1,
            -i16::MAX,
            "-3.0 should clamp to -i16::MAX ({})",
            -i16::MAX
        );
    }
}
